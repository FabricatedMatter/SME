//! Treasure pools (loot), the item index used for autocompletion, and the "game index"
//! (names of vanilla pools, dungeons, biomes, npcs, monsters, stagehands).

use crate::assets::{relative_to, Assets};
use anyhow::{anyhow, bail, Result};
use rand::Rng;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};

const ITEM_EXTS: &[&str] = &[
    ".item", ".activeitem", ".consumable", ".object", ".matitem", ".liqitem", ".miningtool", ".flashlight", ".wieldeditem",
    ".harvestingtool", ".beamaxe", ".tillingtool", ".painttool", ".wiretool", ".inspectiontool", ".instrument", ".thrownitem",
    ".unlock", ".augment", ".currency", ".head", ".chest", ".legs", ".back", ".codex", ".throwitem", ".gun", ".sword", ".shield",
];

#[derive(Serialize, Clone)]
pub struct ItemInfo {
    pub name: String,
    pub shortdescription: String,
    pub category: String,
    pub rarity: String,
    pub kind: String,
    pub icon: Option<String>,
}

fn strip_markup(s: &str) -> String {
    // remove ^color; codes
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() {
        if c == '^' {
            skip = true;
            continue;
        }
        if skip {
            if c == ';' {
                skip = false;
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// An icon spec (`image.png:frame?directives`) as `path` or `path#x,y,w,h` (one frame of a sheet).
fn icon_ref(a: &Assets, frames: &mut crate::frames::Frames, item_path: &str, spec: &str) -> String {
    let no_dir = spec.split('?').next().unwrap_or(spec);
    let (img, frame) = match no_dir.split_once(':') {
        Some((i, f)) => (i, Some(f)),
        None => (no_dir, None),
    };
    let path = relative_to(item_path, img);
    match frames.resolve(a, &path, frame) {
        Some([x, y, w, h]) if w > 0 && h > 0 => format!("{path}#{x},{y},{w},{h}"),
        _ => path,
    }
}

pub fn build_items(a: &Assets) -> Vec<ItemInfo> {
    let mut out = BTreeMap::new();
    let mut frames = crate::frames::Frames::default();
    for p in a.list(|p| ITEM_EXTS.iter().any(|e| p.ends_with(e))) {
        let Ok(v) = a.json(&p) else { continue };
        let kind = p.rsplit('.').next().unwrap_or("").to_string();
        let name_key = match kind.as_str() {
            "object" => "objectName",
            "codex" => "id",
            _ => "itemName",
        };
        let mut name = v.get(name_key).and_then(|x| x.as_str()).unwrap_or("").to_string();
        if kind == "codex" && !name.is_empty() {
            name = format!("{name}-codex");
        }
        if name.is_empty() {
            continue;
        }
        let icon = match v.get("inventoryIcon") {
            Some(Value::String(s)) => Some(icon_ref(a, &mut frames, &p, s)),
            Some(Value::Array(arr)) => arr.first().and_then(|x| x.get("image")).and_then(|x| x.as_str()).map(|s| icon_ref(a, &mut frames, &p, s)),
            _ => None,
        };
        out.insert(
            name.clone(),
            ItemInfo {
                name,
                shortdescription: strip_markup(v.get("shortdescription").and_then(|x| x.as_str()).unwrap_or("")),
                category: v.get("category").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                rarity: v.get("rarity").and_then(|x| x.as_str()).unwrap_or("").to_lowercase(),
                kind,
                icon,
            },
        );
    }
    out.into_values().collect()
}

fn names_in(a: &Assets, ext: &str, key: &str) -> Vec<String> {
    let mut v: Vec<String> = a
        .list(|p| p.ends_with(ext))
        .iter()
        .filter_map(|p| a.json(p).ok()?.get(key)?.as_str().map(String::from))
        .collect();
    v.sort();
    v.dedup();
    v
}

pub fn game_index(a: &Assets) -> Value {
    let mut pools = Map::new();
    for p in a.list(|p| p.ends_with(".treasurepools")) {
        if let Ok(Value::Object(m)) = a.json(&p) {
            for k in m.keys() {
                pools.insert(k.clone(), json!({ "file": p, "vanilla": a.in_vanilla(&p) }));
            }
        }
    }
    let mut dungeons = Map::new();
    for p in a.list(|p| p.ends_with(".dungeon")) {
        if let Ok(v) = a.json(&p) {
            if let Some(n) = v.pointer("/metadata/name").and_then(|x| x.as_str()) {
                dungeons.insert(n.to_string(), json!({ "file": p, "vanilla": a.in_vanilla(&p) }));
            }
        }
    }
    let mut biomes = Vec::new();
    for p in a.list(|p| p.starts_with("/biomes/") && p.ends_with(".biome")) {
        let Ok(v) = a.json(&p) else { continue };
        let micro = |k: &str| -> Value {
            let items = v.pointer(&format!("/{k}/items")).and_then(|x| x.as_array()).cloned().unwrap_or_default();
            Value::Array(
                items
                    .iter()
                    .filter(|i| i.get("type").and_then(|x| x.as_str()) == Some("microdungeon"))
                    .cloned()
                    .collect(),
            )
        };
        biomes.push(json!({
            "path": p,
            "name": v.get("name"),
            "friendlyName": v.get("friendlyName"),
            "surface": v.pointer("/surfacePlaceables/items").is_some(),
            "underground": v.pointer("/undergroundPlaceables/items").is_some(),
            "surfaceMicro": micro("surfacePlaceables"),
            "undergroundMicro": micro("undergroundPlaceables"),
        }));
    }
    let distributions = a.json("/biomes/distributions.config").unwrap_or(Value::Null);
    let species = names_in(a, ".species", "kind");
    json!({
        "pools": pools,
        "dungeons": dungeons,
        "biomes": biomes,
        "distributions": distributions,
        "npcTypes": names_in(a, ".npctype", "type"),
        "monsterTypes": names_in(a, ".monstertype", "type"),
        "stagehands": names_in(a, ".stagehand", "type"),
        "species": species,
    })
}

// ───────────────────────── loot simulation ─────────────────────────

/// (name, count, parameters?) from any item descriptor form.
fn descriptor(v: &Value) -> Option<(String, u64)> {
    match v {
        Value::String(s) => Some((s.clone(), 1)),
        Value::Array(a) => Some((a.first()?.as_str()?.to_string(), a.get(1).and_then(|x| x.as_u64()).unwrap_or(1))),
        Value::Object(o) => Some((
            o.get("name").or(o.get("item")).and_then(|x| x.as_str())?.to_string(),
            o.get("count").and_then(|x| x.as_u64()).unwrap_or(1),
        )),
        _ => None,
    }
}

fn level_entry<'a>(pool: &'a Value, level: f64) -> Option<&'a Value> {
    // like the game's ParametricTable: the highest start level ≤ level, else the lowest one
    let mut v: Vec<(f64, &Value)> = pool
        .as_array()?
        .iter()
        .filter_map(|e| Some((e.get(0).and_then(|x| x.as_f64()).unwrap_or(0.0), e.get(1)?)))
        .collect();
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    v.iter().rev().find(|(l, _)| *l <= level).or(v.first()).map(|x| x.1)
}

fn pick<'a, T>(rng: &mut impl Rng, items: &'a [(f64, T)]) -> Option<&'a T> {
    let total: f64 = items.iter().map(|x| x.0).sum();
    if total <= 0.0 {
        return None;
    }
    let mut r = rng.random::<f64>() * total;
    for (w, t) in items {
        if r < *w {
            return Some(t);
        }
        r -= w;
    }
    items.last().map(|x| &x.1)
}

fn roll(lookup: &dyn Fn(&str) -> Option<Value>, name: &str, level: f64, rng: &mut impl Rng, visited: &mut Vec<String>, out: &mut Vec<(String, u64)>) -> Result<()> {
    if visited.iter().any(|v| v == name) {
        bail!("loop in treasure pools: {} → {name}", visited.join(" → "));
    }
    let pool = lookup(name).ok_or_else(|| anyhow!("unknown treasure pool '{name}'"))?;
    let cfg = level_entry(&pool, level).ok_or_else(|| anyhow!("pool '{name}' has no level entries"))?.clone();
    visited.push(name.to_string());
    let allow_dup = cfg.get("allowDuplication").and_then(|x| x.as_bool()).unwrap_or(true);
    let mut seen: HashSet<String> = HashSet::new();
    let mut add = |items: Vec<(String, u64)>, out: &mut Vec<(String, u64)>| {
        for it in items {
            if allow_dup || seen.insert(it.0.clone()) {
                out.push(it);
            }
        }
    };
    for f in cfg.get("fill").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
        if let Some(p) = f.get("pool").and_then(|x| x.as_str()) {
            let mut sub = Vec::new();
            roll(lookup, p, level, rng, visited, &mut sub)?;
            add(sub, out);
        } else if let Some(d) = f.get("item").and_then(descriptor) {
            add(vec![d], out);
        }
    }
    let entries: Vec<(f64, Value)> = cfg
        .get("pool")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().map(|e| (e.get("weight").and_then(|x| x.as_f64()).unwrap_or(0.0), e.clone())).collect())
        .unwrap_or_default();
    if !entries.is_empty() {
        let rounds = match cfg.get("poolRounds") {
            None => 1,
            Some(Value::Number(n)) => n.as_f64().unwrap_or(1.0) as i64,
            Some(Value::Array(a)) => {
                let opts: Vec<(f64, i64)> =
                    a.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_i64()?))).collect();
                pick(rng, &opts).copied().unwrap_or(1)
            }
            _ => 1,
        };
        for _ in 0..rounds.max(0) {
            if let Some(e) = pick(rng, &entries) {
                if let Some(p) = e.get("pool").and_then(|x| x.as_str()) {
                    let mut sub = Vec::new();
                    roll(lookup, p, level, rng, visited, &mut sub)?;
                    add(sub, out);
                } else if let Some(d) = e.get("item").and_then(descriptor) {
                    add(vec![d], out);
                }
            }
        }
    }
    visited.pop();
    Ok(())
}

/// Rolls `pool` `rolls` times. `project_pools` (pool name → levels) override vanilla pools.
pub fn simulate(a: &Assets, project_pools: &Value, pool: &str, level: f64, rolls: u32) -> Result<Vec<Vec<(String, u64)>>> {
    let mut all: Map<String, Value> = Map::new();
    for p in a.list(|p| p.ends_with(".treasurepools")) {
        if let Ok(Value::Object(m)) = a.json(&p) {
            all.extend(m);
        }
    }
    if let Value::Object(m) = project_pools {
        all.extend(m.clone());
    }
    let lookup = |n: &str| all.get(n).cloned();
    let mut rng = rand::rng();
    let mut out = Vec::new();
    for _ in 0..rolls.min(500) {
        let mut items = Vec::new();
        roll(&lookup, pool, level, &mut rng, &mut Vec::new(), &mut items)?;
        // merge stacks of the same item for readability
        let mut merged: Vec<(String, u64)> = Vec::new();
        for (n, c) in items {
            match merged.iter_mut().find(|m| m.0 == n) {
                Some(m) => m.1 += c,
                None => merged.push((n, c)),
            }
        }
        out.push(merged);
    }
    Ok(out)
}

/// Detects reference cycles among pools (game throws on these).
pub fn find_cycles(pools: &Map<String, Value>) -> Vec<String> {
    fn refs(v: &Value) -> Vec<String> {
        let mut out = Vec::new();
        for lvl in v.as_array().cloned().unwrap_or_default() {
            if let Some(cfg) = lvl.get(1) {
                for k in ["fill", "pool"] {
                    for e in cfg.get(k).and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                        if let Some(p) = e.get("pool").and_then(|x| x.as_str()) {
                            out.push(p.to_string());
                        }
                    }
                }
            }
        }
        out
    }
    let mut found = Vec::new();
    fn dfs(n: &str, pools: &Map<String, Value>, stack: &mut Vec<String>, found: &mut Vec<String>, done: &mut HashSet<String>) {
        if let Some(i) = stack.iter().position(|s| s == n) {
            found.push(format!("{} → {n}", stack[i..].join(" → ")));
            return;
        }
        if done.contains(n) {
            return;
        }
        let Some(v) = pools.get(n) else { return };
        stack.push(n.to_string());
        for r in refs(v) {
            dfs(&r, pools, stack, found, done);
        }
        stack.pop();
        done.insert(n.to_string());
    }
    let mut done = HashSet::new();
    for k in pools.keys() {
        dfs(k, pools, &mut Vec::new(), &mut found, &mut done);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rolls_and_cycles() {
        let pools = json!({
            "a": [[0, {"fill": [{"item": ["gold", 2]}], "pool": [{"weight": 1, "pool": "b"}], "poolRounds": 2}]],
            "b": [[0, {"pool": [{"weight": 1, "item": "apple"}]}], [5, {"pool": [{"weight": 1, "item": "pear"}]}]]
        });
        let lookup = |n: &str| pools.get(n).cloned();
        let mut rng = rand::rng();
        let mut out = Vec::new();
        roll(&lookup, "a", 1.0, &mut rng, &mut vec![], &mut out).unwrap();
        assert_eq!(out, vec![("gold".into(), 2), ("apple".into(), 1), ("apple".into(), 1)]);
        let mut out = Vec::new();
        roll(&lookup, "b", 6.0, &mut rng, &mut vec![], &mut out).unwrap();
        assert_eq!(out[0].0, "pear");
        let cyc = json!({"x": [[0, {"pool": [{"weight": 1, "pool": "y"}]}]], "y": [[0, {"fill": [{"pool": "x"}]}]]});
        assert!(!find_cycles(cyc.as_object().unwrap()).is_empty());
    }
}
