//! Scaffold a domain instance from the embedded `template/` — the binary is
//! self-consistent at its build SHA (works where `$CLAUDE_PLUGIN_ROOT` does
//! not exist), with `--plugin-root` as the live-checkout override.

use std::path::Path;

use anyhow::{bail, Context};
use include_dir::{include_dir, Dir};

use crate::dates::Date;

static TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/../template");

fn substitute(text: &str, owner: &str, today: Date) -> String {
    text.replace("<owner>", owner)
        .replace("{date}", &today.to_string())
}

fn copy_embedded(
    dir: &Dir,
    target: &Path,
    owner: &str,
    today: Date,
    written: &mut Vec<String>,
) -> anyhow::Result<()> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(d) => {
                let dest = target.join(d.path());
                std::fs::create_dir_all(&dest)?;
                copy_embedded(d, target, owner, today, written)?;
            }
            include_dir::DirEntry::File(f) => {
                // The template's README explains the template; an instance
                // does not carry it (scaffold procedure in the skill).
                if f.path() == Path::new("README.md") {
                    continue;
                }
                let dest = target.join(f.path());
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                match f.contents_utf8() {
                    Some(text) => std::fs::write(&dest, substitute(text, owner, today))?,
                    None => std::fs::write(&dest, f.contents())?,
                }
                written.push(f.path().display().to_string());
            }
        }
    }
    Ok(())
}

fn copy_fs(
    src: &Path,
    target: &Path,
    owner: &str,
    today: Date,
    written: &mut Vec<String>,
) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(src).sort_by_file_name() {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        if rel == Path::new("README.md") {
            continue;
        }
        let dest = target.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            match std::fs::read_to_string(entry.path()) {
                Ok(text) => std::fs::write(&dest, substitute(&text, owner, today))?,
                Err(_) => {
                    std::fs::copy(entry.path(), &dest)?;
                }
            }
            written.push(rel.display().to_string());
        }
    }
    Ok(())
}

/// `owner` may be `founder` or `org/founder`; normalized to the ref form.
pub fn scaffold(
    target: &Path,
    owner: &str,
    today: Date,
    plugin_root: Option<&Path>,
) -> anyhow::Result<Vec<String>> {
    let owner = if owner.starts_with("org/") {
        owner.to_string()
    } else {
        format!("org/{owner}")
    };
    if target.exists() {
        let non_empty = std::fs::read_dir(target)
            .with_context(|| format!("cannot read {}", target.display()))?
            .next()
            .is_some();
        if non_empty {
            bail!(
                "{} exists and is not empty — scaffold refuses to overwrite",
                target.display()
            );
        }
    } else {
        std::fs::create_dir_all(target)?;
    }

    let mut written = Vec::new();
    match plugin_root {
        Some(root) => {
            let src = root.join("template");
            if !src.is_dir() {
                bail!("{} has no template/ directory", root.display());
            }
            copy_fs(&src, target, &owner, today, &mut written)?;
        }
        None => copy_embedded(&TEMPLATE, target, &owner, today, &mut written)?,
    }
    written.sort();
    Ok(written)
}
