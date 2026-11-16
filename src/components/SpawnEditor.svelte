<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import { buildPatches, newRule, type SpawnRule } from "../lib/spawn";
  import type { BiomeInfo } from "../lib/types";

  let filter = $state("");
  let showPatches = $state(false);
  const touch = () => (editor.spawnDirty = true);

  const biomes = $derived(game.index?.biomes ?? []);
  const groups = $derived.by(() => {
    const g = new Map<string, BiomeInfo[]>();
    for (const b of biomes) {
      const folder = b.path.split("/").slice(2, -1).join("/");
      if (folder.includes("old")) continue;
      if (!g.has(folder)) g.set(folder, []);
      g.get(folder)!.push(b);
    }
    const order = ["surface", "underground", "surface_detached", "underground_detached", "core", "space", "atmosphere"];
    const rank = (f: string) => (order.includes(f) ? order.indexOf(f) : 99);
    return [...g.entries()].sort((a, b) => rank(a[0]) - rank(b[0]) || a[0].localeCompare(b[0]));
  });

  const dists = $derived.by(() => {
    const d = game.index?.distributions ?? {};
    const keys = Object.keys(d);
    const micro = keys.filter((k) => /dungeon/i.test(k));
    return [...micro, ...keys.filter((k) => !micro.includes(k))].map((k) => ({ key: k, v: d[k] }));
  });

  function distLabel(v: any): string {
    if (!v) return "";
    if (v.type === "random") return `random, ${Math.round((v.blockProbability ?? 0) * 100)}% per block`;
    return v.type ?? "";
  }

  function biomeName(b: BiomeInfo) {
    return String(b.friendlyName ?? b.name ?? b.path.split("/").pop()?.replace(".biome", ""));
  }

  function toggleBiome(r: SpawnRule, path: string, on: boolean) {
    r.biomes = on ? [...new Set([...r.biomes, path])] : r.biomes.filter((b) => b !== path);
    touch();
  }

  function setGroup(r: SpawnRule, list: BiomeInfo[], on: boolean) {
    const avail = list.filter((b) => (r.layer === "surface" ? b.surface : b.underground)).map((b) => b.path);
    r.biomes = on ? [...new Set([...r.biomes, ...avail])] : r.biomes.filter((b) => !avail.includes(b));
    touch();
  }

  function addRule() {
    editor.spawnRules.push(newRule(editor.dungeonNames[0] ?? ""));
    touch();
  }

  const preview = $derived(showPatches ? buildPatches($state.snapshot(editor.spawnRules) as SpawnRule[], editor.spawnParsed) : {});
</script>

<div class="page">
  <div class="intro panel">
    <h2><PixelIcon name="globe" scale={3} color="var(--gold)" /> Where your microdungeons spawn</h2>
    <p class="muted">
      Each rule adds your dungeons to biomes. <b>Surface biomes are the planet types</b> (forest, desert, tundra…); underground
      biomes are the cave layers. SME writes one <code>.biome.patch</code> per biome. No Lua is needed.
    </p>
    <div class="row">
      <button class="gold" onclick={addRule}><PixelIcon name="plus" scale={2} /> New spawn rule</button>
      <label class="chk"><input type="checkbox" bind:checked={showPatches} /> show generated patch files</label>
    </div>
  </div>

  {#each editor.spawnRules as r, ri (r.id)}
    <section class="panel rule">
      <div class="row head">
        <span class="label">Rule {ri + 1}</span>
        <div class="spacer"></div>
        <button class="red small" onclick={() => {
          editor.spawnRules.splice(ri, 1);
          touch();
        }}><PixelIcon name="trash" scale={2} /></button>
      </div>
      <div class="grid">
        <div class="field">
          <span class="muted">Dungeons <span class="small">(one is picked at random per spot)</span></span>
          <div class="chips">
            {#each [...new Set([...editor.dungeonNames, ...r.dungeons])] as n}
              <label class="chip" class:on={r.dungeons.includes(n)}>
                <input type="checkbox" checked={r.dungeons.includes(n)} onchange={(e) => {
                  r.dungeons = (e.currentTarget as HTMLInputElement).checked ? [...r.dungeons, n] : r.dungeons.filter((x) => x !== n);
                  touch();
                }} />{n}
                {#if !editor.dungeonNames.includes(n)}<span class="small muted">(not in project)</span>{/if}
              </label>
            {/each}
          </div>
        </div>
        <div class="field">
          <span class="muted">Layer</span>
          <select value={r.layer} onchange={(e) => {
            r.layer = (e.currentTarget as HTMLSelectElement).value as SpawnRule["layer"];
            touch();
          }}><option value="surface">surface (planet surface)</option><option value="underground">underground (caves)</option></select>
        </div>
        <div class="field">
          <span class="muted">Placement</span>
          <select value={r.mode} onchange={(e) => {
            r.mode = (e.currentTarget as HTMLSelectElement).value as SpawnRule["mode"];
            touch();
          }}>
            <option value="floor">on the floor</option>
            <option value="ceiling">hanging from ceilings</option>
            <option value="background">in background walls</option>
            <option value="ocean">in oceans</option>
          </select>
        </div>
        <div class="field">
          <span class="muted">How common</span>
          <select value={r.distribution ?? ""} onchange={(e) => {
            const v = (e.currentTarget as HTMLSelectElement).value;
            r.distribution = v || null;
            touch();
          }}>
            <option value="">custom chance…</option>
            {#each dists as d}<option value={d.key}>{d.key}{distLabel(d.v) ? ` (${distLabel(d.v)})` : ""}</option>{/each}
          </select>
          {#if r.distribution === null}
            <label class="row small">chance per spot <input class="num" type="number" step="0.01" min="0" max="1" value={r.probability} onchange={(e) => {
              r.probability = Number((e.currentTarget as HTMLInputElement).value);
              touch();
            }} /></label>
          {/if}
        </div>
        <div class="field">
          <span class="muted">Priority / variants</span>
          <div class="row">
            <input class="num" type="number" value={r.priority} title="priority" onchange={(e) => {
              r.priority = Number((e.currentTarget as HTMLInputElement).value);
              touch();
            }} />
            <input class="num" type="number" min="1" value={r.variants} title="variants" onchange={(e) => {
              r.variants = Number((e.currentTarget as HTMLInputElement).value);
              touch();
            }} />
          </div>
        </div>
      </div>

      <div class="biomes">
        <div class="row">
          <span class="muted">Biomes ({r.biomes.length} selected)</span>
          <input type="search" placeholder="filter biomes" bind:value={filter} />
        </div>
        {#each groups as [folder, list]}
          {@const shown = list.filter((b) => !filter || biomeName(b).toLowerCase().includes(filter.toLowerCase()) || b.path.includes(filter))}
          {#if shown.length}
            <div class="bgroup">
              <div class="row">
                <span class="label">{folder}</span>
                <button class="small" onclick={() => setGroup(r, list, true)}>all</button>
                <button class="small" onclick={() => setGroup(r, list, false)}>none</button>
              </div>
              <div class="blist">
                {#each shown as b}
                  {@const ok = r.layer === "surface" ? b.surface : b.underground}
                  {@const vanillaCount = (r.layer === "surface" ? b.surfaceMicro : b.undergroundMicro).length}
                  <label class="biome" class:on={r.biomes.includes(b.path)} class:dis={!ok} title="{b.path}{ok ? '' : `\nThis biome has no ${r.layer} placeables`}{vanillaCount ? `\nAlready has ${vanillaCount} vanilla microdungeon rule(s)` : ''}">
                    <input type="checkbox" disabled={!ok && !r.biomes.includes(b.path)} checked={r.biomes.includes(b.path)} onchange={(e) => toggleBiome(r, b.path, (e.currentTarget as HTMLInputElement).checked)} />
                    {biomeName(b)}
                  </label>
                {/each}
              </div>
            </div>
          {/if}
        {/each}
        {#if !biomes.length}<p class="muted">Loading biome list…</p>{/if}
      </div>
    </section>
  {:else}
    <p class="muted pad">No spawn rules yet.</p>
  {/each}

  {#if editor.spawnParsed.untouchable.size}
    <p class="muted small pad">Patch files SME leaves as they are (they use test ops or patch sets): {[...editor.spawnParsed.untouchable].join(", ")}</p>
  {/if}

  {#if showPatches}
    <section class="panel">
      <h3>Generated patch files</h3>
      {#each Object.entries(preview) as [p, ops]}
        <div class="patch">
          <div class="label">{p}{ops.length ? "" : " (will be deleted)"}</div>
          <pre>{JSON.stringify(ops, null, 2)}</pre>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .page {
    padding: 14px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2,
  h3 {
    margin: 0 0 6px;
    font-weight: normal;
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .head {
    margin-bottom: 6px;
  }
  .spacer {
    flex: 1;
  }
  .grid {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1.4fr 0.8fr;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip,
  .biome {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px 2px 4px;
    background: #1c1c1c;
    border: 2px solid #000;
    cursor: pointer;
  }
  .chip.on,
  .biome.on {
    background: #2c2600;
  }
  .biome.dis {
    opacity: 0.4;
  }
  .chip input,
  .biomes {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .blist {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }
  .num {
    width: 72px;
    padding: 3px 6px;
  }
  .chk {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  pre {
    background: #0a0a0a;
    padding: 8px;
    font-size: 12px;
    font-family: monospace;
    overflow-x: auto;
    margin: 4px 0 10px;
  }
  .small {
    font-size: 13px;
  }
  .pad {
    padding: 0 6px;
  }
</style>
