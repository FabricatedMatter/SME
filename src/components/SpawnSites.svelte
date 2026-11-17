<script lang="ts" module>
  import type { SpawnSite } from "../lib/types";

  const PLANET_NAMES: Record<string, string> = { scorchedcity: "Scorched City", snow: "Snow", moon: "Moon" };
  export const planetName = (p: string) => PLANET_NAMES[p] ?? p.charAt(0).toUpperCase() + p.slice(1);
  /** the game's planet icon (bookmark icons are named after planet types) */
  export const planetIcon = (p: string) => `a:/interface/bookmarks/icons/${p}.png`;

  /** "/biomes/distributions.config:mainBiomeRareMicrodungeon" → "rare" */
  export function rarityOf(dist: string): string {
    const d = dist.split(":").pop() ?? "";
    if (/rare/i.test(d)) return "rare";
    if (/^tiy/i.test(d)) return "terrain shape";
    if (/encounter/i.test(d)) return "encounter";
    return "common";
  }

  const RANK = { main: 0, secondary: 1, minibiome: 2 } as const;
  /** per layer: each planet once, with the strongest way it gets the dungeon */
  export function byLayer(sites: SpawnSite[]) {
    const out: Record<"surface" | "underground", { planet: string; via: SpawnSite["via"]; biomes: Set<string> }[]> = { surface: [], underground: [] };
    for (const s of sites) {
      const list = out[s.layer];
      const have = list.find((x) => x.planet === s.planet);
      if (!have) list.push({ planet: s.planet, via: s.via, biomes: new Set([s.biome]) });
      else {
        have.biomes.add(s.biome);
        if (RANK[s.via] < RANK[have.via]) have.via = s.via;
      }
    }
    for (const l of Object.values(out)) l.sort((a, b) => RANK[a.via] - RANK[b.via] || a.planet.localeCompare(b.planet));
    return out;
  }
</script>

<script lang="ts">
  import Thumb from "./Thumb.svelte";

  // Where a microdungeon can turn up: planet chips (the game's planet icons) for the surface and
  // for caves. Bright = the planet's main biome; faded = only in some areas (a secondary biome
  // or a minibiome patch).
  let { sites, compact = false }: { sites: SpawnSite[]; compact?: boolean } = $props();
  const layers = $derived(byLayer(sites));
  const VIA_TEXT = { main: "everywhere on the planet", secondary: "in its secondary biome areas", minibiome: "in small minibiome patches" };
</script>

<div class="sites" class:compact>
  {#each [["surface", "Surface"], ["underground", "Caves"]] as const as [k, title]}
    {#if layers[k].length}
      <div class="layer">
        <span class="ltitle label">{title}</span>
        <div class="chips">
          {#each layers[k] as p (p.planet)}
            <span class="chip {p.via}" title="{planetName(p.planet)} planets: {VIA_TEXT[p.via]} ({[...p.biomes].join(', ')} biome)">
              <Thumb image={planetIcon(p.planet)} size={compact ? 14 : 18} />
              {planetName(p.planet)}
            </span>
          {/each}
        </div>
      </div>
    {/if}
  {/each}
  {#if !sites.length}<span class="muted small">Not placed by any planet's biomes (only by quests, missions or other dungeons).</span>{/if}
</div>

<style>
  .sites {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .layer {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }
  .ltitle {
    width: 58px;
    flex: none;
    font-size: 12px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px 1px 3px;
    font-size: 13px;
    background: #1d2a1d;
    border: 1px solid #3c6a3c;
  }
  .chip.secondary,
  .chip.minibiome {
    background: #1a1a1a;
    border: 1px dashed #4a4a4a;
    color: var(--muted);
  }
  .chip.minibiome {
    font-size: 12px;
  }
  .compact .chip {
    font-size: 12px;
    padding: 0 5px 0 2px;
  }
</style>
