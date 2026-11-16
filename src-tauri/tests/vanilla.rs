//! Round-trip every vanilla dungeon map through import → export and check that the game
//! would build exactly the same dungeon from it. Skipped when Starbound isn't installed.

use sme_lib::assets::{find_install, Assets};
use sme_lib::catalog;
use sme_lib::tmx;
use std::collections::HashMap;

#[test]
fn vanilla_maps_roundtrip() {
    let Some(dir) = find_install(None) else {
        eprintln!("Starbound not installed; skipping");
        return;
    };
    let a = Assets::open(&dir).unwrap();
    let cat = catalog::build(&a);
    let counts: HashMap<String, u32> = cat.tilesets.iter().map(|t| (t.path.clone(), t.tilecount)).collect();
    let props: HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>> =
        cat.tilesets.iter().map(|t| (t.path.clone(), t.tiles.iter().map(|x| x.props.clone()).collect())).collect();
    let tc = |p: &str| counts.get(p).copied();
    let tp = |p: &str, i: u32| props.get(p).and_then(|v| v.get(i as usize)).cloned();
    let maps = a.list(|p| p.starts_with("/dungeons/") && p.ends_with(".json"));
    let (mut ok, mut skipped) = (0, 0);
    let mut failures = Vec::new();
    for p in &maps {
        let v = a.json(p).unwrap();
        if v.get("tilesets").is_none() {
            skipped += 1;
            continue;
        }
        let imported = match tmx::import(&v, &tc) {
            Ok(i) => i,
            Err(e) => {
                failures.push(format!("{p}: import: {e}"));
                continue;
            }
        };
        let out = tmx::export(&imported.doc, &tc, "../../../").unwrap();
        let before = tmx::semantic(&v, &tp, &tc).unwrap();
        let after = tmx::semantic(&out, &tp, &tc).unwrap();
        if before != after {
            let diff: Vec<_> = before.iter().filter(|x| !after.contains(x)).take(3).cloned().collect();
            let diff2: Vec<_> = after.iter().filter(|x| !before.contains(x)).take(3).cloned().collect();
            failures.push(format!("{p}: {} vs {} entries; missing {diff:?}; extra {diff2:?}", before.len(), after.len()));
        } else {
            ok += 1;
        }
    }
    eprintln!("{ok} maps identical, {skipped} skipped, {} failed", failures.len());
    for f in failures.iter().take(10) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty());
    assert!(ok > 1000);
}

#[test]
fn vanilla_data_parses() {
    let Some(dir) = find_install(None) else { return };
    let a = Assets::open(&dir).unwrap();
    for ext in [".dungeon", ".treasurepools", ".biome"] {
        let files = a.list(|p| p.ends_with(ext));
        let bad: Vec<_> = files.iter().filter(|p| a.json(p).is_err()).collect();
        eprintln!("{ext}: {} files, {} unparseable {:?}", files.len(), bad.len(), bad.iter().take(3).collect::<Vec<_>>());
        assert!(bad.is_empty());
    }
    let cat = catalog::build(&a);
    eprintln!("{} tilesets, {} materials, {} mods, {} liquids, {} templates", cat.tilesets.len(), cat.materials.len(), cat.mods.len(), cat.liquids.len(), cat.templates.len());
    assert!(cat.materials.contains_key("cobblestone"));
    let objs = catalog::build_objects(&a);
    eprintln!("{} objects, {} containers", objs.len(), objs.values().filter(|o| o.container).count());
    assert!(objs.values().any(|o| o.container));
}
