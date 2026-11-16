// End-to-end smoke test, run with `sme --selftest <mod folder>`: drives the editor's own
// actions (new dungeon, painting, chest + loot pool, spawn rule, save, check) and writes a
// report to `sme-selftest.json` in the mod folder.

import { api } from "./api";
import { editor, joinRel, partFiles } from "./editor.svelte";
import { game } from "./game.svelte";
import { GROUPS, MapDoc } from "./mapdoc.svelte";
import { newRule } from "./spawn";

export async function selftest() {
  const log: string[] = [];
  const ok = (cond: any, msg: string) => log.push(`${cond ? "PASS" : "FAIL"} ${msg}`);
  try {
    const d = editor.newDungeon("smeselftest", { width: 30, height: 20, anchors: true });
    const mpath = joinRel(d, partFiles(editor.files[d].parts[0])[0]);
    await editor.openMap(mpath, d);
    const md = editor.maps[mpath];
    ok(!!md, "new dungeon map opened");

    const cobble = game.brushes.find((b) => b.kind === "material" && b.name === "cobblestone")!;
    const brick = game.brushes.find((b) => b.kind === "material" && b.name === "woodenwindow1" ) ?? game.brushes.find((b) => b.kind === "material" && b.name === "darkwood")!;
    ok(!!cobble && !!brick, "found material brushes");
    md.begin();
    md.fillRect("front", { x: 5, y: 15, w: 20, h: 1 }, md.intern(cobble.ref));
    md.fillRect("front", { x: 5, y: 8, w: 1, h: 7 }, md.intern(cobble.ref));
    md.fillRect("front", { x: 24, y: 8, w: 1, h: 7 }, md.intern(cobble.ref));
    md.fillRect("back", { x: 6, y: 8, w: 18, h: 7 }, md.intern(brick.ref));
    md.commit();
    ok(md.get("front", 10, 15) === md.intern(cobble.ref), "painted floor");
    md.undo();
    ok(md.get("front", 10, 15) === 0, "undo removes the floor");
    md.redo();
    ok(md.get("front", 10, 15) !== 0, "redo restores the floor");

    const chest = game.brushes.find((b) => b.kind === "object" && game.objects[b.name]?.container && b.name.includes("chest"))!;
    ok(!!chest, `found a container brush (${chest?.name})`);
    const pos = MapDoc.placeAt(chest.props, 12, 14);
    const o = md.addObject(GROUPS.objects, { tile: md.intern(chest.ref), flipX: false, x: pos.x, y: pos.y, width: 16, height: 16, name: "", type: "", properties: {}, visible: true });
    const a = md.tileAnchor(md.findObject(o.id)!.group, o);
    ok(a.x === 12 && a.y === 14, `chest anchored at the clicked tile (${a.x},${a.y})`);

    const lootPath = "treasure/smeselftest.treasurepools";
    editor.setFile(lootPath, { smeselftestTreasure: [[0, { fill: [{ item: ["money", 25] }], pool: [{ weight: 1, item: ["torch", 3] }], poolRounds: 1 }]] });
    o.properties.parameters = JSON.stringify({ treasurePools: ["smeselftestTreasure"] });
    md.touchObjects();

    const rule = newRule("smeselftest");
    rule.biomes = ["/biomes/surface/forest.biome", "/biomes/surface/desert.biome"];
    editor.spawnRules.push(rule);
    editor.spawnDirty = true;

    const saved = await editor.saveAll();
    ok(saved, "saved");
    ok(await api.fileExists(mpath), "map file written");
    ok(await api.fileExists("biomes/surface/forest.biome.patch"), "forest patch written");
    const patch = await api.readJson("biomes/surface/forest.biome.patch");
    ok(patch?.[0]?.value?.microdungeons?.[0] === "smeselftest", "patch adds the dungeon");

    // re-read the map from disk and compare
    const back = await api.readMap(mpath);
    const rmd = new MapDoc(mpath, back.doc);
    ok(rmd.ref(rmd.get("front", 10, 15))?.tileset === cobble.ref.tileset, "floor survives save/load");
    const rchest = rmd.allObjects().find((e) => e.obj.properties.parameters);
    ok(!!rchest && rmd.tileAnchor(rchest.group, rchest.obj).x === 12, "chest survives save/load");

    const sim = await api.simulateLoot(await api.readJson(lootPath), "smeselftestTreasure", 1, 3);
    ok(sim.length === 3 && sim[0].some(([n]) => n === "money"), "loot simulation rolls the pool");

    await editor.check(true);
    const errs = editor.problems.filter((p) => p.severity === "error" && !p.file.includes("smetest/"));
    ok(errs.length === 0, `no validation errors for the new dungeon (${errs.map((e) => e.message).join("; ")})`);
    log.push(...editor.problems.map((p) => `  ${p.severity}: ${p.file}: ${p.message}`));
  } catch (e) {
    log.push(`FAIL exception: ${e}`);
  }
  await api.writeJson("sme-selftest.json", { log });
}
