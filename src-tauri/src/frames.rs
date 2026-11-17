//! Starbound `.frames` files: which rectangle of a sprite sheet `image.png:frame` refers to.
//!
//! The frames file for `/dir/image.png` is `/dir/image.frames`, else the nearest
//! `default.frames` in `/dir` or a parent folder. It has a `frameGrid` (`size`, `dimensions`,
//! optional `begin` and `names`) and/or a `frameList` (`name: [x0, y0, x1, y1]`), plus
//! `aliases`. Rectangles are in image pixels from the top-left corner.

use crate::assets::Assets;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct FrameSpec {
    frames: HashMap<String, [u32; 4]>, // x, y, w, h
    first: Option<[u32; 4]>,
    aliases: HashMap<String, String>,
}

impl FrameSpec {
    pub fn parse(v: &Value) -> FrameSpec {
        let mut s = FrameSpec::default();
        let u = |x: Option<&Value>| x.and_then(Value::as_u64).unwrap_or(0) as u32;
        if let Some(g) = v.get("frameGrid") {
            let size = g.get("size");
            let (w, h) = (u(size.and_then(|s| s.get(0))), u(size.and_then(|s| s.get(1))));
            let dims = g.get("dimensions");
            let (cols, rows) = (u(dims.and_then(|d| d.get(0))), u(dims.and_then(|d| d.get(1))));
            let begin = g.get("begin");
            let (bx, by) = (u(begin.and_then(|b| b.get(0))), u(begin.and_then(|b| b.get(1))));
            let names = g.get("names").and_then(Value::as_array);
            for y in 0..rows {
                for x in 0..cols {
                    let rect = [bx + x * w, by + y * h, w, h];
                    if s.first.is_none() {
                        s.first = Some(rect);
                    }
                    let name = match names {
                        Some(rows) => rows.get(y as usize).and_then(|r| r.get(x as usize)).and_then(Value::as_str).map(String::from),
                        None => Some((y * cols + x).to_string()),
                    };
                    if let Some(n) = name {
                        s.frames.insert(n, rect);
                    }
                }
            }
        }
        if let Some(list) = v.get("frameList").and_then(Value::as_object) {
            for (k, r) in list {
                let c = |i: usize| r.get(i).and_then(Value::as_u64).unwrap_or(0) as u32;
                let rect = [c(0), c(1), c(2).saturating_sub(c(0)), c(3).saturating_sub(c(1))];
                if s.first.is_none() {
                    s.first = Some(rect);
                }
                s.frames.insert(k.clone(), rect);
            }
        }
        if let Some(al) = v.get("aliases").and_then(Value::as_object) {
            for (k, t) in al {
                if let Some(t) = t.as_str() {
                    s.aliases.insert(k.clone(), t.to_string());
                }
            }
        }
        s
    }

    /// The rectangle for `frame` (following aliases); the first frame if it isn't found.
    pub fn rect(&self, frame: &str) -> Option<[u32; 4]> {
        let mut f = frame.to_string();
        for _ in 0..8 {
            if let Some(r) = self.frames.get(&f) {
                return Some(*r);
            }
            match self.aliases.get(&f) {
                Some(t) => f = t.clone(),
                None => break,
            }
        }
        self.first
    }
}

/// Resolves `image.png:frame` specs to crop rectangles, caching parsed frames files.
#[derive(Default)]
pub struct Frames {
    cache: HashMap<String, Option<FrameSpec>>,
}

impl Frames {
    fn frames_path(a: &Assets, image: &str) -> Option<String> {
        let (dir, file) = image.rsplit_once('/')?;
        let stem = file.rsplit_once('.').map(|x| x.0).unwrap_or(file);
        let own = format!("{dir}/{stem}.frames");
        if a.exists(&own) {
            return Some(own);
        }
        let mut d = dir.to_string();
        loop {
            let def = format!("{d}/default.frames");
            if a.exists(&def) {
                return Some(def);
            }
            match d.rsplit_once('/') {
                Some((parent, _)) => d = parent.to_string(),
                None => return None,
            }
        }
    }

    /// `(image path, crop)` for an image spec such as `/items/x/sword.png:idle?hueshift=20`.
    pub fn resolve(&mut self, a: &Assets, image: &str, frame: Option<&str>) -> Option<[u32; 4]> {
        let frame = frame.filter(|f| !f.is_empty())?;
        let fp = Self::frames_path(a, image)?;
        let spec = self.cache.entry(fp.clone()).or_insert_with(|| a.json(&fp).ok().map(|v| FrameSpec::parse(&v)));
        // frame specs can contain tags like "<frame>": fall back to the first frame
        let f = if frame.contains('<') { "" } else { frame };
        spec.as_ref()?.rect(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grid_list_aliases() {
        let s = FrameSpec::parse(&json!({
            "frameGrid": {"size": [24, 64], "dimensions": [3, 2], "names": [["a", "b", null], ["d", "e", "f"]]},
            "frameList": {"x": [1, 2, 11, 22]},
            "aliases": {"z": "e", "y": "z"}
        }));
        assert_eq!(s.rect("b"), Some([24, 0, 24, 64]));
        assert_eq!(s.rect("f"), Some([48, 64, 24, 64]));
        assert_eq!(s.rect("y"), Some([24, 64, 24, 64]));
        assert_eq!(s.rect("x"), Some([1, 2, 10, 20]));
        assert_eq!(s.rect("missing"), Some([0, 0, 24, 64]));
    }
}
