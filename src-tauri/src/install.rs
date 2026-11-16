//! Playtesting: put the project into `Starbound/mods/` (symlink, copy, or packed `.pak`).
//! SME only ever removes what it installed itself.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

const MARKER: &str = ".sme-installed";

fn mod_name(root: &Path) -> String {
    let n: String = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sme_mod".into())
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("sme_{n}")
}

fn targets(root: &Path, mods: &Path) -> (PathBuf, PathBuf) {
    let n = mod_name(root);
    (mods.join(&n), mods.join(format!("{n}.pak")))
}

/// Describes the current install, if any.
pub fn state(root: &Path, mods: &Path) -> Option<String> {
    let (dir, pak) = targets(root, mods);
    if let Ok(t) = std::fs::read_link(&dir) {
        if t == root {
            return Some(format!("symlink: {}", dir.display()));
        }
    }
    if dir.join(MARKER).is_file() {
        return Some(format!("copy: {}", dir.display()));
    }
    if pak.is_file() {
        return Some(format!("pak: {}", pak.display()));
    }
    None
}

pub fn uninstall(root: &Path, mods: &Path) -> Result<String> {
    let (dir, pak) = targets(root, mods);
    let mut done = Vec::new();
    if let Ok(t) = std::fs::read_link(&dir) {
        if t == root {
            std::fs::remove_file(&dir)?;
            done.push(dir.display().to_string());
        }
    } else if dir.join(MARKER).is_file() {
        std::fs::remove_dir_all(&dir)?;
        done.push(dir.display().to_string());
    }
    if pak.is_file() {
        std::fs::remove_file(&pak)?;
        done.push(pak.display().to_string());
    }
    Ok(if done.is_empty() { "nothing was installed".into() } else { format!("removed {}", done.join(", ")) })
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(from) {
        let e = e?;
        let rel = e.path().strip_prefix(from)?;
        let r = rel.to_string_lossy();
        if r == "sme.json" || r.starts_with(".git") {
            continue;
        }
        let dst = to.join(rel);
        if e.file_type().is_dir() {
            std::fs::create_dir_all(&dst)?;
        } else if e.file_type().is_file() {
            std::fs::copy(e.path(), &dst)?;
        }
    }
    Ok(())
}

/// method: "symlink" | "copy" | "pak"
pub fn install(root: &Path, mods: &Path, method: &str) -> Result<String> {
    if !mods.is_dir() {
        std::fs::create_dir_all(mods)?;
    }
    uninstall(root, mods)?;
    let (dir, pak) = targets(root, mods);
    if dir.exists() || pak.exists() {
        bail!("{} exists and was not created by SME; remove it first", dir.display());
    }
    match method {
        "symlink" => {
            #[cfg(unix)]
            std::os::unix::fs::symlink(root, &dir)?;
            #[cfg(not(unix))]
            bail!("symlinks are not supported on this platform; use copy");
            Ok(format!("linked {} → {}", dir.display(), root.display()))
        }
        "copy" => {
            copy_dir(root, &dir)?;
            std::fs::write(dir.join(MARKER), b"installed by SME; safe to delete\n")?;
            Ok(format!("copied to {}", dir.display()))
        }
        "pak" => {
            let n = crate::pak::pack_dir(root, &pak, &["sme.json", ".git"])?;
            Ok(format!("packed {n} files into {}", pak.display()))
        }
        m => bail!("unknown install method {m}"),
    }
}
