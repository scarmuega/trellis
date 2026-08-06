//! Running harness sessions.
//!
//! One session per due task, launched from an argv template (`tmpl`), with
//! stdout and stderr captured to a file under `.trellis/runtime/logs/` — the
//! session-trail half of the ledger, where git holds the durable half.
//!
//! Two guards, at different scales. Within the process, a task already in
//! flight is never launched twice. Across restarts, nothing here is the
//! guard: dispatch idempotence comes from the taker flipping its plan
//! `ready → active` on disk, so a session that dies before claiming is simply
//! retried on the next tick (decision 0029). Children are put in their own
//! process group, so a Ctrl-C to the daemon does not kill a session
//! mid-artifact.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde::Serialize;

use super::config::RuntimeConfig;
use crate::dates::{self, Date};

pub struct InFlight {
    pub label: String,
    pub log: String,
    pub started: String,
    /// This session's back-channel handle, when the daemon is serving one
    /// (`mcp`). Carried here so reaping can close the channel with the
    /// process that owned it.
    token: Option<String>,
    child: Child,
}

/// One in-flight session, as `/api/status` reports it.
#[derive(Debug, Serialize)]
pub struct InFlightView {
    pub key: String,
    pub label: String,
    pub log: String,
    pub started: String,
    pub token: Option<String>,
}

#[derive(Debug)]
pub struct Finished {
    pub key: String,
    pub label: String,
    pub exit: Option<i32>,
    pub token: Option<String>,
}

pub struct Spawner {
    root: PathBuf,
    log_dir: PathBuf,
    max_concurrent: usize,
    in_flight: HashMap<String, InFlight>,
    /// Keys already started, when the caller wants each task to run at most
    /// once for the lifetime of this spawner. `--once` sets it: draining a
    /// due set must terminate even against a harness that never claims its
    /// plan, where the continuous loop relies on the tick interval instead
    /// (retry-next-tick, decision 0029).
    ledger: Option<HashSet<String>>,
}

impl Spawner {
    pub fn new(root: &Path, max_concurrent: usize) -> Spawner {
        Spawner {
            root: root.to_path_buf(),
            log_dir: RuntimeConfig::runtime_dir(root).join("logs"),
            max_concurrent,
            in_flight: HashMap::new(),
            ledger: None,
        }
    }

    /// Run each task at most once for this spawner's lifetime.
    pub fn once_per_task(mut self) -> Spawner {
        self.ledger = Some(HashSet::new());
        self
    }

    pub fn already_ran(&self, key: &str) -> bool {
        self.ledger.as_ref().is_some_and(|l| l.contains(key))
    }

    pub fn is_busy(&self, key: &str) -> bool {
        self.in_flight.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.in_flight.len()
    }

    pub fn is_empty(&self) -> bool {
        self.in_flight.is_empty()
    }

    /// Free session slots. A task that finds none stays due and is retried on
    /// the next tick, so a busy daemon defers work rather than dropping it.
    pub fn capacity(&self) -> usize {
        self.max_concurrent.saturating_sub(self.in_flight.len())
    }

    pub fn in_flight(&self) -> Vec<InFlightView> {
        let mut views: Vec<InFlightView> = self
            .in_flight
            .iter()
            .map(|(key, f)| InFlightView {
                key: key.clone(),
                label: f.label.clone(),
                log: f.log.clone(),
                started: f.started.clone(),
                token: f.token.clone(),
            })
            .collect();
        views.sort_by(|a, b| a.key.cmp(&b.key));
        views
    }

    /// Launch one session. Returns the log path, relative to the runtime
    /// directory.
    pub fn spawn(
        &mut self,
        key: &str,
        label: &str,
        argv: &[String],
        token: Option<String>,
    ) -> anyhow::Result<String> {
        let (program, rest) = argv
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("{label}: empty command"))?;

        let stamp = timestamp(dates::today());
        let name = format!("{stamp}-{}.log", slug(key));
        std::fs::create_dir_all(&self.log_dir)?;
        let path = self.log_dir.join(&name);
        let mut file = std::fs::File::create(&path)?;
        writeln!(file, "# {label}")?;
        writeln!(file, "# {}", shell_quote(argv))?;
        file.flush()?;
        let err = file.try_clone()?;

        // A relative program path is resolved against the root the child will
        // run in, not the daemon's cwd — std leaves that platform-specific.
        let program: PathBuf = if program.contains('/') && Path::new(program).is_relative() {
            self.root.join(program)
        } else {
            PathBuf::from(program)
        };

        let mut cmd = Command::new(&program);
        cmd.args(rest)
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(err));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("{label}: cannot run {}: {e}", program.display()))?;
        if let Some(ledger) = &mut self.ledger {
            ledger.insert(key.to_string());
        }
        self.in_flight.insert(
            key.to_string(),
            InFlight {
                label: label.to_string(),
                log: name.clone(),
                started: stamp,
                token,
                child,
            },
        );
        Ok(name)
    }

    /// Collect whatever has exited. Never blocks.
    pub fn reap(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        self.in_flight.retain(|key, f| match f.child.try_wait() {
            Ok(Some(status)) => {
                done.push(Finished {
                    key: key.clone(),
                    label: f.label.clone(),
                    exit: status.code(),
                    token: f.token.clone(),
                });
                false
            }
            Ok(None) => true,
            Err(_) => {
                // Unwaitable child: stop tracking it rather than wedge the key.
                done.push(Finished {
                    key: key.clone(),
                    label: f.label.clone(),
                    exit: None,
                    token: f.token.clone(),
                });
                false
            }
        });
        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// Block until every session has exited — what `--once` owes its caller.
    pub fn wait_all(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        for (key, mut f) in self.in_flight.drain() {
            let exit = f.child.wait().ok().and_then(|s| s.code());
            done.push(Finished {
                key,
                label: f.label.clone(),
                exit,
                token: f.token.clone(),
            });
        }
        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// Ask every session to stop, and report how many were signalled.
    ///
    /// The children stay tracked: cancelling is not a second way for a session
    /// to end, it is a reason for the ordinary one. `reap`/`wait_all` collect
    /// them as normal and record the exit the signal produced, which
    /// `report_exit` already renders as "killed by signal".
    pub fn cancel_all(&self) -> usize {
        self.in_flight
            .values()
            .filter(|f| signal_group(f.child.id()))
            .count()
    }
}

/// `SIGTERM` to the session's whole process group — the tools it started, not
/// just the harness that started them. Children have been group leaders since
/// they were first spawned, so the negative pid is the whole feature.
///
/// Shelling out matches `alive()`'s precedent next door, and buys the same
/// thing: no dependency for one signal.
#[cfg(unix)]
fn signal_group(pid: u32) -> bool {
    Command::new("kill")
        .args(["-TERM", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn signal_group(_pid: u32) -> bool {
    false
}

/// `YYYY-MM-DDThhmmss` — the date honours `TRELLIS_TODAY`, the time is the
/// wall clock, so log names sort chronologically and stay legible.
pub(super) fn timestamp(today: Date) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let sod = secs.rem_euclid(86_400);
    format!(
        "{today}T{:02}{:02}{:02}",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Filesystem-safe form of a task key (`plan:plans/x.md` → `plan-plans-x.md`).
pub(super) fn slug(key: &str) -> String {
    let s: String = key
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' => c,
            _ => '-',
        })
        .collect();
    s.trim_matches('-').to_string()
}

/// A copy-pasteable rendering of the argv for the log header. Never parsed —
/// the daemon runs the array, not this string.
pub fn shell_quote(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.is_empty()
                || a.contains(|c: char| c.is_whitespace() || "\"'$`\\|&;<>()*?[]#~".contains(c))
            {
                format!("'{}'", a.replace('\'', r"'\''"))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Spawner) {
        let dir = tempfile::TempDir::new().unwrap();
        let spawner = Spawner::new(dir.path(), 2);
        (dir, spawner)
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_session_runs_and_its_output_lands_in_the_log() {
        let (dir, mut spawner) = fixture();
        let log = spawner
            .spawn(
                "ritual:focus",
                "focus",
                &argv(&["sh", "-c", "echo hi; echo bad >&2"]),
                None,
            )
            .unwrap();
        let done = spawner.wait_all();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].exit, Some(0));
        let text = std::fs::read_to_string(
            RuntimeConfig::runtime_dir(dir.path())
                .join("logs")
                .join(&log),
        )
        .unwrap();
        assert!(text.contains("# focus"), "{text}");
        assert!(text.contains("hi"), "{text}");
        assert!(text.contains("bad"), "stderr is captured too: {text}");
    }

    #[test]
    fn a_task_in_flight_is_reported_busy_until_it_is_reaped() {
        let (_dir, mut spawner) = fixture();
        spawner
            .spawn(
                "plan:plans/x.md",
                "plans/x.md",
                &argv(&["sh", "-c", "exit 3"]),
                None,
            )
            .unwrap();
        assert!(spawner.is_busy("plan:plans/x.md"));
        assert_eq!(spawner.capacity(), 1);
        let done = spawner.wait_all();
        assert_eq!(done[0].exit, Some(3));
        assert!(!spawner.is_busy("plan:plans/x.md"));
        assert_eq!(spawner.capacity(), 2);
    }

    #[test]
    fn a_missing_harness_binary_is_an_error_not_a_silent_skip() {
        let (_dir, mut spawner) = fixture();
        let err = spawner
            .spawn(
                "ritual:focus",
                "focus",
                &argv(&["definitely-not-a-command-42"]),
                None,
            )
            .unwrap_err();
        assert!(err.to_string().contains("cannot run"), "{err}");
        assert!(!spawner.is_busy("ritual:focus"));
    }

    #[test]
    fn a_relative_harness_path_resolves_against_the_root() {
        let (dir, mut spawner) = fixture();
        let bin = dir.path().join("tools/echoer");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "#!/bin/sh\necho ran\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        spawner
            .spawn("ritual:x", "x", &argv(&["tools/echoer"]), None)
            .unwrap();
        assert_eq!(spawner.wait_all()[0].exit, Some(0));
    }

    #[test]
    fn a_cancelled_session_is_reaped_as_killed_by_signal() {
        let (_dir, mut spawner) = fixture();
        spawner
            .spawn(
                "ritual:slow",
                "slow",
                &argv(&["sh", "-c", "sleep 60"]),
                None,
            )
            .unwrap();
        assert_eq!(spawner.cancel_all(), 1);
        // The child stays tracked through the ordinary reap, which is what
        // keeps cancellation from being a second way for a session to end.
        let done = spawner.wait_all();
        assert_eq!(done.len(), 1);
        assert_eq!(
            done[0].exit, None,
            "killed by signal, which report_exit renders as such"
        );
    }

    #[test]
    fn log_names_are_filesystem_safe() {
        assert_eq!(slug("plan:plans/ship-it.md"), "plan-plans-ship-it.md");
        assert_eq!(slug("ritual:conventions lint"), "ritual-conventions-lint");
    }

    #[test]
    fn the_log_header_quotes_what_a_shell_would_split() {
        assert_eq!(
            shell_quote(&argv(&["claude", "-p", "/trellis:act coder advance"])),
            "claude -p '/trellis:act coder advance'"
        );
    }
}
