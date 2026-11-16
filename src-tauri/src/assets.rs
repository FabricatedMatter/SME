//! Layered read-only view over Starbound assets: the vanilla `packed.pak` first, then any
//! directory sources (the open project) on top. Later sources win, like the game's mod order.

use crate::pak::Pak;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub enum Source {
    Pak(Pak),
    Dir(PathBuf),
}

pub struct Assets {
    /// Starbound install folder (the one containing `assets/` and `tiled/`).
    pub install: PathBuf,
    pub sources: Vec<Source>,
}

impl Assets {
    pub fn open(install: &Path) -> Result<Assets> {
        let pak = Pak::open(&install.join("assets/packed.pak"))?;
        Ok(Assets { install: install.to_path_buf(), sources: vec![Source::Pak(pak)] })
    }

    pub fn vanilla(&self) -> Option<&Pak> {
        self.sources.iter().find_map(|s| if let Source::Pak(p) = s { Some(p) } else { None })
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        for s in self.sources.iter().rev() {
            match s {
                Source::Pak(p) if p.contains(path) => return p.read(path),
                Source::Dir(d) => {
                    let f = d.join(path.trim_start_matches('/'));
                    if f.is_file() {
                        return Ok(std::fs::read(f)?);
                    }
                }
                _ => {}
            }
        }
        Err(anyhow!("asset {path} not found"))
    }

    pub fn exists(&self, path: &str) -> bool {
        self.sources.iter().any(|s| match s {
            Source::Pak(p) => p.contains(path),
            Source::Dir(d) => d.join(path.trim_start_matches('/')).is_file(),
        })
    }

    /// True if the asset only comes from the vanilla pak (no project file overrides it).
    pub fn in_vanilla(&self, path: &str) -> bool {
        self.vanilla().is_some_and(|p| p.contains(path))
    }

    pub fn json(&self, path: &str) -> Result<Value> {
        crate::sbjson::parse(&self.read(path)?).map_err(|e| anyhow!("{path}: {e}"))
    }

    /// Every asset path (across all sources) for which `pred` holds, sorted.
    pub fn list(&self, pred: impl Fn(&str) -> bool) -> Vec<String> {
        let mut out = BTreeSet::new();
        for s in &self.sources {
            match s {
                Source::Pak(p) => out.extend(p.paths().filter(|x| pred(x)).cloned()),
                Source::Dir(d) => {
                    for e in walkdir::WalkDir::new(d).into_iter().filter_map(|e| e.ok()) {
                        if !e.file_type().is_file() {
                            continue;
                        }
                        if let Ok(rel) = e.path().strip_prefix(d) {
                            let p = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                            if pred(&p) {
                                out.insert(p);
                            }
                        }
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

/// Resolves `rel` against the directory of asset `base` (both asset paths).
pub fn relative_to(base: &str, rel: &str) -> String {
    if rel.starts_with('/') {
        return rel.to_string();
    }
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    parts.pop();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Finds the Starbound install: explicit setting first, then the usual Steam libraries.
pub fn find_install(configured: Option<&str>) -> Option<PathBuf> {
    let ok = |p: &Path| p.join("assets/packed.pak").is_file();
    if let Some(c) = configured.filter(|c| !c.is_empty()) {
        let p = PathBuf::from(c);
        if ok(&p) {
            return Some(p);
        }
    }
    let home = dirs::home_dir()?;
    let mut libs = vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        PathBuf::from("C:/Program Files (x86)/Steam"),
        home.join("Library/Application Support/Steam"),
    ];
    // extra libraries listed in libraryfolders.vdf
    for base in libs.clone() {
        if let Ok(vdf) = std::fs::read_to_string(base.join("steamapps/libraryfolders.vdf")) {
            for line in vdf.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("\"path\"") {
                    let v = rest.trim().trim_matches('"').replace("\\\\", "/");
                    libs.push(PathBuf::from(v));
                }
            }
        }
    }
    libs.into_iter().map(|l| l.join("steamapps/common/Starbound")).find(|p| ok(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rel() {
        assert_eq!(relative_to("/tiles/materials/dirt.material", "dirt.png"), "/tiles/materials/dirt.png");
        assert_eq!(relative_to("/tiles/materials/dirt.material", "/tiles/x.png"), "/tiles/x.png");
        assert_eq!(relative_to("/a/b/c.json", "../d.png"), "/a/d.png");
    }
}
