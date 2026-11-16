//! Everything the editor needs to know about the game's building blocks, gathered once from
//! the assets: Tiled tilesets (the brush catalogue), materials / platforms / matmods with their
//! render templates (for joint tiling), liquids, and the object database.

use crate::assets::{relative_to, Assets};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Serialize, Clone)]
pub struct TileInfo {
    /// Tiled tile properties exactly as in the tileset (`material`, `object`, `clear`, …).
    pub props: Map<String, Value>,
    /// Preview image: `i:<path under install>` or `a:<asset path>`.
    pub image: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct TilesetInfo {
    /// Asset path, e.g. `/tilesets/packed/materials.json`.
    pub path: String,
    pub name: String,
    pub tilecount: u32,
    pub properties: Map<String, Value>,
    pub tiles: Vec<TileInfo>,
    /// False when the tileset comes from the open project rather than vanilla.
    pub vanilla: bool,
}

#[derive(Serialize, Clone)]
pub struct MaterialInfo {
    pub name: String,
    pub id: i64,
    pub path: String,
    pub shortdescription: String,
    pub category: String,
    /// Render template asset path (key into `Catalog::templates`).
    pub template: Option<String>,
    pub params: Value,
    /// Main texture as an asset path.
    pub texture: Option<String>,
    /// For platforms / non-blocks (rendered behind blocks in game).
    pub collision: String,
}

#[derive(Serialize, Clone)]
pub struct LiquidInfo {
    pub name: String,
    pub id: i64,
    pub color: Vec<u8>,
    pub texture: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Catalog {
    pub install: String,
    pub tilesets: Vec<TilesetInfo>,
    pub materials: BTreeMap<String, MaterialInfo>,
    pub mods: BTreeMap<String, MaterialInfo>,
    pub liquids: BTreeMap<String, LiquidInfo>,
    pub templates: BTreeMap<String, Value>,
    pub background_color: Vec<u8>,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// Turns a Tiled image path into something the UI protocol can serve.
fn image_ref(tileset_path: &str, image: &str) -> String {
    let img = image.replace('\\', "/");
    if let Some(i) = img.find("tiled/packed/") {
        return format!("i:{}", &img[i..]);
    }
    format!("a:{}", relative_to(tileset_path, &img))
}

pub fn parse_tileset(path: &str, v: &Value, vanilla: bool) -> TilesetInfo {
    let tilecount = v.get("tilecount").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    let props = v.get("tileproperties").and_then(|x| x.as_object()).cloned().unwrap_or_default();
    let images = v.get("tiles").cloned().unwrap_or(Value::Null);
    let mut tiles = Vec::with_capacity(tilecount as usize);
    for i in 0..tilecount {
        let k = i.to_string();
        let mut p = props.get(&k).and_then(|x| x.as_object()).cloned().unwrap_or_default();
        // modern Tiled (>=1.2) format: tiles: [{id, image, properties:[{name,value}]}]
        let mut image = images.get(&k).and_then(|t| t.get("image")).and_then(|x| x.as_str()).map(String::from);
        if let Some(arr) = images.as_array() {
            if let Some(t) = arr.iter().find(|t| t.get("id").and_then(|x| x.as_u64()) == Some(i as u64)) {
                image = image.or(t.get("image").and_then(|x| x.as_str()).map(String::from));
                if let Some(pa) = t.get("properties").and_then(|x| x.as_array()) {
                    for e in pa {
                        if let (Some(n), Some(val)) = (e.get("name").and_then(|x| x.as_str()), e.get("value")) {
                            p.insert(n.into(), val.clone());
                        }
                    }
                }
            }
        }
        tiles.push(TileInfo { props: p, image: image.map(|im| image_ref(path, &im)) });
    }
    TilesetInfo {
        path: path.to_string(),
        name: s(v, "name"),
        tilecount,
        properties: v.get("properties").and_then(|x| x.as_object()).cloned().unwrap_or_default(),
        tiles,
        vanilla,
    }
}

fn material_info(a: &Assets, path: &str, v: &Value, name_key: &str, id_key: &str) -> MaterialInfo {
    let params = v.get("renderParameters").cloned().unwrap_or(Value::Null);
    let texture = params.get("texture").and_then(|x| x.as_str()).map(|t| relative_to(path, t));
    let _ = a;
    MaterialInfo {
        name: s(v, name_key),
        id: v.get(id_key).and_then(|x| x.as_i64()).unwrap_or(-1),
        path: path.to_string(),
        shortdescription: s(v, "shortdescription"),
        category: s(v, "category"),
        template: v.get("renderTemplate").and_then(|x| x.as_str()).map(|t| relative_to(path, t)),
        params,
        texture,
        collision: v.get("collisionKind").and_then(|x| x.as_str()).unwrap_or("solid").to_string(),
    }
}

pub fn build(a: &Assets) -> Catalog {
    let mut tilesets = Vec::new();
    for p in a.list(|p| p.starts_with("/tilesets/") && p.ends_with(".json")) {
        if let Ok(v) = a.json(&p) {
            tilesets.push(parse_tileset(&p, &v, a.in_vanilla(&p)));
        }
    }
    let mut materials = BTreeMap::new();
    let mut templates = BTreeMap::new();
    for p in a.list(|p| p.ends_with(".material")) {
        if let Ok(v) = a.json(&p) {
            let m = material_info(a, &p, &v, "materialName", "materialId");
            if !m.name.is_empty() {
                materials.insert(m.name.clone(), m);
            }
        }
    }
    let mut mods = BTreeMap::new();
    for p in a.list(|p| p.ends_with(".matmod")) {
        if let Ok(v) = a.json(&p) {
            let m = material_info(a, &p, &v, "modName", "modId");
            if !m.name.is_empty() {
                mods.insert(m.name.clone(), m);
            }
        }
    }
    for m in materials.values().chain(mods.values()) {
        if let Some(t) = &m.template {
            if !templates.contains_key(t) {
                if let Ok(v) = a.json(t) {
                    templates.insert(t.clone(), v);
                }
            }
        }
    }
    let mut liquids = BTreeMap::new();
    for p in a.list(|p| p.ends_with(".liquid")) {
        if let Ok(v) = a.json(&p) {
            let name = s(&v, "name");
            liquids.insert(
                name.clone(),
                LiquidInfo {
                    name,
                    id: v.get("liquidId").and_then(|x| x.as_i64()).unwrap_or(-1),
                    color: v
                        .get("color")
                        .and_then(|x| x.as_array())
                        .map(|c| c.iter().map(|x| x.as_u64().unwrap_or(255) as u8).collect())
                        .unwrap_or(vec![80, 120, 255, 128]),
                    texture: v.get("texture").and_then(|x| x.as_str()).map(|t| relative_to(&p, t)),
                },
            );
        }
    }
    let background_color = a
        .json("/rendering.config")
        .ok()
        .and_then(|r| r.get("backgroundLayerColor").cloned())
        .and_then(|c| c.as_array().map(|c| c.iter().map(|x| x.as_u64().unwrap_or(128) as u8).collect()))
        .unwrap_or(vec![128, 128, 128]);
    Catalog {
        install: a.install.to_string_lossy().into_owned(),
        tilesets,
        materials,
        mods,
        liquids,
        templates,
        background_color,
    }
}

// ───────────────────────── objects ─────────────────────────

#[derive(Serialize, Clone)]
pub struct ObjectInfo {
    pub name: String,
    pub path: String,
    pub shortdescription: String,
    pub category: String,
    pub race: String,
    pub object_type: String,
    pub colony_tags: Vec<String>,
    /// Containers (chests etc.) can take `treasurePools`.
    pub container: bool,
    pub slot_count: u32,
    pub input_nodes: Vec<[i32; 2]>,
    pub output_nodes: Vec<[i32; 2]>,
    /// Inventory icon as an asset path (with frame/directives stripped).
    pub icon: Option<String>,
}

fn nodes(v: &Value, k: &str) -> Vec<[i32; 2]> {
    v.get(k)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let p = p.as_array()?;
                    Some([p.first()?.as_i64()? as i32, p.get(1)?.as_i64()? as i32])
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn build_objects(a: &Assets) -> BTreeMap<String, ObjectInfo> {
    let mut out = BTreeMap::new();
    for p in a.list(|p| p.ends_with(".object")) {
        let Ok(v) = a.json(&p) else { continue };
        let name = s(&v, "objectName");
        if name.is_empty() {
            continue;
        }
        let object_type = s(&v, "objectType");
        let slot_count = v.get("slotCount").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let icon = v
            .get("inventoryIcon")
            .and_then(|x| x.as_str())
            .map(|i| relative_to(&p, i.split(['?', ':']).next().unwrap_or(i)));
        out.insert(
            name.clone(),
            ObjectInfo {
                name,
                path: p.clone(),
                shortdescription: s(&v, "shortdescription"),
                category: s(&v, "category"),
                race: s(&v, "race"),
                container: object_type == "container" || slot_count > 0,
                object_type,
                colony_tags: v
                    .get("colonyTags")
                    .and_then(|x| x.as_array())
                    .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                slot_count,
                input_nodes: nodes(&v, "inputNodes"),
                output_nodes: nodes(&v, "outputNodes"),
                icon,
            },
        );
    }
    out
}
