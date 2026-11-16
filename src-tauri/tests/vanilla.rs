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

/// Unpacks every subscribed Starbound workshop mod and round-trips its dungeon maps.
/// Skipped when there are none.
#[test]
fn workshop_paks_unpack_and_roundtrip() {
    let Some(dir) = find_install(None) else { return };
    let workshop = dir.join("../../workshop/content/211820");
    let Ok(entries) = std::fs::read_dir(&workshop) else { return };
    let a = Assets::open(&dir).unwrap();
    let cat = catalog::build(&a);
    let counts: HashMap<String, u32> = cat.tilesets.iter().map(|t| (t.path.clone(), t.tilecount)).collect();
    let props: HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>> =
        cat.tilesets.iter().map(|t| (t.path.clone(), t.tiles.iter().map(|x| x.props.clone()).collect())).collect();
    let tc = |p: &str| counts.get(p).copied();
    let tp = |p: &str, i: u32| props.get(p).and_then(|v| v.get(i as usize)).cloned();
    for e in entries.flatten() {
        let pak_path = e.path().join("contents.pak");
        if !pak_path.is_file() {
            continue;
        }
        let pak = sme_lib::pak::Pak::open(&pak_path).unwrap();
        let out = std::env::temp_dir().join(format!("sme-ws-{}-{}", std::process::id(), sme_lib::pak::suggested_dir_name(&pak)));
        let _ = std::fs::remove_dir_all(&out);
        let n = sme_lib::pak::unpack(&pak, &out).unwrap();
        assert!(out.join("_metadata").is_file());
        let info = sme_lib::project::scan(&out).unwrap();
        let mut maps = 0;
        for m in &info.maps {
            let v = sme_lib::sbjson::parse(&std::fs::read(out.join(m)).unwrap()).unwrap();
            let imp = tmx::import(&v, &tc).unwrap_or_else(|e| panic!("{m}: {e}"));
            let back = tmx::export(&imp.doc, &tc, "../../../").unwrap();
            assert_eq!(tmx::semantic(&v, &tp, &tc).unwrap(), tmx::semantic(&back, &tp, &tc).unwrap(), "{m}");
            if !imp.warnings.is_empty() {
                eprintln!("  {m}: {:?}", imp.warnings);
            }
            maps += 1;
        }
        eprintln!("{}: {n} files, {maps} maps round-trip, {} patches, errors {:?}", pak_path.display(), info.patches.len(), info.errors);
        std::fs::remove_dir_all(&out).unwrap();
    }
}
