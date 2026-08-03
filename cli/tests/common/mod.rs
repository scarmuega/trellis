#![allow(dead_code)] // shared across test binaries; not every bin uses every helper

//! Fixture harness: ports `evals/run.sh`'s layer composition (skeleton +
//! overlay + fixture delta) and `evals/prepare.mjs`'s date-token
//! substitution, anchored to a FIXED date via `TRELLIS_TODAY` so dwell
//! arithmetic and rendered views are hermetic.

use std::path::{Path, PathBuf};
use std::process::Command;

use trellis::dates::Date;

/// Every test runs "today = 2026-08-03".
pub const ANCHOR: &str = "2026-08-03";
/// The composing commit is backdated well before the anchor.
pub const BACKDATE: &str = "2026-06-24";

pub fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)
}

pub struct Fixture {
    pub dir: tempfile::TempDir,
}

fn anchor_date() -> Date {
    Date::parse(ANCHOR).unwrap()
}

fn substitute_tokens(text: &str) -> String {
    let anchor = anchor_date().days_from_epoch();
    let iso = |days: i64| Date::from_epoch_days(days).to_string();
    let mut out = text.replace("%%TODAY%%", &iso(anchor));
    let ago = regex_lite(r"%%DAYS_AGO_(\d+)%%");
    while let Some((full, n)) = ago(&out) {
        out = out.replacen(&full, &iso(anchor - n), 1);
    }
    let ahead = regex_lite(r"%%DAYS_AHEAD_(\d+)%%");
    while let Some((full, n)) = ahead(&out) {
        out = out.replacen(&full, &iso(anchor + n), 1);
    }
    out
}

/// Tiny matcher: returns (full match, captured number) for the first hit.
fn regex_lite(pattern: &str) -> impl Fn(&str) -> Option<(String, i64)> + '_ {
    let re = regex::Regex::new(pattern).unwrap();
    move |s: &str| {
        re.captures(s)
            .map(|c| (c[0].to_string(), c[1].parse().unwrap()))
    }
}

fn copy_layer(src: &Path, dest: &Path) {
    for entry in walkdir::WalkDir::new(src).sort_by_file_name() {
        let entry = entry.unwrap();
        let rel = entry.path().strip_prefix(src).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
        } else {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            let content = std::fs::read_to_string(entry.path())
                .map(|t| substitute_tokens(&t))
                .map(String::into_bytes)
                .unwrap_or_else(|_| std::fs::read(entry.path()).unwrap());
            std::fs::write(&target, content).unwrap();
        }
    }
}

impl Fixture {
    pub fn compose(layers: &[&str]) -> Fixture {
        let dir = tempfile::TempDir::new().unwrap();
        for layer in layers {
            copy_layer(&repo_path(layer), dir.path());
        }
        let f = Fixture { dir };
        f.git(&["init", "-q"]);
        f.git(&["config", "user.email", "fixture@example.invalid"]);
        f.git(&["config", "user.name", "fixture"]);
        f.commit_at(BACKDATE, "compose fixture");
        f
    }

    pub fn healthy() -> Fixture {
        Fixture::compose(&["evals/skeleton", "evals/fixtures/healthy"])
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    pub fn commit_at(&self, date: &str, msg: &str) {
        self.git(&["add", "-A"]);
        let stamp = format!("{date}T09:00:00Z");
        let out = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["commit", "-q", "--allow-empty", "-m", msg])
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    pub fn write(&self, rel: &str, content: &str) {
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, substitute_tokens(content)).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root().join(rel)).unwrap()
    }

    /// The trellis binary, cwd at the fixture root, hermetic today.
    pub fn bin(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("trellis").unwrap();
        cmd.current_dir(self.root())
            .env("TRELLIS_TODAY", ANCHOR)
            .env_remove("CLAUDE_PLUGIN_ROOT");
        cmd
    }

    pub fn lint_json(&self, extra: &[&str]) -> serde_json::Value {
        let out = self
            .bin()
            .args(["lint", "--format", "json"])
            .args(extra)
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "lint --format json did not emit JSON: {e}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
    }
}

pub fn finding_items(report: &serde_json::Value) -> Vec<u8> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["item"].as_u64().unwrap() as u8)
        .collect()
}
