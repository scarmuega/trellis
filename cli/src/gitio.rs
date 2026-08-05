//! Git access by shelling out — the whole reference binding presumes `git`
//! on PATH (gate.mjs spawned it; every workflow checks out). Five plumbing
//! operations, memoized per run. Dwell and flow derive from the status
//! timeline: the frontmatter `status:` value at each commit that touched
//! the file.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::dates::Date;
use crate::frontmatter;

/// (date, status) transitions, oldest first.
pub type Timeline = Vec<(Date, Option<String>)>;

pub struct Git {
    root: PathBuf,
    is_repo: RefCell<Option<bool>>,
    ignored: RefCell<Option<Vec<String>>>,
    timelines: RefCell<HashMap<String, Timeline>>,
}

impl Git {
    pub fn new(root: PathBuf) -> Git {
        Git {
            root,
            is_repo: RefCell::new(None),
            ignored: RefCell::new(None),
            timelines: RefCell::new(HashMap::new()),
        }
    }

    fn run(&self, args: &[&str]) -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            None
        }
    }

    pub fn is_repo(&self) -> bool {
        if let Some(v) = *self.is_repo.borrow() {
            return v;
        }
        let v = self.run(&["rev-parse", "--git-dir"]).is_some();
        *self.is_repo.borrow_mut() = Some(v);
        v
    }

    /// Paths git excludes, root-relative, sorted; a wholly-ignored directory
    /// collapses to one entry with a trailing `/` rather than its contents —
    /// which is what keeps a `node_modules/` a single line instead of forty
    /// thousand. Empty outside a repo (and on any git failure), so a
    /// non-repo root sees exactly the pre-scope behavior.
    ///
    /// Deliberately *not* `--no-empty-directory`: paired with `--ignored` it
    /// drops a wholly-ignored directory whose parent is itself untracked —
    /// exactly the `node_modules/` under a not-yet-committed deployment unit
    /// this is for. An empty directory in the prune set costs nothing.
    pub fn ignored_paths(&self) -> Vec<String> {
        if let Some(v) = self.ignored.borrow().as_ref() {
            return v.clone();
        }
        let mut paths: Vec<String> = self
            .run(&[
                "ls-files",
                "-z",
                "--others",
                "--ignored",
                "--directory",
                "--exclude-standard",
            ])
            .map(|out| {
                out.split('\0')
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        *self.ignored.borrow_mut() = Some(paths.clone());
        paths
    }

    /// File content at HEAD, or None when not in HEAD / not a repo.
    pub fn head_text(&self, rel: &str) -> Option<String> {
        self.run(&["show", &format!("HEAD:{rel}")])
    }

    /// Uncommitted changes (staged or not) touching the path.
    pub fn is_dirty(&self, rel: &str) -> bool {
        self.run(&["status", "--porcelain", "--", rel])
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }

    /// Commits touching the path, oldest first: (sha, committer date).
    pub fn commits(&self, rel: &str) -> Vec<(String, Date)> {
        let Some(out) = self.run(&["log", "--reverse", "--format=%H %cs", "--", rel]) else {
            return vec![];
        };
        out.lines()
            .filter_map(|l| {
                let (sha, date) = l.split_once(' ')?;
                Some((sha.to_string(), Date::parse(date)?))
            })
            .collect()
    }

    pub fn text_at(&self, sha: &str, rel: &str) -> Option<String> {
        self.run(&["show", &format!("{sha}:{rel}")])
    }

    fn status_at(&self, sha: &str, rel: &str) -> Option<String> {
        let text = self.text_at(sha, rel)?;
        frontmatter::extract(&text).and_then(|fm| fm.get_str("status"))
    }

    /// (date, status) at every commit touching the file, oldest first,
    /// consecutive equal statuses compressed to the first occurrence.
    pub fn status_timeline(&self, rel: &str) -> Timeline {
        if let Some(t) = self.timelines.borrow().get(rel) {
            return t.clone();
        }
        let mut timeline: Timeline = Vec::new();
        for (sha, date) in self.commits(rel) {
            let status = self.status_at(&sha, rel);
            if timeline.last().map(|(_, s)| s) != Some(&status) {
                timeline.push((date, status));
            }
        }
        self.timelines
            .borrow_mut()
            .insert(rel.to_string(), timeline.clone());
        timeline
    }

    /// Date of the commit that set the file's *current* status — the start
    /// of the trailing run equal to `current`. `None` when the flip is not
    /// committed yet (dwell zero) or there is no history.
    pub fn status_set_date(&self, rel: &str, current: &str) -> Option<Date> {
        let timeline = self.status_timeline(rel);
        let last = timeline.last()?;
        if last.1.as_deref() != Some(current) {
            return None;
        }
        Some(last.0)
    }

    pub fn first_commit_date(&self, rel: &str) -> Option<Date> {
        self.commits(rel).first().map(|(_, d)| *d)
    }

    pub fn last_commit_date(&self, rel: &str) -> Option<Date> {
        self.commits(rel).last().map(|(_, d)| *d)
    }
}
