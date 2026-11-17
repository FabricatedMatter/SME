//! Which objects and items a player can actually come across outside of story missions.
//!
//! Nothing in the game files says "mission only", so this is worked out from how things are
//! used: an object counts as normal if the player can obtain it (crafting, loot, shops, quest
//! rewards, …), if biomes place it, or if a dungeon on a regular planet uses it. Everything else
//! (protectorate backgrounds, ship booster flames, outpost shop fronts, NPC-only weapons, …) is
//! hidden by default in the palette and the loot item list.

use crate::assets::{relative_to, Assets};
use crate::catalog::Catalog;
use crate::tmx;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Availability {
    /// Item / object names a player can obtain.
    obtainable: HashSet<String>,
    /// Object names placed by biomes or by dungeons on regular planets.
    seen: HashSet<String>,
}

impl Availability {
    pub fn hidden_object(&self, name: &str) -> bool {
        !self.obtainable.contains(name) && !self.seen.contains(name)
    }

    /// `kind` is the item's file extension (`object`, `liqitem`, …).
    pub fn hidden_item(&self, name: &str, kind: &str) -> bool {
        match kind {
            "object" => self.hidden_object(name),
            // the matter manipulator's liquid collector picks up any liquid
            "liqitem" => false,
            _ => !self.obtainable.contains(name),
        }
    }
}

fn strings(v: &Value, out: &mut HashSet<String>) {
    match v {
        Value::String(s) => {
            out.insert(s.clone());
        }
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(m) => {
            for (k, x) in m {
                out.insert(k.clone());
                strings(x, out);
            }
        }
        _ => {}
    }
}

/// Only vanilla files count: the result is cached across project changes, and the user's own
/// recipes / loot shouldn't make mission props look "normal".
fn each_json(a: &Assets, pred: impl Fn(&str) -> bool, mut f: impl FnMut(&str, &Value)) {
    for p in a.list(|p| pred(p) && a.in_vanilla(p)) {
        if let Ok(v) = a.json(&p) {
            f(&p, &v);
        }
    }
}

/// Dungeons that only exist inside missions, the outpost, the Ark, ships, stations and so on.
fn mission_dungeon(name: &str, instance: &HashSet<String>, on_planets: &HashSet<String>) -> bool {
    if instance.contains(name) {
        return true;
    }
    if on_planets.contains(name) {
        return false;
    }
    const PREFIXES: &[&str] = &[
        "outpost", "playerstation", "peacekeeperstation", "genboss", "spacestation", "bountyspace", "bountyship", "bountylair",
        "ancientvault", "cultist", "museum", "protectorate", "arena", "testarena", "techchallenge", "mechtest", "challengerooms",
    ];
    PREFIXES.iter().any(|p| name.starts_with(p)) || name.contains("mission") || name.contains("encounter") || name.ends_with("ship")
}

/// Object names placed by one Tiled map.
fn map_objects(a: &Assets, map_path: &str, props: &HashMap<&str, &crate::catalog::TilesetInfo>, out: &mut HashSet<String>) {
    let Ok(m) = a.json(map_path) else { return };
    let mut sets: Vec<(u32, &crate::catalog::TilesetInfo)> = Vec::new();
    for ts in m.get("tilesets").and_then(|x| x.as_array()).into_iter().flatten() {
        let (Some(first), Some(src)) = (ts.get("firstgid").and_then(|x| x.as_u64()), ts.get("source").and_then(|x| x.as_str())) else {
            continue;
        };
        if let Some(t) = props.get(tmx::tileset_asset_path(src).as_str()) {
            sets.push((first as u32, t));
        }
    }
    sets.sort_by_key(|(f, _)| std::cmp::Reverse(*f));
    let mut gids = HashSet::new();
    for l in m.get("layers").and_then(|x| x.as_array()).into_iter().flatten() {
        if l.get("type").and_then(|x| x.as_str()) == Some("tilelayer") {
            let n = (l.get("width").and_then(|x| x.as_u64()).unwrap_or(0) * l.get("height").and_then(|x| x.as_u64()).unwrap_or(0)) as usize;
            if let Ok(d) = tmx::decode_layer(l, n) {
                gids.extend(d);
            }
        }
        for o in l.get("objects").and_then(|x| x.as_array()).into_iter().flatten() {
            if let Some(g) = o.get("gid").and_then(|x| x.as_u64()) {
                gids.insert(g as u32);
            }
        }
    }
    for g in gids {
        let g = g & !tmx::FLIP_ALL;
        if g == 0 {
            continue;
        }
        if let Some((first, ts)) = sets.iter().find(|(f, _)| *f <= g) {
            if let Some(name) = ts.tiles.get((g - first) as usize).and_then(|t| t.props.get("object")).and_then(|x| x.as_str()) {
                out.insert(name.to_string());
            }
        }
    }
}

pub fn build(a: &Assets, cat: &Catalog) -> Availability {
    let mut obtainable = HashSet::new();
    let mut seen = HashSet::new();

    // crafting
    each_json(a, |p| p.ends_with(".recipe"), |_, v| {
        if let Some(o) = v.get("output") {
            for k in ["item", "name"] {
                if let Some(n) = o.get(k).and_then(|x| x.as_str()) {
                    obtainable.insert(n.to_string());
                }
            }
        }
    });
    // loot, monster drops, merchant stock, quest rewards, starting gear
    each_json(
        a,
        |p| p.ends_with(".treasurepools") || p.ends_with(".questtemplate") || p == "/player.config" || p == "/npcs/merchantpools.config",
        |_, v| strings(v, &mut obtainable),
    );
    each_json(a, |p| p.ends_with(".species"), |_, v| {
        if let Some(b) = v.get("defaultBlueprints") {
            strings(b, &mut obtainable);
        }
    });
    // what mining blocks and collecting liquids gives
    each_json(a, |p| p.ends_with(".material") || p.ends_with(".matmod") || p.ends_with(".liquid"), |_, v| {
        if let Some(d) = v.get("itemDrop").and_then(|x| x.as_str()) {
            obtainable.insert(d.to_string());
        }
    });
    // shop objects (Frögg Furnishing, Treasured Trophies, …) and crafting station upgrades
    each_json(a, |p| p.ends_with(".object"), |_, v| {
        for k in ["storeInventory", "interactData"] {
            if let Some(x) = v.get(k) {
                strings(x, &mut obtainable);
            }
        }
        for st in v.get("upgradeStages").and_then(|x| x.as_array()).into_iter().flatten() {
            if let Some(x) = st.get("itemSpawnParameters") {
                strings(x, &mut obtainable);
            }
        }
    });

    // biome placeables (plants, rocks, ruins, …) and the dungeons regular planets get
    let mut planet_strings = HashSet::new();
    each_json(
        a,
        |p| p.ends_with(".biome") || p.starts_with("/terrestrial_worlds") || p.starts_with("/celestial") || p.starts_with("/plants/"),
        |_, v| strings(v, &mut planet_strings),
    );
    let instance: HashSet<String> = a
        .json("/instance_worlds.config")
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|m| {
            m.iter()
                .flat_map(|(k, w)| [Some(k.clone()), w.get("dungeonWorld").and_then(|x| x.as_str()).map(String::from)])
                .flatten()
                .collect()
        })
        .unwrap_or_default();

    let props: HashMap<&str, &crate::catalog::TilesetInfo> = cat.tilesets.iter().map(|t| (t.path.as_str(), t)).collect();
    for p in a.list(|p| p.ends_with(".dungeon")) {
        if !a.in_vanilla(&p) {
            continue;
        }
        let Ok(v) = a.json(&p) else { continue };
        let name = v.pointer("/metadata/name").and_then(|x| x.as_str()).unwrap_or("");
        if mission_dungeon(name, &instance, &planet_strings) {
            continue;
        }
        for t in v.get("tiles").and_then(|x| x.as_array()).into_iter().flatten() {
            for b in t.get("brush").and_then(|x| x.as_array()).into_iter().flatten() {
                if b.get(0).and_then(|x| x.as_str()) == Some("object") {
                    if let Some(n) = b.get(1).and_then(|x| x.as_str()) {
                        seen.insert(n.to_string());
                    }
                }
            }
        }
        for part in v.get("parts").and_then(|x| x.as_array()).into_iter().flatten() {
            let Some(def) = part.get("def").and_then(|x| x.as_array()) else { continue };
            if def.first().and_then(|x| x.as_str()) != Some("tmx") {
                continue;
            }
            let files: Vec<&str> = match def.get(1) {
                Some(Value::String(s)) => vec![s.as_str()],
                Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).collect(),
                _ => vec![],
            };
            for f in files {
                map_objects(a, &relative_to(&p, f), &props, &mut seen);
            }
        }
    }
    seen.extend(planet_strings);
    Availability { obtainable, seen }
}
