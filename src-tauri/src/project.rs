//! A project is a Starbound mod folder. SME keeps editor-only state in `sme.json` next to
//! `_metadata`; everything else is plain game files that work without SME.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct FileEntry {
    /// Path relative to the project root, with forward slashes.
    pub path: String,
    pub json: Value,
}

#[derive(Serialize, Clone)]
pub struct ProjectInfo {
    pub root: String,
    /// "mod" when the folder is a mod (has `_metadata`), "loose" for a lone dungeon folder.
    pub mode: String,
    pub metadata: Value,
    pub metadata_file: String,
    pub dungeons: Vec<FileEntry>,
    pub treasure: Vec<FileEntry>,
    pub patches: Vec<FileEntry>,
    /// Every `.json` under `dungeons/` (candidate maps), so maps no dungeon lists still show up.
    pub maps: Vec<String>,
    pub tilesets: Vec<String>,
    pub sme: Value,
    pub errors: Vec<String>,
}

pub fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// The mod root for a file: the nearest ancestor with `_metadata`/`.metadata`.
pub fn find_mod_root(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|a| a.join("_metadata").is_file() || a.join(".metadata").is_file())
        .map(Path::to_path_buf)
}

/// Resolves a project-relative path, refusing anything that escapes the root.
pub fn safe_join(root: &Path, relp: &str) -> Result<PathBuf> {
    let relp = relp.trim_start_matches('/');
    if relp.split(['/', '\\']).any(|s| s == "..") {
        bail!("path {relp} leaves the project folder");
    }
    Ok(root.join(relp))
}

pub fn scan(root: &Path) -> Result<ProjectInfo> {
    if !root.is_dir() {
        return Err(anyhow!("{} is not a folder", root.display()));
    }
    let meta_name = if root.join("_metadata").is_file() {
        "_metadata"
    } else if root.join(".metadata").is_file() {
        ".metadata"
    } else {
        ""
    };
    let mut errors = Vec::new();
    let metadata = if meta_name.is_empty() {
        Value::Null
    } else {
        match std::fs::read(root.join(meta_name)).map_err(anyhow::Error::from).and_then(|b| crate::sbjson::parse(&b)) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{meta_name}: {e}"));
                Value::Null
            }
        }
    };
    let mut info = ProjectInfo {
        root: root.to_string_lossy().into_owned(),
        mode: if meta_name.is_empty() { "loose".into() } else { "mod".into() },
        metadata,
        metadata_file: if meta_name.is_empty() { "_metadata".into() } else { meta_name.into() },
        dungeons: vec![],
        treasure: vec![],
        patches: vec![],
        maps: vec![],
        tilesets: vec![],
        sme: std::fs::read(root.join("sme.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null),
        errors,
    };
    for e in walkdir::WalkDir::new(root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
        if !e.file_type().is_file() {
            continue;
        }
        let r = rel(root, e.path());
        if r.starts_with('.') || r.contains("/.") || r.starts_with("node_modules") {
            continue;
        }
        let load = |info: &mut ProjectInfo| -> Option<Value> {
            match std::fs::read(e.path()).map_err(anyhow::Error::from).and_then(|b| crate::sbjson::parse(&b)) {
                Ok(v) => Some(v),
                Err(err) => {
                    info.errors.push(format!("{r}: {err}"));
                    None
                }
            }
        };
        if r.ends_with(".dungeon") {
            if let Some(json) = load(&mut info) {
                info.dungeons.push(FileEntry { path: r, json });
            }
        } else if r.ends_with(".treasurepools") {
            if let Some(json) = load(&mut info) {
                info.treasure.push(FileEntry { path: r, json });
            }
        } else if r.ends_with(".patch") {
            if let Some(json) = load(&mut info) {
                info.patches.push(FileEntry { path: r, json });
            }
        } else if r.starts_with("dungeons/") && r.ends_with(".json") {
            info.maps.push(r);
        } else if r.starts_with("tilesets/") && r.ends_with(".json") {
            info.tilesets.push(r);
        }
    }
    Ok(info)
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct NewProject {
    pub dir: String,
    pub name: String,
    pub friendly_name: String,
    pub author: String,
    pub description: String,
}

pub fn create(p: &NewProject) -> Result<PathBuf> {
    let root = PathBuf::from(&p.dir);
    if root.join("_metadata").exists() {
        bail!("{} already contains a mod", root.display());
    }
    std::fs::create_dir_all(root.join("dungeons/microdungeons"))?;
    let meta = serde_json::json!({
        "name": p.name,
        "friendlyName": p.friendly_name,
        "author": p.author,
        "version": "1.0",
        "description": p.description,
        "priority": 0,
        "tags": "Dungeons",
        "includes": [],
        "requires": []
    });
    std::fs::write(root.join("_metadata"), crate::sbjson::pretty(&meta))?;
    Ok(root)
}

/// Writes text atomically (temp file + rename), creating parent folders.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = path.with_extension(format!(
        "{}.smetmp",
        path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
