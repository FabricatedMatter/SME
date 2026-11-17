//! Treasure pools (loot), the item index used for autocompletion, and the "game index"
//! (names of vanilla pools, dungeons, biomes, npcs, monsters, stagehands).

use crate::assets::{relative_to, Assets};
use anyhow::{anyhow, bail, Result};
use rand::Rng;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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
    /// Built with random parts / stats when it drops (random-build weapons and shields).
    pub random: bool,
    /// Armor and clothes: the folder of the set this piece belongs to.
    pub set: Option<String>,
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

/// An icon made of several drawables (`[{image, position}]`) as `a@x,y|b@x,y` (see Thumb.svelte).
fn layered_icon(a: &Assets, frames: &mut crate::frames::Frames, item_path: &str, arr: &[Value]) -> Option<String> {
    let layers: Vec<String> = arr
        .iter()
        .filter_map(|d| {
            let img = d.get("image")?.as_str()?;
            let pos = d.get("position").and_then(|p| p.as_array());
            let c = |i: usize| pos.and_then(|p| p.get(i)).and_then(|x| x.as_f64()).unwrap_or(0.0);
            Some(format!("{}@{},{}", icon_ref(a, frames, item_path, img), c(0), c(1)))
        })
        .collect();
    match layers.len() {
        0 => None,
        1 => layers[0].split_once('@').map(|(r, _)| r.to_string()),
        _ => Some(layers.join("|")),
    }
}

/// Width and height from a PNG header.
fn png_size(a: &Assets, path: &str) -> Option<(f64, f64)> {
    let b = a.read(path).ok()?;
    if b.len() < 24 || &b[1..4] != b"PNG" {
        return None;
    }
    let u = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as f64;
    Some((u(16), u(20)))
}

/// Icons the game only puts together when the item is generated: random-build weapons
/// (variant 1 of each part, guns laid out butt → barrel like `buildweapon.lua`), random
/// shields (`buildshield.lua`), and codexes (their `icon`).
fn built_icon(a: &Assets, frames: &mut crate::frames::Frames, item_path: &str, v: &Value) -> Option<String> {
    if let Some(icon) = v.get("icon").and_then(|x| x.as_str()) {
        return Some(icon_ref(a, frames, item_path, icon));
    }
    let bc = v.get("builderConfig")?.get(0)?;
    let parts = bc.get("animationParts")?.as_object()?;
    let part = |k: &str| -> Option<String> {
        let p = parts.get(k)?;
        let s = p.get("path").and_then(|x| x.as_str()).or(p.as_str())?;
        Some(relative_to(item_path, &s.replace("<variant>", "1")))
    };
    let names = |k: &str| -> Vec<String> {
        bc.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
    };
    if v.get("builder").and_then(|x| x.as_str()).is_some_and(|b| b.ends_with("buildshield.lua")) {
        return Some(icon_ref(a, frames, item_path, &format!("{}:nearidle", part("shield")?)));
    }
    let mut x_of = HashMap::new();
    let mut x = 0.0;
    for g in names("gunParts") {
        let (w, _) = png_size(a, &part(&g)?)?;
        x_of.insert(g, x + w / 2.0);
        x += w;
    }
    let layers: Vec<(String, f64)> = names("iconDrawables")
        .into_iter()
        .filter_map(|n| {
            let p = part(&n).filter(|p| a.exists(p))?;
            Some((icon_ref(a, frames, item_path, &p), x_of.get(&n).copied().unwrap_or(0.0)))
        })
        .collect();
    match layers.len() {
        0 => None,
        1 if layers[0].1 == 0.0 => Some(layers[0].0.clone()),
        _ => Some(layers.iter().map(|(r, x)| format!("{r}@{x},0")).collect::<Vec<_>>().join("|")),
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
            Some(Value::Array(arr)) => layered_icon(a, &mut frames, &p, arr),
            _ => built_icon(a, &mut frames, &p, &v),
        };
        out.insert(
            name.clone(),
            ItemInfo {
                name,
                shortdescription: strip_markup(v.get("shortdescription").and_then(|x| x.as_str()).unwrap_or("")),
                category: v.get("category").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                rarity: v.get("rarity").and_then(|x| x.as_str()).unwrap_or("").to_lowercase(),
                random: v.get("inventoryIcon").is_none() && v.get("builderConfig").is_some(),
                set: matches!(kind.as_str(), "head" | "chest" | "legs" | "back").then(|| p.rsplit_once('/').map(|x| x.0.to_string())).flatten(),
                kind,
                icon,
            },
        );
    }
    out.into_values().collect()
}

/// Where each microdungeon can turn up: dungeon name → [{planet, layer ("surface" or
/// "underground"), via ("main" | "secondary" | "minibiome"), biome, distribution}], following
/// terrestrial_worlds.config: planet type → layers → regions (+ their sub-regions, i.e.
/// minibiomes) → biomes (all the threat steps up to the planet's highest threat) → the biome's
/// microdungeon placeables. Surface placeables of a surface-layer biome are "surface";
/// everything else is in caves.
fn micro_spawns(a: &Assets, biomes: &[Value]) -> Value {
    let Ok(tw) = a.json("/terrestrial_worlds.config") else { return json!({}) };
    let strs = |v: Option<&Value>| -> Vec<String> { v.and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() };
    let regions = tw.get("regionTypes").and_then(|x| x.as_object()).cloned().unwrap_or_default();
    let region_defaults = tw.get("regionDefaults").cloned().unwrap_or(json!({}));
    // region → biome names usable at threat ≤ max
    let region_biomes = |rt: &str, max: f64| -> Vec<String> {
        let Some(r) = regions.get(rt) else { return vec![] };
        let mut out = Vec::new();
        for step in r.get("biome").and_then(|x| x.as_array()).into_iter().flatten() {
            if step.get(0).and_then(|x| x.as_f64()).unwrap_or(0.0) <= max {
                out.extend(strs(step.get(1)));
            }
        }
        out
    };
    let sub_regions = |rt: &str| -> Vec<String> { strs(regions.get(rt).and_then(|r| r.get("subRegion")).or(region_defaults.get("subRegion"))) };
    // biome name → its microdungeon entries (surface, underground)
    let mut by_biome: HashMap<String, (&Value, &Value)> = HashMap::new();
    for b in biomes {
        if let Some(n) = b.get("name").and_then(|x| x.as_str()) {
            by_biome.insert(n.to_string(), (&b["surfaceMicro"], &b["undergroundMicro"]));
        }
    }
    let empty = json!({});
    let default_layers = tw.pointer("/planetDefaults/layers").and_then(|x| x.as_object()).cloned().unwrap_or_default();
    let mut out: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut seen = HashSet::new();
    for (planet, pt) in tw.get("planetTypes").and_then(|x| x.as_object()).into_iter().flatten() {
        let max = pt.pointer("/threatRange/1").and_then(|x| x.as_f64()).unwrap_or(10.0);
        let layers = pt.get("layers").and_then(|x| x.as_object()).cloned().unwrap_or_default();
        let names: BTreeSet<&String> = default_layers.keys().chain(layers.keys()).collect();
        for layer in names {
            let mut l = default_layers.get(layer).and_then(|x| x.as_object()).cloned().unwrap_or_default();
            for (k, v) in layers.get(layer).and_then(|x| x.as_object()).unwrap_or(empty.as_object().unwrap()) {
                l.insert(k.clone(), v.clone());
            }
            if matches!(layer.as_str(), "space" | "atmosphere") {
                continue;
            }
            let mut regs: Vec<(String, &str)> = strs(l.get("primaryRegion")).into_iter().map(|r| (r, "main")).collect();
            regs.extend(strs(l.get("secondaryRegions")).into_iter().map(|r| (r, "secondary")));
            let subs: Vec<(String, &str)> = regs.iter().flat_map(|(r, _)| sub_regions(r).into_iter().map(|s| (s, "minibiome"))).collect();
            regs.extend(subs);
            for (rt, via) in regs {
                for biome in region_biomes(&rt, max) {
                    let Some((surf, under)) = by_biome.get(&biome) else { continue };
                    for (entries, placed_on_surface) in [(surf, true), (under, false)] {
                        for e in entries.as_array().into_iter().flatten() {
                            let where_ = if layer == "surface" && placed_on_surface { "surface" } else { "underground" };
                            let dist = e.get("distribution").and_then(|x| x.as_str()).unwrap_or("");
                            for d in strs(e.get("microdungeons")) {
                                if seen.insert((d.clone(), planet.clone(), where_, via, biome.clone())) {
                                    out.entry(d).or_default().push(json!({ "planet": planet, "layer": where_, "via": via, "biome": biome, "distribution": dist }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    json!(out)
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

fn entry_item(e: &Value) -> Option<&str> {
    match e.get("item")? {
        Value::String(s) => Some(s.as_str()),
        Value::Array(a) => a.first().and_then(|x| x.as_str()),
        Value::Object(o) => o.get("name").or(o.get("item")).and_then(|x| x.as_str()),
        _ => None,
    }
}

/// A table's entries in all its versions: (item or nested table, weight; fill counts as 1).
fn entries(cfg_levels: &Value) -> Vec<(&Value, f64)> {
    let mut out = Vec::new();
    for l in cfg_levels.as_array().into_iter().flatten() {
        let Some(cfg) = l.get(1) else { continue };
        for e in cfg.get("pool").and_then(|x| x.as_array()).into_iter().flatten() {
            out.push((e, e.get("weight").and_then(|w| w.as_f64()).unwrap_or(1.0)));
        }
        for e in cfg.get("fill").and_then(|x| x.as_array()).into_iter().flatten() {
            out.push((e, 1.0));
        }
    }
    out
}

/// Item names that show what a loot table gives, best first. The editor uses the first one with
/// an icon. Its own entries come before nested tables' (so chests that all nest the same basic
/// table still look different), and among them items few other tables have, then by weight;
/// pixels last. `spread[item]` = how many tables list it directly.
pub fn sample_items(name: &str, all: &Map<String, Value>, spread: &HashMap<String, usize>) -> Vec<String> {
    let mut cands: Vec<(bool, u32, usize, f64, String)> = Vec::new(); // money, depth, spread, -weight, name
    let mut seen = HashSet::new();
    let mut queue = vec![(name.to_string(), 0u32)];
    while let Some((t, depth)) = queue.pop() {
        if depth > 4 || !seen.insert(t.clone()) {
            continue;
        }
        let Some(levels) = all.get(&t) else { continue };
        for (e, w) in entries(levels) {
            if let Some(n) = entry_item(e) {
                cands.push((n == "money", depth, spread.get(n).copied().unwrap_or(0), -w, n.to_string()));
            } else if let Some(p) = e.get("pool").and_then(|x| x.as_str()) {
                queue.insert(0, (p.to_string(), depth + 1));
            }
        }
    }
    cands.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)).then(a.3.total_cmp(&b.3)));
    let mut out: Vec<String> = Vec::new();
    for c in cands {
        if !out.contains(&c.4) {
            out.push(c.4);
            if out.len() >= 6 {
                break;
            }
        }
    }
    out
}

pub fn game_index(a: &Assets) -> Value {
    let mut pools = Map::new();
    let mut contents = Map::new();
    for p in a.list(|p| p.ends_with(".treasurepools")) {
        if let Ok(Value::Object(m)) = a.json(&p) {
            for (k, v) in m {
                pools.insert(k.clone(), json!({ "file": p, "vanilla": a.in_vanilla(&p) }));
                contents.insert(k, v);
            }
        }
    }
    let mut spread: HashMap<String, usize> = HashMap::new();
    for levels in contents.values() {
        let names: HashSet<&str> = entries(levels).iter().filter_map(|(e, _)| entry_item(e)).collect();
        for n in names {
            *spread.entry(n.to_string()).or_default() += 1;
        }
    }
    for (k, v) in pools.iter_mut() {
        v["items"] = json!(sample_items(k, &contents, &spread));
    }
    let mut dungeons = Map::new();
    for p in a.list(|p| p.ends_with(".dungeon")) {
        if let Ok(v) = a.json(&p) {
            if let Some(n) = v.pointer("/metadata/name").and_then(|x| x.as_str()) {
                let parts = v.get("parts").and_then(|x| x.as_array()).map_or(0, |x| x.len());
                dungeons.insert(n.to_string(), json!({ "file": p, "vanilla": a.in_vanilla(&p), "parts": parts }));
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
    let micro_spawns = micro_spawns(a, &biomes);
    let distributions = a.json("/biomes/distributions.config").unwrap_or(Value::Null);
    let species = names_in(a, ".species", "kind");
    json!({
        "pools": pools,
        "dungeons": dungeons,
        "biomes": biomes,
        "microSpawns": micro_spawns,
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
