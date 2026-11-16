<script lang="ts">
  import { api } from "../lib/api";
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import ItemPicker from "./ItemPicker.svelte";
  import Thumb from "./Thumb.svelte";

  let { path }: { path: string } = $props();
  const file = $derived(editor.files[path] ?? {});
  const names = $derived(Object.keys(file));
  let selected = $state<string | null>(null);
  let levelIdx = $state(0);
  $effect(() => {
    if (!selected || !names.includes(selected)) selected = names[0] ?? null;
  });
  const pool = $derived<any[] | null>(selected ? file[selected] : null);
  const entry = $derived(pool?.[levelIdx] ?? pool?.[0]);
  const cfg = $derived(entry?.[1]);
  const touch = () => editor.markDirty(path);

  // ── item descriptors ──
  function descName(it: any): string {
    if (typeof it === "string") return it;
    if (Array.isArray(it)) return it[0] ?? "";
    return it?.name ?? it?.item ?? "";
  }
  function descCount(it: any): number {
    if (Array.isArray(it)) return it[1] ?? 1;
    if (it && typeof it === "object") return it.count ?? 1;
    return 1;
  }
  function hasParams(it: any): boolean {
    return (Array.isArray(it) && it[2] !== undefined) || (it && typeof it === "object" && !Array.isArray(it) && it.parameters !== undefined);
  }
  function makeDesc(old: any, name: string, count: number): any {
    if (hasParams(old)) {
      if (Array.isArray(old)) return [name, count, old[2]];
      return { ...old, name, count };
    }
    return [name, count];
  }

  // ── pools ──
  function uniqueName(base: string) {
    let n = base,
      i = 2;
    while (file[n] || editor.projectPools[n] || game.index?.pools[n]) n = `${base}${i++}`;
    return n;
  }
  function addPool() {
    const prefix = (editor.project?.metadata?.name ?? "my").replace(/\W+/g, "").toLowerCase();
    const n = uniqueName(`${prefix}Treasure`);
    file[n] = [[0, { fill: [], pool: [{ weight: 1, pool: "money" }], poolRounds: [[0.5, 1], [0.5, 2]] }]];
    selected = n;
    levelIdx = 0;
    touch();
  }
  function renamePool(old: string, n: string) {
    n = n.trim();
    if (!n || n === old) return;
    if (file[n] || editor.projectPools[n] || game.index?.pools[n]) {
      editor.toast(`A pool named ${n} already exists`, "error");
      return;
    }
    const entries = Object.entries(file).map(([k, v]) => [k === old ? n : k, v]);
    editor.setFile(path, Object.fromEntries(entries));
    selected = n;
    // update chests in open maps
    let changed = 0;
    for (const md of Object.values(editor.maps))
      for (const { obj } of md.allObjects()) {
        const p = obj.properties.parameters;
        if (typeof p !== "string" || !p.includes(old)) continue;
        try {
          const j = JSON.parse(p);
          if (Array.isArray(j.treasurePools) && j.treasurePools.includes(old)) {
            j.treasurePools = j.treasurePools.map((x: string) => (x === old ? n : x));
            obj.properties.parameters = JSON.stringify(j);
            md.touchObjects();
            changed++;
          }
        } catch {
          /* ignore */
        }
      }
    if (changed) editor.toast(`Updated ${changed} chest(s) in open maps`);
  }
  function deletePool(n: string) {
    if (!confirm(`Delete pool ${n}?`)) return;
    delete file[n];
    touch();
  }

  function addLevel() {
    if (!pool) return;
    const last = pool[pool.length - 1];
    pool.push([Number(last?.[0] ?? 0) + 1, JSON.parse(JSON.stringify(last?.[1] ?? { pool: [] }))]);
    pool.sort((a, b) => a[0] - b[0]);
    levelIdx = pool.length - 1;
    touch();
  }
  function removeLevel(i: number) {
    if (!pool || pool.length < 2) return;
    pool.splice(i, 1);
    levelIdx = 0;
    touch();
  }

  const totalWeight = $derived(((cfg?.pool ?? []) as any[]).reduce((s, e) => s + (Number(e.weight) || 0), 0));
  const roundsIsTable = $derived(Array.isArray(cfg?.poolRounds));

  // ── simulation ──
  let simLevel = $state(1);
  let sim = $state<[string, number][][] | null>(null);
  let simErr = $state("");
  async function simulate() {
    if (!selected) return;
    simErr = "";
    try {
      const projectPools: Record<string, any> = {};
      for (const p of editor.poolPaths) Object.assign(projectPools, $state.snapshot(editor.files[p]));
      sim = await api.simulateLoot(projectPools, selected, simLevel, 12);
    } catch (e) {
      simErr = String(e);
      sim = null;
    }
  }
  const itemByName = $derived(new Map(game.items.map((i) => [i.name, i])));
  const usedBy = $derived.by(() => {
    if (!selected) return 0;
    let n = 0;
    for (const md of Object.values(editor.maps)) for (const { obj } of md.allObjects()) if (String(obj.properties.parameters ?? "").includes(`"${selected}"`)) n++;
    return n;
  });
</script>

<div class="page">
  <aside class="pools">
    <div class="bar">
      <span class="label">Pools in {path.split("/").pop()}</span>
    </div>
    {#each names as n}
      <button class="pool" class:on={selected === n} onclick={() => {
        selected = n;
        levelIdx = 0;
      }}>{n}</button>
    {/each}
    <button class="gold small" onclick={addPool}><PixelIcon name="plus" scale={2} /> New pool</button>
    <p class="muted small">Pool names are global across all mods and vanilla, so prefix them with your mod's name.</p>
  </aside>

  {#if selected && pool && cfg}
    <section class="detail">
      <div class="bar">
        <PixelIcon name="chest" scale={3} color="var(--gold)" />
        <input class="pname" value={selected} onchange={(e) => renamePool(selected!, (e.currentTarget as HTMLInputElement).value)} />
        <span class="muted small">used by {usedBy} chest(s) in open maps</span>
        <div class="spacer"></div>
        <button class="red small" onclick={() => deletePool(selected!)}><PixelIcon name="trash" scale={2} /></button>
      </div>

      <div class="levels">
        <span class="muted">Threat level:</span>
        {#each pool as lv, i}
          <span class="lvl" class:on={i === levelIdx}>
            <button class="small" class:gold={i === levelIdx} onclick={() => (levelIdx = i)}>{lv[0]}+</button>
            {#if i === levelIdx}
              <input class="num" type="number" step="0.5" value={lv[0]} onchange={(e) => {
                lv[0] = Number((e.currentTarget as HTMLInputElement).value);
                touch();
              }} />
              {#if pool.length > 1}<button class="small flat" title="Remove level" onclick={() => removeLevel(i)}><PixelIcon name="x" scale={2} /></button>{/if}
            {/if}
          </span>
        {/each}
        <button class="small" title="Different loot from this world threat level up" onclick={addLevel}><PixelIcon name="plus" scale={2} /> level</button>
      </div>

      <div class="box">
        <h4>Always contains <span class="muted small">(fill)</span></h4>
        {#each cfg.fill ?? [] as f, i}
          <div class="row">
            <select value={f.pool !== undefined ? "pool" : "item"} onchange={(e) => {
              const k = (e.currentTarget as HTMLSelectElement).value;
              cfg.fill[i] = k === "pool" ? { pool: "money" } : { item: ["money", 1] };
              touch();
            }}><option value="item">item</option><option value="pool">pool</option></select>
            {#if f.pool !== undefined}
              <ItemPicker kind="pool" value={f.pool} onchange={(v) => {
                f.pool = v;
                touch();
              }} />
            {:else}
              <ItemPicker value={descName(f.item)} onchange={(v) => {
                f.item = makeDesc(f.item, v, descCount(f.item));
                touch();
              }} />
              <input class="num" type="number" min="1" value={descCount(f.item)} onchange={(e) => {
                f.item = makeDesc(f.item, descName(f.item), Number((e.currentTarget as HTMLInputElement).value));
                touch();
              }} />
            {/if}
            <button class="small flat" onclick={() => {
              cfg.fill.splice(i, 1);
              touch();
            }}><PixelIcon name="x" scale={2} /></button>
          </div>
        {/each}
        <button class="small" onclick={() => {
          (cfg.fill ??= []).push({ item: ["money", 50] });
          touch();
        }}><PixelIcon name="plus" scale={2} /> fill entry</button>
      </div>

      <div class="box">
        <h4>Random picks <span class="muted small">(pool: each round picks one entry by weight)</span></h4>
        {#each cfg.pool ?? [] as p, i}
          <div class="row">
            <input class="num" type="number" step="0.01" min="0" title="weight" value={p.weight} onchange={(e) => {
              p.weight = Number((e.currentTarget as HTMLInputElement).value);
              touch();
            }} />
            <span class="pct muted">{totalWeight ? Math.round((100 * (Number(p.weight) || 0)) / totalWeight) : 0}%</span>
            <select value={p.pool !== undefined ? "pool" : "item"} onchange={(e) => {
              const k = (e.currentTarget as HTMLSelectElement).value;
              const w = p.weight;
              cfg.pool[i] = k === "pool" ? { weight: w, pool: "money" } : { weight: w, item: ["money", 1] };
              touch();
            }}><option value="item">item</option><option value="pool">pool</option></select>
            {#if p.pool !== undefined}
              <ItemPicker kind="pool" value={p.pool} onchange={(v) => {
                p.pool = v;
                touch();
              }} />
            {:else}
              <ItemPicker value={descName(p.item)} onchange={(v) => {
                p.item = makeDesc(p.item, v, descCount(p.item));
                touch();
              }} />
              <input class="num" type="number" min="1" value={descCount(p.item)} onchange={(e) => {
                p.item = makeDesc(p.item, descName(p.item), Number((e.currentTarget as HTMLInputElement).value));
                touch();
              }} />
            {/if}
            <button class="small flat" onclick={() => {
              cfg.pool.splice(i, 1);
              touch();
            }}><PixelIcon name="x" scale={2} /></button>
          </div>
        {/each}
        <button class="small" onclick={() => {
          (cfg.pool ??= []).push({ weight: 0.1, item: ["money", 1] });
          touch();
        }}><PixelIcon name="plus" scale={2} /> random entry</button>

        <h4>Rounds <span class="muted small">(how many random picks)</span></h4>
        <div class="row">
          <label><input type="radio" checked={!roundsIsTable} onchange={() => {
            cfg.poolRounds = 1;
            touch();
          }} /> fixed</label>
          <label><input type="radio" checked={roundsIsTable} onchange={() => {
            cfg.poolRounds = [[0.5, 1], [0.5, 2]];
            touch();
          }} /> weighted</label>
          {#if !roundsIsTable}
            <input class="num" type="number" min="0" value={cfg.poolRounds ?? 1} onchange={(e) => {
              cfg.poolRounds = Number((e.currentTarget as HTMLInputElement).value);
              touch();
            }} />
          {/if}
        </div>
        {#if roundsIsTable}
          {#each cfg.poolRounds as r, i}
            <div class="row">
              <span class="muted">weight</span>
              <input class="num" type="number" step="0.05" value={r[0]} onchange={(e) => {
                r[0] = Number((e.currentTarget as HTMLInputElement).value);
                touch();
              }} />
              <span class="muted">→</span>
              <input class="num" type="number" min="0" value={r[1]} onchange={(e) => {
                r[1] = Number((e.currentTarget as HTMLInputElement).value);
                touch();
              }} />
              <span class="muted">round(s)</span>
              <button class="small flat" onclick={() => {
                cfg.poolRounds.splice(i, 1);
                touch();
              }}><PixelIcon name="x" scale={2} /></button>
            </div>
          {/each}
          <button class="small" onclick={() => {
            cfg.poolRounds.push([0.1, 3]);
            touch();
          }}><PixelIcon name="plus" scale={2} /> option</button>
        {/if}
        <div class="row">
          <label class="chk"><input type="checkbox" checked={cfg.allowDuplication ?? true} onchange={(e) => {
            const v = (e.currentTarget as HTMLInputElement).checked;
            if (v) delete cfg.allowDuplication;
            else cfg.allowDuplication = false;
            touch();
          }} /> allow the same item twice</label>
          <span class="muted">level variance</span>
          <input class="num" type="number" value={cfg.levelVariance?.[0] ?? 0} onchange={(e) => {
            cfg.levelVariance = [Number((e.currentTarget as HTMLInputElement).value), cfg.levelVariance?.[1] ?? 0];
            touch();
          }} />
          <input class="num" type="number" value={cfg.levelVariance?.[1] ?? 0} onchange={(e) => {
            cfg.levelVariance = [cfg.levelVariance?.[0] ?? 0, Number((e.currentTarget as HTMLInputElement).value)];
            touch();
          }} />
        </div>
      </div>

      <div class="box">
        <div class="row">
          <h4>Try it</h4>
          <span class="muted">at threat level</span>
          <input class="num" type="number" min="0" step="1" bind:value={simLevel} />
          <button class="gold small" onclick={simulate}><PixelIcon name="spark" scale={2} /> Open 12 chests</button>
        </div>
        {#if simErr}<p class="bad">{simErr}</p>{/if}
        {#if sim}
          <div class="sim">
            {#each sim as chest}
              <div class="chest">
                {#each chest as [n, c]}
                  {@const it = itemByName.get(n)}
                  <span class="it" title="{it?.shortdescription ?? n} ×{c}">
                    <Thumb image={it?.icon ? "a:" + it.icon : null} size={24} />
                    {#if c > 1}<span class="cnt">{c}</span>{/if}
                  </span>
                {:else}
                  <span class="muted small">empty</span>
                {/each}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </section>
  {:else}
    <section class="detail"><p class="muted">No pool selected. Create one on the left.</p></section>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .pools {
    width: 240px;
    flex: none;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-right: 2px solid #000;
    overflow-y: auto;
  }
  .pool {
    justify-content: flex-start;
  }
  .pool.on {
    background: var(--btn-gold-fill);
  }
  .detail {
    flex: 1;
    padding: 12px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .bar,
  .row,
  .levels {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .lvl {
    display: inline-flex;
    gap: 2px;
    align-items: center;
  }
  .pname {
    font-size: 18px;
    width: 320px;
  }
  .spacer {
    flex: 1;
  }
  .box {
    background: var(--inset);
    border: 2px solid var(--inset-line);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h4 {
    margin: 4px 0 0;
    font-weight: normal;
    color: var(--gold);
  }
  .num {
    width: 72px;
    padding: 3px 6px;
  }
  .pct {
    width: 40px;
    text-align: right;
  }
  .chk {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .sim {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 6px;
  }
  .chest {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    background: #0a0a0a;
    border: 2px solid #3a3a3a;
    padding: 4px;
    min-height: 36px;
  }
  .it {
    position: relative;
    background: #1c1c1c;
    padding: 2px;
  }
  .cnt {
    position: absolute;
    right: 0;
    bottom: -2px;
    font-size: 11px;
    text-shadow: 1px 1px 0 #000;
  }
  .bad {
    color: var(--bad);
  }
  .small {
    font-size: 13px;
  }
</style>
