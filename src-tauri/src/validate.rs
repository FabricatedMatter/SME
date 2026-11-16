//! Pre-flight checks that catch, in the editor, the mistakes that otherwise only show up as
//! errors in `starbound.log` (or as a dungeon silently never spawning).

use crate::assets::{relative_to, Assets};
use crate::catalog::{Catalog, ObjectInfo};
use crate::tmx::{Doc, TileRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Serialize, Clone, Debug)]
pub struct Problem {
    /// "error" (game will throw or the dungeon cannot spawn), "warning", or "info".
    pub severity: &'static str,
    pub file: String,
    pub message: String,
    /// Tile coordinates in editor space (x right, y down from the top row), if applicable.
    pub x: Option<i64>,
    pub y: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct FileJson {
    pub path: String,
    pub json: Value,
}

#[derive(Deserialize, Default)]
pub struct MapDoc {
    pub path: String,
    pub doc: Doc,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Input {
    pub dungeons: Vec<FileJson>,
    pub maps: Vec<MapDoc>,
    pub pools: Vec<FileJson>,
    pub patches: Vec<FileJson>,
    /// Project file paths (relative) that exist, for checking part `def` references.
    pub files: Vec<String>,
}

fn p(sev: &'static str, file: &str, msg: impl Into<String>) -> Problem {
    Problem { severity: sev, file: file.to_string(), message: msg.into(), x: None, y: None }
}

fn at(mut pr: Problem, x: i64, y: i64) -> Problem {
    pr.x = Some(x);
    pr.y = Some(y);
    pr
}

fn num(v: Option<&Value>) -> f64 {
    v.and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).or(v.as_f64())).unwrap_or(0.0)
}

pub fn run(a: &Assets, cat: &Catalog, objects: &BTreeMap<String, ObjectInfo>, input: &Input) -> Vec<Problem> {
    let mut out = Vec::new();
    let tiles: HashMap<&str, &crate::catalog::TilesetInfo> = cat.tilesets.iter().map(|t| (t.path.as_str(), t)).collect();
    let tile_props = |t: &TileRef| -> Option<Map<String, Value>> {
        tiles.get(t.tileset.as_str()).and_then(|ts| ts.tiles.get(t.index as usize)).map(|ti| ti.props.clone())
    };
    let files: HashSet<&str> = input.files.iter().map(|s| s.as_str()).collect();

    // ── treasure pools ──
    let mut vanilla_pools: HashMap<String, String> = HashMap::new();
    if let Some(pak) = a.vanilla() {
        for path in pak.paths().filter(|p| p.ends_with(".treasurepools")) {
            if let Ok(Value::Object(m)) = pak.read(path).map_err(anyhow::Error::from).and_then(|b| crate::sbjson::parse(&b)) {
                for k in m.keys() {
                    vanilla_pools.insert(k.clone(), path.clone());
                }
            }
        }
    }
    let mut project_pools: Map<String, Value> = Map::new();
    let mut pool_file: HashMap<String, String> = HashMap::new();
    for f in &input.pools {
        let asset = format!("/{}", f.path);
        let Some(m) = f.json.as_object() else {
            out.push(p("error", &f.path, "treasure pool file must be a JSON object"));
            continue;
        };
        for (k, v) in m {
            if let Some(vf) = vanilla_pools.get(k) {
                if *vf != asset {
                    out.push(p("error", &f.path, format!("pool '{k}' already exists in vanilla ({vf}); the game will refuse to load. Rename it.")));
                }
            }
            if let Some(other) = pool_file.get(k) {
                out.push(p("error", &f.path, format!("pool '{k}' is also defined in {other}")));
            }
            pool_file.insert(k.clone(), f.path.clone());
            match v.as_array() {
                Some(levels) if !levels.is_empty() => {
                    for l in levels {
                        if l.as_array().map(|x| x.len()) != Some(2) {
                            out.push(p("error", &f.path, format!("pool '{k}': each level entry must be [level, {{…}}]")));
                        } else if let Some(cfg) = l.get(1) {
                            if cfg.get("fill").is_none() && cfg.get("pool").is_none() {
                                out.push(p("warning", &f.path, format!("pool '{k}' has a level with neither fill nor pool items")));
                            }
                            for e in cfg.get("pool").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                                if e.get("weight").is_none() {
                                    out.push(p("error", &f.path, format!("pool '{k}': every weighted entry needs a weight")));
                                }
                            }
                        }
                    }
                }
                _ => out.push(p("error", &f.path, format!("pool '{k}' needs at least one level entry"))),
            }
            project_pools.insert(k.clone(), v.clone());
        }
    }
    let all_pool = |n: &str| project_pools.contains_key(n) || vanilla_pools.contains_key(n);
    for (k, v) in &project_pools {
        for lvl in v.as_array().cloned().unwrap_or_default() {
            for key in ["fill", "pool"] {
                for e in lvl.get(1).and_then(|c| c.get(key)).and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                    if let Some(r) = e.get("pool").and_then(|x| x.as_str()) {
                        if !all_pool(r) {
                            out.push(p("error", pool_file.get(k).map(|s| s.as_str()).unwrap_or(""), format!("pool '{k}' references unknown pool '{r}'")));
                        }
                    }
                }
            }
        }
    }
    for c in crate::treasure::find_cycles(&project_pools) {
        out.push(p("error", "", format!("treasure pool loop: {c}")));
    }

    // ── dungeons ──
    let mut vanilla_dungeons: HashMap<String, String> = HashMap::new();
    if let Some(pak) = a.vanilla() {
        for path in pak.paths().filter(|p| p.ends_with(".dungeon")) {
            if let Ok(v) = pak.read(path).map_err(anyhow::Error::from).and_then(|b| crate::sbjson::parse(&b)) {
                if let Some(n) = v.pointer("/metadata/name").and_then(|x| x.as_str()) {
                    vanilla_dungeons.insert(n.to_string(), path.clone());
                }
            }
        }
    }
    let mut seen_names: HashMap<String, String> = HashMap::new();
    let mut project_dungeons = HashSet::new();
    for d in &input.dungeons {
        let meta = d.json.get("metadata");
        let name = meta.and_then(|m| m.get("name")).and_then(|x| x.as_str()).unwrap_or("");
        if name.is_empty() {
            out.push(p("error", &d.path, "metadata.name is missing"));
        } else {
            project_dungeons.insert(name.to_string());
            if let Some(vf) = vanilla_dungeons.get(name) {
                if *vf != format!("/{}", d.path) {
                    out.push(p("error", &d.path, format!("dungeon name '{name}' is already used by vanilla ({vf})")));
                }
            }
            if let Some(o) = seen_names.insert(name.to_string(), d.path.clone()) {
                out.push(p("error", &d.path, format!("dungeon name '{name}' is also used by {o}")));
            }
        }
        if meta.and_then(|m| m.get("species")).is_none() {
            out.push(p("error", &d.path, "metadata.species is missing (use \"generic\")"));
        }
        let parts = d.json.get("parts").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        if parts.is_empty() {
            out.push(p("error", &d.path, "dungeon has no parts"));
        }
        let mut part_names = HashSet::new();
        for part in &parts {
            let pn = part.get("name").and_then(|x| x.as_str()).unwrap_or("");
            if pn.is_empty() {
                out.push(p("error", &d.path, "a part has no name"));
            } else if !part_names.insert(pn.to_string()) {
                out.push(p("error", &d.path, format!("duplicate part name '{pn}'")));
            }
            let def = part.get("def");
            let kind = def.and_then(|x| x.get(0)).and_then(|x| x.as_str()).unwrap_or("");
            if kind != "tmx" && kind != "image" {
                out.push(p("error", &d.path, format!("part '{pn}': def must be [\"tmx\", file] or [\"image\", file]")));
                continue;
            }
            let fl: Vec<String> = match def.and_then(|x| x.get(1)) {
                Some(Value::String(s)) => vec![s.clone()],
                Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
                _ => vec![],
            };
            for f in fl {
                let ap = relative_to(&format!("/{}", d.path), &f);
                let rel = ap.trim_start_matches('/');
                if !files.contains(rel) && !a.exists(&ap) {
                    out.push(p("error", &d.path, format!("part '{pn}': file {f} does not exist")));
                }
            }
        }
        for an in meta.and_then(|m| m.get("anchor")).and_then(|x| x.as_array()).cloned().unwrap_or_default() {
            if let Some(an) = an.as_str() {
                if !part_names.contains(an) {
                    out.push(p("error", &d.path, format!("anchor '{an}' is not a part of this dungeon")));
                }
            }
        }
        if meta.and_then(|m| m.get("anchor")).and_then(|x| x.as_array()).is_none_or(|x| x.is_empty()) {
            out.push(p("error", &d.path, "metadata.anchor is empty: the game needs at least one part to start from"));
        }
    }

    // ── maps ──
    for m in &input.maps {
        let doc = &m.doc;
        let h = doc.height as i64;
        for l in &doc.layers {
            if l.name != "front" && l.name != "back" {
                out.push(p("error", &m.path, format!("tile layer '{}' must be named 'front' or 'back'", l.name)));
            }
        }
        // world-space anchors
        let mut ground: (i64, i64) = (0, 0);
        let mut air: (i64, i64) = (0, h);
        let mut liquid: (i64, i64) = (0, 0);
        let mut has_anchor = false;
        let mut note = |props: &Map<String, Value>, x: i64, wy: i64| {
            if props.contains_key("worldGenMustContainSolid") && wy > ground.1 {
                ground = (x, wy);
            }
            if props.contains_key("worldGenMustContainAir") && wy < air.1 {
                air = (x, wy);
            }
            if (props.contains_key("worldGenMustContainLiquid") || props.contains_key("worldGenMustNotContainLiquid")) && wy > liquid.1 {
                liquid = (x, wy);
            }
        };
        let pal_props: Vec<Option<Map<String, Value>>> =
            doc.palette.iter().map(|t| t.as_ref().and_then(&tile_props)).collect();
        for (pi, t) in doc.palette.iter().enumerate() {
            if let Some(t) = t {
                if pal_props[pi].is_none() {
                    out.push(p("error", &m.path, format!("tile {}#{} no longer exists in that tileset", t.tileset, t.index)));
                }
            }
        }
        for l in &doc.layers {
            for (i, &c) in l.data.iter().enumerate() {
                if c == 0 {
                    continue;
                }
                if let Some(Some(pr)) = pal_props.get(c as usize) {
                    let (x, y) = ((i as i64) % doc.width as i64, (i as i64) / doc.width as i64);
                    if pr.keys().any(|k| k.starts_with("worldGen")) {
                        has_anchor = true;
                    }
                    note(pr, x, h - 1 - y);
                    if pr.get("invalid").and_then(|v| v.as_str()) == Some("true") {
                        out.push(at(p("error", &m.path, "tile marked invalid in its tileset (removed from the game)"), x, y));
                    }
                }
            }
        }
        // objects
        let mut placed: Vec<(String, i64, i64, bool)> = Vec::new();
        for g in &doc.groups {
            for o in &g.objects {
                let mut merged = g.properties.clone();
                if let Some(t) = o.tile {
                    if let Some(Some(tp)) = pal_props.get(t as usize) {
                        merged.extend(tp.clone());
                    }
                }
                merged.extend(o.properties.clone());
                let tx = (o.x as i64) / 8 - (num(merged.get("imagePositionX")) / 8.0) as i64;
                let ty = (o.y as i64) / 8 + (num(merged.get("imagePositionY")) / 8.0) as i64;
                if o.tile.is_some() {
                    let wy = h - ty;
                    note(&merged, tx, wy);
                    if merged.keys().any(|k| k.starts_with("worldGen")) {
                        has_anchor = true;
                    }
                    if let Some(name) = merged.get("object").and_then(|x| x.as_str()) {
                        let ey = h - 1 - wy;
                        match objects.get(name) {
                            None => out.push(at(p("error", &m.path, format!("unknown object '{name}'")), tx, ey)),
                            Some(info) => {
                                placed.push((name.to_string(), tx, wy, o.flip_x));
                                let params = merged.get("parameters").and_then(|v| match v {
                                    Value::String(s) => serde_json::from_str::<Value>(s).ok(),
                                    other => Some(other.clone()),
                                });
                                if merged.contains_key("parameters") && params.is_none() {
                                    out.push(at(p("error", &m.path, format!("{name}: 'parameters' is not valid JSON")), tx, ey));
                                }
                                if let Some(pools) = params.as_ref().and_then(|p| p.get("treasurePools")).and_then(|x| x.as_array()) {
                                    if !info.container {
                                        out.push(at(p("warning", &m.path, format!("{name} is not a container; treasurePools has no effect")), tx, ey));
                                    }
                                    for pl in pools.iter().filter_map(|x| x.as_str()) {
                                        if !all_pool(pl) {
                                            out.push(at(p("error", &m.path, format!("{name}: unknown treasure pool '{pl}'")), tx, ey));
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if o.polyline.is_none() && (o.width < 8.0 || o.height < 8.0) && !o.properties.contains_key("stagehand") {
                    out.push(at(p("warning", &m.path, "rectangle smaller than one tile affects nothing"), (o.x as i64) / 8, (o.y as i64) / 8));
                }
                if let Some(npc) = merged.get("npc").and_then(|x| x.as_str()) {
                    if npc.is_empty() {
                        out.push(at(p("error", &m.path, "npc brush without species"), tx, ty));
                    }
                }
            }
        }
        // wires: each vertex must land on an object's wire node
        let node_tiles: HashSet<(i64, i64)> = placed
            .iter()
            .flat_map(|(name, x, y, flip)| {
                let info = &objects[name];
                info.input_nodes
                    .iter()
                    .chain(info.output_nodes.iter())
                    .map(|n| {
                        let nx = if *flip { -n[0] } else { n[0] };
                        (x + nx as i64, y + n[1] as i64)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for g in &doc.groups {
            for o in &g.objects {
                if let (Some(pl), None) = (&o.polyline, o.tile) {
                    for q in pl {
                        let wx = (o.x as i64) / 8 + (q[0] as i64) / 8;
                        let wy = h - 1 - (o.y as i64) / 8 - (q[1] as i64) / 8;
                        if !node_tiles.contains(&(wx, wy)) {
                            out.push(at(
                                p("warning", &m.path, "wire point is not on an object's wire node (\"Dungeon wire endpoint not found\")"),
                                wx,
                                h - 1 - wy,
                            ));
                        }
                    }
                }
            }
        }
        let g1 = ground.1.max(liquid.1);
        if air.1 < g1 {
            out.push(at(
                p(
                    "error",
                    &m.path,
                    format!(
                        "anchor order: a 'must contain solid/liquid' anchor (row {}) is above the lowest 'must contain air' anchor (row {}). Move air anchors above solid ones.",
                        h - 1 - g1,
                        h - 1 - air.1
                    ),
                ),
                air.0,
                h - 1 - air.1,
            ));
        }
        if !has_anchor {
            out.push(p("warning", &m.path, "no world-gen anchors (air/solid markers): the part may spawn floating or buried"));
        }
    }

    // ── biome patches ──
    for f in &input.patches {
        let target = format!("/{}", f.path.trim_end_matches(".patch"));
        if !a.in_vanilla(&target) && !files.contains(target.trim_start_matches('/')) {
            out.push(p("warning", &f.path, format!("patch target {target} does not exist in vanilla assets")));
            continue;
        }
        let tj = a.json(&target).ok();
        let ops: Vec<Value> = match &f.json {
            Value::Array(arr) if arr.first().is_some_and(|x| x.is_array()) => arr.iter().flat_map(|x| x.as_array().cloned().unwrap_or_default()).collect(),
            Value::Array(arr) => arr.clone(),
            _ => {
                out.push(p("error", &f.path, "a patch file must be a JSON array of operations"));
                continue;
            }
        };
        for op in ops {
            let kind = op.get("op").and_then(|x| x.as_str()).unwrap_or("");
            let path = op.get("path").and_then(|x| x.as_str()).unwrap_or("");
            if !["add", "remove", "replace", "move", "copy", "test"].contains(&kind) {
                out.push(p("error", &f.path, format!("unknown patch op '{kind}'")));
                continue;
            }
            if kind == "add" {
                if let (Some(tj), Some(parent)) = (&tj, path.rsplit_once('/').map(|x| x.0)) {
                    if !parent.is_empty() && tj.pointer(parent).is_none() {
                        out.push(p("error", &f.path, format!("patch path {parent} does not exist in {target}")));
                    }
                }
            }
            if let Some(v) = op.get("value") {
                for name in v.get("microdungeons").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                    if let Some(n) = name.as_str() {
                        if !project_dungeons.contains(n) && !vanilla_dungeons.contains_key(n) {
                            out.push(p("error", &f.path, format!("spawns unknown dungeon '{n}'")));
                        }
                    }
                }
            }
        }
    }
    out
}
