//! Trellis root discovery: the nearest directory at or above a start point
//! holding all four markers — `trellis.toml` (file), `problem/`,
//! `solution/`, `org/` (directories). Port of `hooks/gate.mjs`.
//!
//! Discovery checks existence only, never content: a corrupt `trellis.toml`
//! still marks a root, so the write gate keeps working while the first real
//! command explains what is wrong with the file.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Root {
    pub path: PathBuf,
}

impl Root {
    pub fn is_root(dir: &Path) -> bool {
        dir.join(crate::domain::FILE).is_file()
            && dir.join("problem").is_dir()
            && dir.join("solution").is_dir()
            && dir.join("org").is_dir()
    }

    /// Walk up from `start` to the filesystem root looking for the markers.
    pub fn discover_from(start: &Path) -> Option<Root> {
        let mut dir = if start.is_absolute() {
            start.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(start)
        };
        loop {
            if Root::is_root(&dir) {
                return Some(Root { path: dir });
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Resolve the operating root: an explicit `--root`/`-C` wins (and must
    /// itself be or contain a root); otherwise walk up from the cwd.
    pub fn discover(explicit: Option<&Path>) -> anyhow::Result<Root> {
        let start = match explicit {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir()?,
        };
        Root::discover_from(&start).ok_or_else(|| {
            anyhow::anyhow!(
                "no Trellis root at or above {} (markers: trellis.toml, problem/, solution/, org/)",
                start.display()
            )
        })
    }

    /// The plugin's own `template/` is shaped like a root; never operate on it
    /// as one (a sibling `.claude-plugin/` identifies the plugin checkout).
    pub fn is_plugin_template(&self) -> bool {
        self.path
            .file_name()
            .map(|n| n == "template")
            .unwrap_or(false)
            && self
                .path
                .parent()
                .map(|p| p.join(".claude-plugin").exists())
                .unwrap_or(false)
    }

    /// Root-relative path with forward slashes.
    pub fn rel(&self, abs: &Path) -> Option<String> {
        let rel = abs.strip_prefix(&self.path).ok()?;
        let s = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        Some(s)
    }
}
