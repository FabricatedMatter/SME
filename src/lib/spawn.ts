// Spawn rules ⇄ biome .patch files.
// A rule says "spawn these microdungeons on these biomes' surface/underground, with this
// distribution". In the mod it becomes one JSON-patch `add` op per biome file:
//   biomes/surface/forest.biome.patch:
//   [{"op":"add","path":"/surfacePlaceables/items/-","value":{"type":"microdungeon",...}}]
// Ops SME does not recognise are preserved untouched.

import type { FileEntry } from "./types";

export interface SpawnRule {
  id: string;
  dungeons: string[];
  layer: "surface" | "underground";
  mode: "floor" | "ceiling" | "background" | "ocean";
  /** preset key into /biomes/distributions.config, or null for custom */
  distribution: string | null;
  /** custom distribution: random with this block probability */
  probability: number;
  priority: number;
  variants: number;
  /** biome asset paths, e.g. /biomes/surface/forest.biome */
  biomes: string[];
}

export const DIST_PREFIX = "/biomes/distributions.config:";

let seq = 0;
export const ruleId = () => `r${Date.now().toString(36)}${(seq++).toString(36)}`;

export function newRule(dungeon: string): SpawnRule {
  return {
    id: ruleId(),
    dungeons: dungeon ? [dungeon] : [],
    layer: "surface",
    mode: "floor",
    distribution: "mainBiomeMicrodungeon",
    probability: 0.3,
    priority: 1,
    variants: 1,
    biomes: [],
  };
}

function ruleValue(r: SpawnRule): any {
  return {
    mode: r.mode,
    priority: r.priority,
    variants: r.variants,
    distribution: r.distribution ? DIST_PREFIX + r.distribution : { type: "random", blockProbability: r.probability },
    type: "microdungeon",
    microdungeons: r.dungeons,
  };
}

function signature(layer: string, v: any): string {
  return JSON.stringify([layer, v.mode, v.priority, v.variants, v.distribution, v.microdungeons]);
}

export interface ParsedSpawns {
  rules: SpawnRule[];
  /** patch file path → ops SME doesn't manage (kept verbatim) */
  foreign: Record<string, any[]>;
  /** patch files that SME should treat as entirely foreign (e.g. use `test` ops) */
  untouchable: Set<string>;
}

export function parseSpawns(patches: FileEntry[]): ParsedSpawns {
  const bySig = new Map<string, SpawnRule>();
  const foreign: Record<string, any[]> = {};
  const untouchable = new Set<string>();
  for (const f of patches) {
    if (!f.path.endsWith(".biome.patch")) continue;
    const j = f.json;
    if (!Array.isArray(j) || j.some((x) => Array.isArray(x)) || j.some((op) => op?.op === "test")) {
      untouchable.add(f.path);
      continue;
    }
    const biome = "/" + f.path.replace(/\.patch$/, "");
    foreign[f.path] = [];
    for (const op of j) {
      const m = typeof op?.path === "string" ? op.path.match(/^\/(surface|underground)Placeables\/items\/-$/) : null;
      const v = op?.value;
      if (op?.op !== "add" || !m || v?.type !== "microdungeon" || !Array.isArray(v.microdungeons)) {
        foreign[f.path].push(op);
        continue;
      }
      const layer = m[1] as SpawnRule["layer"];
      const sig = signature(layer, v);
      let r = bySig.get(sig);
      if (!r) {
        const dist = v.distribution;
        r = {
          id: ruleId(),
          dungeons: v.microdungeons.map(String),
          layer,
          mode: v.mode ?? "floor",
          distribution: typeof dist === "string" && dist.startsWith(DIST_PREFIX) ? dist.slice(DIST_PREFIX.length) : null,
          probability: typeof dist === "object" && dist?.blockProbability != null ? Number(dist.blockProbability) : 0.3,
          priority: Number(v.priority ?? 1),
          variants: Number(v.variants ?? 1),
          biomes: [],
        };
        bySig.set(sig, r);
      }
      if (!r.biomes.includes(biome)) r.biomes.push(biome);
    }
  }
  return { rules: [...bySig.values()], foreign, untouchable };
}

/** Patch file path → ops, for every biome patch SME manages (empty array = delete file). */
export function buildPatches(rules: SpawnRule[], parsed: ParsedSpawns): Record<string, any[]> {
  const out: Record<string, any[]> = {};
  for (const [p, ops] of Object.entries(parsed.foreign)) out[p] = [...ops];
  for (const r of rules) {
    if (!r.dungeons.length) continue;
    for (const b of r.biomes) {
      const p = b.replace(/^\//, "") + ".patch";
      if (parsed.untouchable.has(p)) continue;
      (out[p] ??= []).push({ op: "add", path: `/${r.layer}Placeables/items/-`, value: ruleValue(r) });
    }
  }
  return out;
}
