// Game data loaded from the user's install: the catalogue (tilesets, materials, liquids),
// object / item databases and name indexes; plus the brush list shown in the palette.

import { api } from "./api";
import { buildProfile, type Profile } from "./render/tiling";
import type { Catalog, GameIndex, ItemInfo, ObjectInfo, Props, Status, TileInfo, TileRef, TilesetInfo } from "./types";

export type BrushKind = "material" | "platform" | "liquid" | "marker" | "object" | "other";

export interface Brush {
  ref: TileRef;
  key: string;
  kind: BrushKind;
  /** material / object / liquid name (or marker id) */
  name: string;
  label: string;
  desc: string;
  image: string | null;
  props: Props;
  /** filter facets like "race:apex", "category:storage", "type:container", "colony:light" */
  facets: string[];
  vanilla: boolean;
}

export const refKey = (r: TileRef) => `${r.tileset}#${r.index}`;

/** Friendlier names + explanations for the marker tiles in miscellaneous.json. */
export const MARKERS: Record<number, { label: string; help: string; color: string }> = {
  0: { label: "Air", help: "Clears this cell: removes whatever the world generator put here (foreground) or the background wall (back layer).", color: "#9ad" },
  1: { label: "Keep background", help: "Magic pink. On the back layer: keep whatever background the world generator made here, instead of clearing it.", color: "#f6c" },
  2: { label: "Invisible wall", help: "Boundary: light-transparent invisible wall that players cannot climb.", color: "#888" },
  3: { label: "Player start", help: "Where the player spawns (only relevant for dungeon worlds / instances).", color: "#6e4" },
  4: { label: "Anchor: needs air", help: "World-gen anchor. The part only spawns where this cell is air. Put these ABOVE the ground line (e.g. inside the building).", color: "#6cf" },
  5: { label: "Anchor: needs solid ground", help: "World-gen anchor. The part only spawns where this cell is solid. Put these in the ground under the structure. Must be below every 'needs air' anchor.", color: "#c84" },
  6: { label: "Biome item", help: "Spawns a random biome decoration (plant, rock…) here.", color: "#8c4" },
  7: { label: "Biome tree", help: "Spawns a tree of the planet's biome here.", color: "#4a2" },
  8: { label: "Surface block 1", help: "Paints with the biome's own surface block (variant 0). Blends the dungeon into any planet.", color: "#a96" },
  9: { label: "Surface block 2", help: "Biome surface block, variant 1.", color: "#a96" },
  10: { label: "Surface block 3", help: "Biome surface block, variant 2.", color: "#a96" },
  11: { label: "Air (overwritable)", help: "Like Air, but other dungeon parts may draw over it.", color: "#9ad" },
  12: { label: "Connector: red", help: "Connects to other parts' red connectors (multi-part dungeons).", color: "#f44" },
  13: { label: "Connector: yellow", help: "Connects to other parts' yellow connectors.", color: "#fd3" },
  14: { label: "Connector: green", help: "Connects to other parts' green connectors.", color: "#4e4" },
  15: { label: "Connector: blue", help: "Connects to other parts' blue connectors.", color: "#48f" },
  16: { label: "Anchor: needs air (back)", help: "World-gen anchor on the background layer: the background must be empty here.", color: "#6cf" },
  17: { label: "Anchor: needs solid (back)", help: "World-gen anchor on the background layer: there must be a background wall here.", color: "#c84" },
  18: { label: "Invisible wall (climbable)", help: "Structure: light-opaque invisible wall that players can climb.", color: "#777" },
  19: { label: "Underwater wall", help: "Invisible wall that lets liquids through.", color: "#579" },
  20: { label: "Zero gravity", help: "Unprotected area without gravity.", color: "#c6f" },
  21: { label: "Zero gravity (protected)", help: "Tile-protected area without gravity: players cannot dig here.", color: "#c6f" },
  22: { label: "Anchor: needs liquid", help: "World-gen anchor: there must be ocean liquid here.", color: "#39f" },
  23: { label: "Anchor: no liquid", help: "World-gen anchor: there must NOT be ocean liquid here.", color: "#933" },
};

export const MISC = "/tilesets/packed/miscellaneous.json";

function tilesetKind(path: string): BrushKind | null {
  if (path.endsWith("/materials.json")) return "material";
  if (path.endsWith("/supports.json")) return "platform";
  if (path.endsWith("/liquids.json")) return "liquid";
  if (path.endsWith("/miscellaneous.json")) return "marker";
  if (path.includes("/objects") || path.endsWith("huge-objects.json")) return "object";
  return null;
}

function facetOf(path: string): string | null {
  const m = path.match(/objects-by-(race|category|type|colonytag)\/([^/]+)\.json$/);
  if (!m) return null;
  const k = m[1] === "colonytag" ? "colony" : m[1];
  return `${k}:${m[2]}`;
}

class Game {
  status = $state<Status | null>(null);
  catalog = $state.raw<Catalog | null>(null);
  objects = $state.raw<Record<string, ObjectInfo>>({});
  items = $state.raw<ItemInfo[]>([]);
  index = $state.raw<GameIndex | null>(null);
  brushes = $state.raw<Brush[]>([]);
  loading = $state("");
  error = $state("");
  /** bumps when catalogue-derived caches change (renderers listen) */
  version = $state(0);

  private tilesets = new Map<string, TilesetInfo>();
  private profiles = new Map<string, Profile | null>();
  private modProfiles = new Map<string, Profile | null>();
  private brushByKey = new Map<string, Brush>();

  async load() {
    this.error = "";
    this.status = await api.status();
    if (!this.status.install) {
      this.error = this.status.error ?? "Starbound not found";
      return;
    }
    this.loading = "Reading tilesets and materials…";
    try {
      this.catalog = await api.catalog();
      this.indexCatalog();
      this.loading = "Reading objects…";
      this.objects = await api.objectDb();
      this.loading = "";
      // these are only needed by the loot / spawn editors; load in the background
      void api.gameIndex().then((i) => (this.index = i));
      void api.itemDb().then((i) => (this.items = i));
    } catch (e) {
      this.error = String(e);
      this.loading = "";
    }
    this.version++;
  }

  private indexCatalog() {
    const c = this.catalog!;
    this.tilesets = new Map(c.tilesets.map((t) => [t.path, t]));
    this.profiles.clear();
    this.modProfiles.clear();
    const out: Brush[] = [];
    const byObject = new Map<string, Brush>();
    const order = (p: string) =>
      p.includes("objects-by-category") ? 0 : p.includes("objects-by-race") ? 1 : p.includes("objects-by-type") ? 2 : 3;
    const sets = [...c.tilesets].sort((a, b) => order(a.path) - order(b.path) || a.path.localeCompare(b.path));
    this.brushByKey.clear();
    for (const ts of sets) {
      const tk = tilesetKind(ts.path);
      const facet = facetOf(ts.path);
      ts.tiles.forEach((t: TileInfo, index) => {
        const p = t.props;
        if (p.invalid === "true" || p.invalid === true) return;
        const ref = { tileset: ts.path, index };
        let kind: BrushKind = tk ?? "other";
        let name = "";
        if (p.object) {
          kind = "object";
          name = String(p.object);
        } else if (p.liquid) {
          kind = "liquid";
          name = String(p.liquid);
        } else if (p.material) {
          name = String(p.material);
          const m = c.materials[name];
          kind = m && (m.collision === "platform" || m.path.includes("/platforms/")) ? "platform" : name.startsWith("metamaterial:") ? "marker" : "material";
        } else if (tk === "marker") {
          name = String(index);
        } else {
          kind = tk === "object" ? "object" : "other";
          name = String(p["//name"] ?? index);
        }
        const marker = ts.path === MISC ? MARKERS[index] : undefined;
        const label = marker?.label ?? String(p["//shortdescription"] ?? c.materials[name]?.shortdescription ?? name);
        const b: Brush = {
          ref,
          key: refKey(ref),
          kind,
          name,
          label: label.replace(/\^[^;]*;/g, ""),
          desc: marker?.help ?? String(p["//description"] ?? ""),
          image: t.image,
          props: p,
          facets: facet ? [facet] : [],
          vanilla: ts.vanilla,
        };
        if (p.source === "true") b.label += " (endless)";
        this.brushByKey.set(b.key, b);
        if (kind === "object") {
          const prev = byObject.get(name);
          if (prev) {
            if (facet && !prev.facets.includes(facet)) prev.facets.push(facet);
            this.brushByKey.set(b.key, prev);
            return;
          }
          byObject.set(name, b);
        }
        out.push(b);
      });
    }
    this.brushes = out;
  }

  tileset(path: string): TilesetInfo | undefined {
    return this.tilesets.get(path);
  }

  /** Tile properties for a reference, or null if the tileset/tile is unknown. */
  props(ref: TileRef | null | undefined): Props | null {
    if (!ref) return null;
    return this.tilesets.get(ref.tileset)?.tiles[ref.index]?.props ?? null;
  }

  image(ref: TileRef | null | undefined): string | null {
    if (!ref) return null;
    return this.tilesets.get(ref.tileset)?.tiles[ref.index]?.image ?? null;
  }

  brush(ref: TileRef | null | undefined): Brush | undefined {
    return ref ? this.brushByKey.get(refKey(ref)) : undefined;
  }

  brushForObject(name: string): Brush | undefined {
    return this.brushes.find((b) => b.kind === "object" && b.name === name);
  }

  material(name: string): Profile | null {
    if (this.profiles.has(name)) return this.profiles.get(name)!;
    const m = this.catalog?.materials[name];
    const p = m && m.template ? buildProfile(m, this.catalog!.templates[m.template]) : null;
    this.profiles.set(name, p);
    return p;
  }

  mod(name: string): Profile | null {
    if (this.modProfiles.has(name)) return this.modProfiles.get(name)!;
    const m = this.catalog?.mods[name];
    const p = m && m.template ? buildProfile(m, this.catalog!.templates[m.template]) : null;
    this.modProfiles.set(name, p);
    return p;
  }

  liquidColor(name: string): string {
    const l = this.catalog?.liquids[name];
    const c = l?.color ?? [80, 120, 255, 128];
    return `rgba(${c[0]},${c[1]},${c[2]},${Math.max(0.35, (c[3] ?? 128) / 255)})`;
  }
}

export const game = new Game();
