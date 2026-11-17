//! Tiled JSON maps (Starbound's "tmx" dungeon parts) ⇄ the editor's document.
//!
//! The document keeps Tiled's geometry (row 0 = top row, object pixel coordinates with y down)
//! so a round trip is lossless, but replaces fragile gids with (tileset, index) references.
//! Parsing rules follow the game's `StarDungeonTMXPart.cpp`.

use anyhow::{anyhow, bail, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;

pub const FLIP_H: u32 = 0x8000_0000;
pub const FLIP_V: u32 = 0x4000_0000;
pub const FLIP_D: u32 = 0x2000_0000;
pub const FLIP_ALL: u32 = FLIP_H | FLIP_V | FLIP_D;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct TileRef {
    /// Asset path of the tileset, e.g. `/tilesets/packed/materials.json`.
    pub tileset: String,
    pub index: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TileLayer {
    pub name: String,
    /// Palette indices (0 = empty), row-major, row 0 = top.
    pub data: Vec<u32>,
    #[serde(default = "one")]
    pub opacity: f64,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub properties: Map<String, Value>,
}

fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Obj {
    pub id: u32,
    /// Palette index for tile objects (placed Starbound objects, single anchors, …).
    #[serde(default)]
    pub tile: Option<u32>,
    #[serde(default)]
    pub flip_x: bool,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    /// Wire: points relative to (x, y) in pixels.
    #[serde(default)]
    pub polyline: Option<Vec<[f64; 2]>>,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(default = "yes")]
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjGroup {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(default = "yes")]
    pub visible: bool,
    pub objects: Vec<Obj>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TilesetRef {
    pub path: String,
    /// The `source` string as written in the imported file (kept for Tiled compatibility).
    #[serde(default)]
    pub source: Option<String>,
    /// Tile count to assume if the tileset cannot be found in the assets.
    #[serde(default)]
    pub tilecount_hint: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Doc {
    pub width: u32,
    pub height: u32,
    /// palette[0] is always None (empty cell).
    pub palette: Vec<Option<TileRef>>,
    pub layers: Vec<TileLayer>,
    pub groups: Vec<ObjGroup>,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(default)]
    pub tilesets: Vec<TilesetRef>,
    #[serde(default)]
    pub next_object_id: u32,
}

/// Normalises Tiled properties (object `{k: v}` or array `[{name, type, value}]`) to a map.
pub fn norm_props(v: Option<&Value>) -> Map<String, Value> {
    match v {
        Some(Value::Object(m)) => m.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|p| Some((p.get("name")?.as_str()?.to_string(), p.get("value")?.clone())))
            .collect(),
        _ => Map::new(),
    }
}

/// Property keys that make a tile/object actually change the world.
pub fn is_brush_key(k: &str) -> bool {
    matches!(
        k,
        "object" | "material" | "front" | "back" | "liquid" | "npc" | "monster" | "stagehand" | "item" | "biomeitems" | "biometree"
            | "wire" | "clear" | "playerstart" | "dungeonid" | "connector" | "surface" | "vehicle" | "mod" | "invalid"
    ) || k.starts_with("worldGen")
}

/// Starbound's `tilesetAssetPath`: everything from the last `/tilesets/` on.
pub fn tileset_asset_path(source: &str) -> String {
    let s = source.replace('\\', "/");
    match s.to_lowercase().rfind("/tilesets/") {
        Some(i) => s[i..].to_string(),
        None if s.to_lowercase().starts_with("tilesets/") => format!("/{s}"),
        None => s,
    }
}

pub fn decode_layer(l: &Value, n: usize) -> Result<Vec<u32>> {
    let data = l.get("data").ok_or_else(|| anyhow!("tile layer without data"))?;
    let out: Vec<u32> = if let Some(arr) = data.as_array() {
        arr.iter().map(|x| x.as_u64().unwrap_or(0) as u32).collect()
    } else {
        let raw = base64::engine::general_purpose::STANDARD.decode(data.as_str().unwrap_or("").trim())?;
        let comp = l.get("compression").and_then(|x| x.as_str()).unwrap_or("");
        let bytes = match comp {
            "" => raw,
            "zlib" => {
                let mut o = Vec::new();
                flate2::read::ZlibDecoder::new(&raw[..]).read_to_end(&mut o)?;
                o
            }
            "gzip" => {
                let mut o = Vec::new();
                flate2::read::GzDecoder::new(&raw[..]).read_to_end(&mut o)?;
                o
            }
            c => bail!("unsupported tile layer compression '{c}'"),
        };
        bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
    };
    if out.len() != n {
        bail!("tile layer data length {} does not match {n}", out.len());
    }
    Ok(out)
}

/// gid → (tileset index in list, local index), using "highest firstgid ≤ gid" like the game.
struct GidMap {
    sets: Vec<(u32, u32, String)>, // firstgid, count, asset path
}

impl GidMap {
    fn resolve(&self, gid: u32) -> Option<TileRef> {
        let gid = gid & !FLIP_ALL;
        if gid == 0 {
            return None;
        }
        self.sets
            .iter()
            .filter(|(f, c, _)| *f <= gid && gid < f + c)
            .max_by_key(|(f, _, _)| *f)
            .map(|(f, _, p)| TileRef { tileset: p.clone(), index: gid - f })
    }
}

#[derive(Serialize)]
pub struct Imported {
    pub doc: Doc,
    pub warnings: Vec<String>,
}

/// `tilecount(asset_path)` returns the current size of a tileset if it exists.
pub fn import(map: &Value, tilecount: &dyn Fn(&str) -> Option<u32>) -> Result<Imported> {
    let mut warnings = Vec::new();
    let tw = map.get("tilewidth").and_then(|x| x.as_u64()).unwrap_or(0);
    let th = map.get("tileheight").and_then(|x| x.as_u64()).unwrap_or(0);
    if tw != 8 || th != 8 {
        warnings.push(format!("tile size is {tw}×{th}; Starbound requires 8×8"));
    }
    if map.get("infinite").and_then(|x| x.as_bool()) == Some(true) {
        bail!("infinite Tiled maps are not supported by Starbound");
    }
    let width = map.get("width").and_then(|x| x.as_u64()).ok_or_else(|| anyhow!("map has no width"))? as u32;
    let height = map.get("height").and_then(|x| x.as_u64()).ok_or_else(|| anyhow!("map has no height"))? as u32;

    let mut raw_sets: Vec<(u32, String, Option<String>)> = Vec::new();
    for ts in map.get("tilesets").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
        let first = ts.get("firstgid").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
        match ts.get("source").and_then(|x| x.as_str()) {
            Some(src) => raw_sets.push((first, tileset_asset_path(src), Some(src.to_string()))),
            None => {
                let name = ts.get("name").and_then(|x| x.as_str()).unwrap_or("?");
                warnings.push(format!("embedded tileset '{name}' is not supported by Starbound and was dropped"));
            }
        }
    }
    raw_sets.sort_by_key(|s| s.0);
    let mut tilesets = Vec::new();
    let mut sets = Vec::new();
    for (i, (first, path, src)) in raw_sets.iter().enumerate() {
        let hint = raw_sets.get(i + 1).map(|n| n.0 - first).unwrap_or(100_000);
        let count = match tilecount(path) {
            Some(c) => c,
            None => {
                warnings.push(format!("tileset {path} not found in assets"));
                hint
            }
        };
        sets.push((*first, count, path.clone()));
        tilesets.push(TilesetRef { path: path.clone(), source: src.clone(), tilecount_hint: count });
    }
    let gm = GidMap { sets };

    let mut palette: Vec<Option<TileRef>> = vec![None];
    let mut pal_idx: HashMap<TileRef, u32> = HashMap::new();
    let mut intern = |gid: u32, warnings: &mut Vec<String>| -> u32 {
        if gid & !FLIP_ALL == 0 {
            return 0;
        }
        match gm.resolve(gid) {
            Some(t) => *pal_idx.entry(t.clone()).or_insert_with(|| {
                palette.push(Some(t));
                (palette.len() - 1) as u32
            }),
            None => {
                warnings.push(format!("gid {} does not belong to any tileset", gid & !FLIP_ALL));
                0
            }
        }
    };

    let mut layers = Vec::new();
    let mut groups = Vec::new();
    let mut max_id = 0u32;
    for l in map.get("layers").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
        let ty = l.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let name = l.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        match ty {
            "tilelayer" => {
                let lw = l.get("width").and_then(|x| x.as_u64()).unwrap_or(width as u64) as u32;
                let lh = l.get("height").and_then(|x| x.as_u64()).unwrap_or(height as u64) as u32;
                let raw = decode_layer(&l, (lw * lh) as usize)?;
                if raw.iter().any(|g| g & FLIP_ALL != 0) {
                    warnings.push(format!("layer '{name}': flipped tiles are ignored by Starbound"));
                }
                // re-grid onto the map size (layers with an offset/size are rare)
                let ox = l.get("x").and_then(|x| x.as_i64()).unwrap_or(0) as i64;
                let oy = l.get("y").and_then(|x| x.as_i64()).unwrap_or(0) as i64;
                let mut data = vec![0u32; (width * height) as usize];
                for y in 0..lh as i64 {
                    for x in 0..lw as i64 {
                        let (mx, my) = (x + ox, y + oy);
                        if mx >= 0 && my >= 0 && (mx as u32) < width && (my as u32) < height {
                            data[(my as u32 * width + mx as u32) as usize] =
                                intern(raw[(y as u32 * lw + x as u32) as usize], &mut warnings);
                        }
                    }
                }
                if name != "front" && name != "back" {
                    warnings.push(format!("tile layer '{name}' must be named 'front' or 'back'"));
                }
                layers.push(TileLayer {
                    name,
                    data,
                    opacity: l.get("opacity").and_then(|x| x.as_f64()).unwrap_or(1.0),
                    visible: l.get("visible").and_then(|x| x.as_bool()).unwrap_or(true),
                    properties: norm_props(l.get("properties")),
                });
            }
            "objectgroup" => {
                let mut objects = Vec::new();
                for o in l.get("objects").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                    let id = o.get("id").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                    max_id = max_id.max(id);
                    let gid = o.get("gid").and_then(|x| x.as_u64()).map(|g| g as u32);
                    if o.get("ellipse").is_some() || o.get("polygon").is_some() {
                        warnings.push(format!("object {id}: ellipses/polygons are not supported and were dropped"));
                        continue;
                    }
                    if o.get("rotation").and_then(|x| x.as_f64()).unwrap_or(0.0) != 0.0 {
                        warnings.push(format!("object {id}: rotation is not supported and was reset"));
                    }
                    let tile = gid.map(|g| intern(g, &mut warnings));
                    if let Some(g) = gid {
                        if g & (FLIP_V | FLIP_D) != 0 {
                            warnings.push(format!("object {id}: vertical/diagonal flips are not supported and were removed"));
                        }
                    }
                    let f = |k: &str| o.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
                    if tile == Some(0) {
                        // A tile object whose tile no longer exists. The game gives it no
                        // properties from the tileset; unless the object carries its own brush
                        // properties it does nothing, so dropping it keeps the map's meaning.
                        let props = norm_props(o.get("properties"));
                        if props.keys().any(|k| is_brush_key(k)) {
                            warnings.push(format!("object {id} uses a missing tile; kept as a 1×1 brush"));
                            objects.push(Obj {
                                id,
                                x: f("x"),
                                y: f("y") - 8.0,
                                width: 8.0,
                                height: 8.0,
                                properties: props,
                                visible: true,
                                ..Default::default()
                            });
                        } else {
                            warnings.push(format!("object {id} uses a missing tile and has no effect; removed"));
                        }
                        continue;
                    }
                    objects.push(Obj {
                        id,
                        tile,
                        flip_x: gid.is_some_and(|g| g & FLIP_H != 0),
                        x: f("x"),
                        y: f("y"),
                        width: f("width"),
                        height: f("height"),
                        polyline: o.get("polyline").and_then(|x| x.as_array()).map(|pts| {
                            pts.iter().map(|p| [p.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0), p.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0)]).collect()
                        }),
                        name: o.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        kind: o.get("type").or(o.get("class")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        properties: norm_props(o.get("properties")),
                        visible: o.get("visible").and_then(|x| x.as_bool()).unwrap_or(true),
                    });
                }
                groups.push(ObjGroup {
                    name,
                    color: l.get("color").and_then(|x| x.as_str()).map(String::from),
                    properties: norm_props(l.get("properties")),
                    visible: l.get("visible").and_then(|x| x.as_bool()).unwrap_or(true),
                    objects,
                });
            }
            other => warnings.push(format!("layer '{name}' of type '{other}' is not supported and was dropped")),
        }
    }
    let next = map.get("nextobjectid").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    warnings.dedup();
    Ok(Imported {
        doc: Doc {
            width,
            height,
            palette,
            layers,
            groups,
            properties: norm_props(map.get("properties")),
            tilesets,
            next_object_id: next.max(max_id + 1),
        },
        warnings,
    })
}

/// Canonical order for tilesets that were not in the imported file.
fn tileset_rank(p: &str) -> (u8, String) {
    let r = if p.ends_with("/materials.json") {
        0
    } else if p.ends_with("/miscellaneous.json") {
        1
    } else if p.ends_with("/liquids.json") {
        2
    } else if p.ends_with("/supports.json") {
        3
    } else if p.starts_with("/tilesets/packed/") {
        4
    } else {
        5
    };
    (r, p.to_string())
}

/// Whole numbers as JSON integers (like Tiled / vanilla files), others as floats.
fn jnum(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn stringify_props(m: &Map<String, Value>) -> Value {
    Value::Object(
        m.iter()
            .map(|(k, v)| {
                let s = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), Value::String(s))
            })
            .collect(),
    )
}

/// `rel_root` is the relative path from the map file's folder to the mod/asset root
/// (e.g. `../../../`), used for tileset `source` paths of tilesets not in the original file.
pub fn export(doc: &Doc, tilecount: &dyn Fn(&str) -> Option<u32>, rel_root: &str) -> Result<Value> {
    // which tilesets are actually used
    let mut used: BTreeMap<String, ()> = BTreeMap::new();
    let mut mark = |idx: u32| {
        if let Some(Some(t)) = doc.palette.get(idx as usize) {
            used.insert(t.tileset.clone(), ());
        }
    };
    for l in &doc.layers {
        l.data.iter().for_each(|&i| mark(i));
    }
    for g in &doc.groups {
        for o in &g.objects {
            if let Some(t) = o.tile {
                mark(t);
            }
        }
    }
    let mut order: Vec<String> = doc.tilesets.iter().map(|t| t.path.clone()).filter(|p| used.contains_key(p)).collect();
    let mut rest: Vec<String> = used.keys().filter(|p| !order.contains(p)).cloned().collect();
    rest.sort_by_key(|p| tileset_rank(p));
    order.extend(rest);

    let mut first: HashMap<String, u32> = HashMap::new();
    let mut ts_json = Vec::new();
    let mut next = 1u32;
    for p in &order {
        let hint = doc.tilesets.iter().find(|t| &t.path == p).map(|t| t.tilecount_hint).unwrap_or(0);
        let count = tilecount(p).unwrap_or(hint).max(1);
        first.insert(p.clone(), next);
        let source = doc
            .tilesets
            .iter()
            .find(|t| &t.path == p)
            .and_then(|t| t.source.clone())
            .filter(|s| tileset_asset_path(s) == *p)
            .unwrap_or_else(|| format!("{}{}", rel_root, p.trim_start_matches('/')));
        ts_json.push(json!({ "firstgid": next, "source": source }));
        next += count;
    }
    let gid = |idx: u32| -> u32 {
        match doc.palette.get(idx as usize) {
            Some(Some(t)) => first.get(&t.tileset).map(|f| f + t.index).unwrap_or(0),
            _ => 0,
        }
    };

    let mut layers = Vec::new();
    for l in &doc.layers {
        let mut o = json!({
            "data": l.data.iter().map(|&i| gid(i)).collect::<Vec<_>>(),
            "height": doc.height,
            "name": l.name,
            "opacity": jnum(l.opacity),
            "type": "tilelayer",
            "visible": l.visible,
            "width": doc.width,
            "x": 0,
            "y": 0
        });
        if !l.properties.is_empty() {
            o["properties"] = stringify_props(&l.properties);
        }
        layers.push(o);
    }
    for g in &doc.groups {
        let objects: Vec<Value> = g
            .objects
            .iter()
            .map(|o| {
                let mut v = json!({
                    "height": jnum(o.height),
                    "id": o.id,
                    "name": o.name,
                    "properties": stringify_props(&o.properties),
                    "rotation": 0,
                    "type": o.kind,
                    "visible": o.visible,
                    "width": jnum(o.width),
                    "x": jnum(o.x),
                    "y": jnum(o.y)
                });
                if let Some(t) = o.tile {
                    let g = gid(t);
                    if g != 0 {
                        v["gid"] = json!(g | if o.flip_x { FLIP_H } else { 0 });
                    }
                }
                if let Some(pl) = &o.polyline {
                    v["polyline"] = Value::Array(pl.iter().map(|p| json!({"x": jnum(p[0]), "y": jnum(p[1])})).collect());
                }
                v
            })
            .collect();
        let mut o = json!({
            "draworder": "topdown",
            "height": 0,
            "name": g.name,
            "objects": objects,
            "opacity": 1,
            "type": "objectgroup",
            "visible": g.visible,
            "width": 0,
            "x": 0,
            "y": 0
        });
        if let Some(c) = &g.color {
            o["color"] = json!(c);
        }
        if !g.properties.is_empty() {
            o["properties"] = stringify_props(&g.properties);
        }
        layers.push(o);
    }
    let max_id = doc.groups.iter().flat_map(|g| g.objects.iter().map(|o| o.id)).max().unwrap_or(0);
    Ok(json!({
        "backgroundcolor": "#000000",
        "height": doc.height,
        "layers": layers,
        "nextobjectid": doc.next_object_id.max(max_id + 1),
        "orientation": "orthogonal",
        "properties": stringify_props(&doc.properties),
        "renderorder": "right-down",
        "tileheight": 8,
        "tilesets": ts_json,
        "tilewidth": 8,
        "version": 1,
        "width": doc.width
    }))
}

/// Mimics the game's `TMXMap::forEachTile`: every (world position, layer, merged properties)
/// brush application, in a canonical sorted form. Used to verify that import→export keeps
/// the meaning of a map, and by the validator.
pub fn semantic(map: &Value, tile_props: &dyn Fn(&str, u32) -> Option<Map<String, Value>>, tilecount: &dyn Fn(&str) -> Option<u32>) -> Result<Vec<String>> {
    let height = map.get("height").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut sets: Vec<(u32, u32, String)> = Vec::new();
    for ts in map.get("tilesets").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
        if let Some(src) = ts.get("source").and_then(|x| x.as_str()) {
            let p = tileset_asset_path(src);
            let f = ts.get("firstgid").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
            sets.push((f, tilecount(&p).unwrap_or(0), p));
        }
    }
    let gm = GidMap { sets };
    let props_of = |gid: u32| -> Option<Map<String, Value>> {
        let t = gm.resolve(gid)?;
        tile_props(&t.tileset, t.index)
    };
    let fmt = |m: &Map<String, Value>| -> String {
        let mut kv: Vec<String> = m
            .iter()
            .filter(|(k, _)| !k.starts_with("//"))
            .map(|(k, v)| format!("{k}={}", v.as_str().map(String::from).unwrap_or(v.to_string())))
            .collect();
        kv.sort();
        kv.join(",")
    };
    let mut out = Vec::new();
    for l in map.get("layers").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
        let lname = l.get("name").and_then(|x| x.as_str()).unwrap_or("");
        match l.get("type").and_then(|x| x.as_str()) {
            Some("tilelayer") => {
                let w = l.get("width").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let h = l.get("height").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let data = decode_layer(&l, w * h)?;
                for (i, g) in data.iter().enumerate() {
                    let g = g & !FLIP_ALL;
                    if g == 0 {
                        continue; // empty (front: nothing, back: implicit clear) – same both ways
                    }
                    let (x, y) = ((i % w) as i64, (i / w) as i64);
                    // unresolved gids behave exactly like empty cells in the game
                    if let Some(m) = props_of(g) {
                        out.push(format!("{lname}@{},{}:{}", x, height - 1 - y, fmt(&m)));
                    }
                }
            }
            Some("objectgroup") => {
                let gprops = norm_props(l.get("properties"));
                for o in l.get("objects").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                    let oprops = norm_props(o.get("properties"));
                    let gid = o.get("gid").and_then(|x| x.as_u64()).map(|g| g as u32);
                    if gid.is_some_and(|g| props_of(g).is_none()) && !oprops.keys().any(|k| is_brush_key(k)) {
                        continue; // tile object without a tile and without own brushes: no effect
                    }
                    let mut merged = gprops.clone();
                    if let Some(g) = gid {
                        if let Some(tp) = props_of(g) {
                            merged.extend(tp);
                        }
                    }
                    merged.extend(oprops.clone());
                    let flip = gid.is_some_and(|g| g & FLIP_H != 0);
                    let ipx = merged.get("imagePositionX").and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).or(v.as_f64())).unwrap_or(0.0);
                    let ipy = merged.get("imagePositionY").and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).or(v.as_f64())).unwrap_or(0.0);
                    let fx = o.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                    let fy = o.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                    let px = fx - (ipx / 8.0) as i64;
                    let py = fy + (ipy / 8.0) as i64;
                    let w = o.get("width").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                    let h = o.get("height").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                    let p = fmt(&merged);
                    if oprops.contains_key("stagehand") {
                        out.push(format!("stagehand@{px},{py},{w},{h}:{p}"));
                    } else if gid.is_some() {
                        out.push(format!("obj@{},{} flip={flip}:{p}", px, height - py));
                    } else if let Some(pl) = o.get("polyline").and_then(|x| x.as_array()) {
                        let pts: Vec<String> = pl
                            .iter()
                            .map(|q| {
                                let qx = q.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                                let qy = q.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0) as i64 / 8;
                                format!("{},{}", px + qx, height - 1 - py - qy)
                            })
                            .collect();
                        out.push(format!("wire[{}]:{p}", pts.join(" ")));
                    } else {
                        out.push(format!("rect@{px},{py},{w},{h}:{p}"));
                    }
                }
            }
            _ => {}
        }
    }
    out.sort();
    Ok(out)
}
