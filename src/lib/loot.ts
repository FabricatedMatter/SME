// Loot helpers shared by the map (container menu), the inspector and the loot editor.
//
// Starbound terms → SME terms:
//   treasure pool         → loot table
//   pool level ("[0, …]") → version for planets of that threat level and up
//   fill                  → always in the chest
//   pool + poolRounds     → random picks (weights → how common, rounds → how many picks)
// A chest uses, for the planet's threat level, the version with the highest start level
// at or below it (or the first version if none is).

import { api } from "./api";
import { editor } from "./editor.svelte";
import { game } from "./game.svelte";
import type { MapDoc } from "./mapdoc.svelte";
import type { ItemInfo, Obj } from "./types";

/** Planet threat level = the tier of armor/weapons you need there. */
export const TIERS = [
  { threat: 1, material: "Iron", item: "ironbar", planets: "Garden, moons, barren worlds" },
  { threat: 2, material: "Tungsten", item: "tungstenbar", planets: "Forest, Desert" },
  { threat: 3, material: "Titanium", item: "titaniumbar", planets: "Ocean, Savannah, Snow" },
  { threat: 4, material: "Durasteel", item: "durasteelbar", planets: "Jungle, Toxic, Alien" },
  { threat: 5, material: "Violium", item: "refinedviolium", planets: "Arctic, Tundra, Midnight" },
  { threat: 6, material: "Solarium", item: "solariumstar", planets: "Volcanic, Magma" },
];

// ── item descriptors: "name", ["name", count, params?] or {name|item, count, parameters} ──
export function descName(it: any): string {
  if (typeof it === "string") return it;
  if (Array.isArray(it)) return it[0] ?? "";
  return it?.name ?? it?.item ?? "";
}
export function descCount(it: any): number {
  if (Array.isArray(it)) return Number(it[1] ?? 1);
  if (it && typeof it === "object") return Number(it.count ?? 1);
  return 1;
}
export function withCount(old: any, count: number): any {
  if (Array.isArray(old)) return old[2] !== undefined ? [old[0], count, old[2]] : [old[0], count];
  if (old && typeof old === "object") return { ...old, count };
  return [old, count];
}

/** Index of the version (level entry) a planet of `threat` uses. */
export function versionFor(levels: any[], threat: number): number {
  let best = -1;
  levels.forEach((l, i) => {
    if (Number(l[0]) <= threat && (best < 0 || Number(l[0]) >= Number(levels[best][0]))) best = i;
  });
  if (best >= 0) return best;
  let low = 0;
  levels.forEach((l, i) => Number(l[0]) < Number(levels[low][0]) && (low = i));
  return low;
}

// ── random picks ──
export interface Rounds {
  /** [weight, number of picks] */
  dist: [number, number][];
  min: number;
  max: number;
  /** odds aren't an even spread between min and max */
  custom: boolean;
}
/** How many random picks a table version makes (`poolRounds`: a number or [[weight, count], …]). */
export function roundsOf(cfg: any): Rounds {
  const r = cfg?.poolRounds;
  if (typeof r === "number") return { dist: [[1, r]], min: r, max: r, custom: false };
  if (Array.isArray(r) && r.length) {
    const dist = r.map((x: any) => [Number(x[0]) || 0, Number(x[1]) || 0] as [number, number]);
    const counts = dist.map((d) => d[1]);
    const min = Math.min(...counts),
      max = Math.max(...counts);
    const even = dist.every((d) => Math.abs(d[0] - dist[0][0]) < 1e-9) && counts.length === max - min + 1;
    return { dist, min, max, custom: !even };
  }
  return { dist: [[1, 1]], min: 1, max: 1, custom: false };
}
/** Chance (0..1) that a random-pick entry of weight `w` turns up at least once in a container. */
export function chanceIn(cfg: any, w: number): number {
  const total = ((cfg?.pool ?? []) as any[]).reduce((s, e) => s + (Number(e.weight) || 0), 0);
  if (!total) return 0;
  const p = w / total;
  const { dist } = roundsOf(cfg);
  const tot = dist.reduce((s, d) => s + d[0], 0) || 1;
  return dist.reduce((s, [dw, n]) => s + (dw / tot) * (1 - Math.pow(1 - p, n)), 0);
}

/** A loot table's versions, from the project or the game. */
export async function loadTable(name: string): Promise<any[] | null> {
  const own = editor.projectPools[name];
  if (own) return editor.files[own]?.[name] ?? null;
  const file = game.index?.pools[name]?.file;
  if (!file) return null;
  try {
    const j = await api.assetJson(file);
    return Array.isArray(j?.[name]) ? j[name] : null;
  } catch {
    return null;
  }
}
/** Opens `n` containers with this table on a planet of `threat` (project tables included). */
export async function rollTable(name: string, threat: number, n: number) {
  const projectPools: Record<string, any> = {};
  for (const p of editor.poolPaths) Object.assign(projectPools, JSON.parse(JSON.stringify(editor.files[p] ?? {})));
  return api.simulateLoot(projectPools, name, threat, n);
}

// ── friendly item groups for the item palette ──
export const ITEM_GROUPS = ["weapons", "armor", "food", "materials", "objects", "other"] as const;
export type ItemGroup = (typeof ITEM_GROUPS)[number];
export const GROUP_LABEL: Record<ItemGroup, string> = {
  weapons: "Weapons",
  armor: "Armor & clothes",
  food: "Food & healing",
  materials: "Materials & money",
  objects: "Furniture & objects",
  other: "Tools & other",
};
const WEAPON_CATS = new Set(
  "broadsword shortsword spear axe hammer dagger bow pistol assaultrifle sniperrifle shotgun rocketlauncher grenadelauncher machinepistol staff wand whip chakram boomerang fistweapon uniqueweapon shield throwableitem".split(" "),
);
const FOOD_CATS = new Set("preparedfood food drink medicine cookingingredient foodjunk".split(" "));
const MAT_CATS = new Set("craftingmaterial block liquid currency upgradecomponent upgrade component platform salvagecomponent tradegoods fuel".split(" "));
export function itemGroup(i: ItemInfo): ItemGroup {
  const c = i.category.toLowerCase();
  if (WEAPON_CATS.has(c) || i.kind === "activeitem" || i.kind === "thrownitem") return "weapons";
  if (["head", "chest", "legs", "back"].includes(i.kind) || c.endsWith("wear") || c.endsWith("armour") || i.kind === "augment") return "armor";
  if (FOOD_CATS.has(c) || i.kind === "consumable") return "food";
  if (MAT_CATS.has(c) || i.kind === "matitem" || i.kind === "liqitem" || i.kind === "currency" || c.includes("material")) return "materials";
  if (i.kind === "object") return "objects";
  return "other";
}

// ── which tables a container uses ──
export function paramsOf(o: Obj): Record<string, any> {
  const p = o.properties.parameters;
  if (!p) return {};
  if (typeof p === "object") return p as Record<string, any>;
  try {
    return JSON.parse(p);
  } catch {
    return {};
  }
}
export function poolsOf(o: Obj): string[] {
  const t = paramsOf(o).treasurePools;
  return Array.isArray(t) ? t.filter((x) => typeof x === "string") : [];
}
export function setPools(md: MapDoc, o: Obj, pools: string[]) {
  const p = paramsOf(o);
  if (pools.length) p.treasurePools = pools;
  else delete p.treasurePools;
  md.begin();
  if (Object.keys(p).length) o.properties = { ...o.properties, parameters: JSON.stringify(p) };
  else {
    const { parameters: _, ...rest } = o.properties;
    o.properties = rest;
  }
  md.commit();
  md.touchObjects();
}

/** The object name of a placed object (e.g. "woodenchest"), if any. */
export function objectNameOf(md: MapDoc, o: Obj): string {
  if (!o.tile) return "";
  const v = game.props(md.ref(o.tile))?.object;
  return v ? String(v) : "";
}

export function poolNameTaken(n: string): boolean {
  return !!(editor.projectPools[n] || game.index?.pools[n]);
}

/** Creates an empty loot table in the project (making a .treasurepools file if needed). */
export function createLootTable(hint: string): { path: string; name: string } {
  const mod = String(editor.project?.metadata?.name ?? "my").replace(/\W+/g, "").toLowerCase() || "my";
  const base = `${mod}${hint.replace(/\W+/g, "").replace(/^./, (c) => c.toUpperCase())}Loot`;
  let name = base,
    i = 2;
  while (poolNameTaken(name)) name = `${base}${i++}`;
  let path = editor.poolPaths[0];
  if (!path) {
    path = `treasure/${mod}.treasurepools`;
    editor.setFile(path, {});
  }
  editor.files[path][name] = [[0, { fill: [], pool: [], poolRounds: [[0.5, 1], [0.5, 2]] }]];
  editor.markDirty(path);
  return { path, name };
}

/** Opens a project loot table in the loot tab, remembering where to go back to. */
export function openLootTable(name: string) {
  const path = editor.projectPools[name];
  if (!path) return false;
  const v = editor.view;
  editor.view = { kind: "loot", path, pool: name, back: v.kind === "map" ? { path: v.path, dungeon: v.dungeon } : undefined };
  return true;
}
