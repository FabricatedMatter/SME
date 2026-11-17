<script lang="ts">
  // The loot tab: what a container (chest, crate, barrel…) can contain. Items are picked from an
  // item palette on the right; the middle shows "always in the container" and "random picks",
  // and a planet-tier strip for tables that differ between easy and dangerous planets.
  import { api } from "../lib/api";
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import { chanceIn, descCount, descName, GROUP_LABEL, ITEM_GROUPS, itemGroup, poolNameTaken, roundsOf, TIERS, versionFor, withCount, type ItemGroup } from "../lib/loot";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import type { ItemInfo } from "../lib/types";
  import Thumb from "./Thumb.svelte";
  import VirtualGrid from "./VirtualGrid.svelte";

  let { path }: { path: string } = $props();
  const file = $derived(editor.files[path] ?? {});
  const names = $derived(Object.keys(file));
  let selected = $state<string | null>(null);
  let tier = $state(1); // selected planet tier on the strip

  // open the table requested by the view (e.g. from a container's menu), once per navigation
  let appliedView: unknown = null;
  $effect(() => {
    const v = editor.view;
    if (v === appliedView || v.kind !== "loot" || !v.pool || !names.includes(v.pool)) return;
    appliedView = v;
    selected = v.pool;
    tier = 1;
  });
  $effect(() => {
    if (!selected || !names.includes(selected)) selected = names[0] ?? null;
  });
  const back = $derived(editor.view.kind === "loot" ? editor.view.back : undefined);

  const levels = $derived<any[] | null>(selected && Array.isArray(file[selected]) ? file[selected] : null);
  const vIdx = $derived(levels?.length ? versionFor(levels, tier) : -1);
  const cfg = $derived(levels && vIdx >= 0 ? levels[vIdx][1] : null);
  const touch = () => editor.markDirty(path);

  // ── versions per planet tier ──
  const VCOLORS = ["#3a7bd5", "#c9a227", "#8e44ad", "#27ae60", "#c0392b", "#16a085"];
  const versionOfTier = $derived(levels ? TIERS.map((t) => versionFor(levels, t.threat)) : []);
  const startsAt = (i: number) => Math.max(1, Math.ceil(Number(levels?.[i]?.[0] ?? 0)));
  const range = $derived.by(() => {
    const ts = TIERS.filter((_, k) => versionOfTier[k] === vIdx).map((t) => t.threat);
    return ts.length ? (ts[0] === ts[ts.length - 1] ? `tier ${ts[0]}` : `tiers ${ts[0]}–${ts[ts.length - 1]}`) : "no tier";
  });
  function splitHere() {
    if (!levels || !cfg) return;
    levels.push([tier, JSON.parse(JSON.stringify(cfg))]);
    levels.sort((a, b) => Number(a[0]) - Number(b[0]));
    touch();
  }
  function mergeVersion() {
    if (!levels || levels.length < 2 || vIdx < 0) return;
    levels.splice(vIdx, 1);
    touch();
  }
  const isFirstVersion = $derived(!!levels && vIdx >= 0 && levels.every((l) => Number(l[0]) >= Number(levels[vIdx][0])));
  const isVersionStart = $derived(!!levels && vIdx >= 0 && startsAt(vIdx) === tier);

  // ── pools list ──
  function uniqueName(base: string) {
    let n = base,
      i = 2;
    while (poolNameTaken(n) || file[n]) n = `${base}${i++}`;
    return n;
  }
  function addPool() {
    const prefix = String(editor.project?.metadata?.name ?? "my").replace(/\W+/g, "").toLowerCase() || "my";
    const n = uniqueName(`${prefix}Loot`);
    file[n] = [[0, { fill: [], pool: [], poolRounds: [[0.5, 1], [0.5, 2]] }]];
    selected = n;
    tier = 1;
    touch();
  }
  function renamePool(old: string, n: string) {
    n = n.trim();
    if (!n || n === old) return;
    if (file[n] || poolNameTaken(n)) {
      editor.toast(`A loot table named ${n} already exists`, "error");
      return;
    }
    editor.setFile(path, Object.fromEntries(Object.entries(file).map(([k, v]) => [k === old ? n : k, v])));
    selected = n;
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
          /* not JSON */
        }
      }
    if (changed) editor.toast(`Updated ${changed} container(s) in open maps`);
  }
  function deletePool(n: string) {
    if (!confirm(`Delete the loot table ${n}? Containers that use it will be flagged by Check.`)) return;
    delete file[n];
    touch();
  }
  const usedBy = $derived.by(() => {
    if (!selected) return 0;
    let n = 0;
    for (const md of Object.values(editor.maps)) for (const { obj } of md.allObjects()) if (String(obj.properties.parameters ?? "").includes(`"${selected}"`)) n++;
    return n;
  });

  // ── entries ──
  const itemByName = $derived(new Map(game.items.map((i) => [i.name, i])));
  const iconOf = (n: string) => {
    const it = itemByName.get(n);
    return it?.icon ? "a:" + it.icon : null;
  };
  const labelOf = (n: string) => itemByName.get(n)?.shortdescription || n;

  const RARITY = [
    { label: "Common", w: 1 },
    { label: "Uncommon", w: 0.4 },
    { label: "Rare", w: 0.15 },
    { label: "Very rare", w: 0.04 },
  ];
  const rarityOf = (w: number) => RARITY.find((r) => Math.abs(r.w - w) < 1e-9)?.label ?? "custom";

  // picks per chest: number, or [[weight, count], …]
  const rounds = $derived(roundsOf(cfg));
  function setRounds(min: number, max: number) {
    if (!cfg) return;
    min = Math.max(0, Math.round(min));
    max = Math.max(min, Math.round(max));
    if (min === max) cfg.poolRounds = min;
    else {
      const n = max - min + 1;
      cfg.poolRounds = Array.from({ length: n }, (_, k) => [Math.round(1000 / n) / 1000, min + k]);
    }
    touch();
  }
  /** chance that an entry turns up at least once in a container */
  const chanceInChest = (w: number) => chanceIn(cfg, w);

  // ── item palette ──
  type Target = "fill" | "pool";
  let target = $state<Target>("pool");
  let group = $state<ItemGroup | "tables">("weapons");
  let q = $state("");
  const grouped = $derived.by(() => {
    const m = new Map<ItemGroup, ItemInfo[]>(ITEM_GROUPS.map((g) => [g, []]));
    for (const i of game.items) if (i.icon) m.get(itemGroup(i))!.push(i);
    for (const l of m.values()) l.sort((a, b) => (a.shortdescription || a.name).localeCompare(b.shortdescription || b.name));
    return m;
  });
  // sorting: by name or rarity, ascending or descending
  let sortBy = $state<"name" | "rarity">("name");
  let descending = $state(false);
  const RANK: Record<string, number> = { common: 0, uncommon: 1, rare: 2, legendary: 3, essential: 4 };
  const label = (i: ItemInfo) => i.shortdescription || i.name;
  const shownItems = $derived.by(() => {
    if (group === "tables") return [];
    const s = q.trim().toLowerCase();
    const src = s ? game.items.filter((i) => i.icon) : (grouped.get(group) ?? []);
    const list = s ? src.filter((i) => `${i.shortdescription} ${i.name} ${i.category}`.toLowerCase().includes(s)) : [...src];
    const dir = descending ? -1 : 1;
    list.sort((a, b) =>
      sortBy === "rarity"
        ? dir * ((RANK[a.rarity] ?? -1) - (RANK[b.rarity] ?? -1)) || label(a).localeCompare(label(b))
        : dir * label(a).localeCompare(label(b)),
    );
    return list;
  });

  // instant hover label (the native title tooltip is slow / missing in the app window)
  let tip = $state<{ text: string; sub: string; x: number; y: number } | null>(null);
  $effect(() => {
    void shownItems;
    tip = null;
  });
  function showTip(e: PointerEvent, i: ItemInfo) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    tip = { text: label(i), sub: [i.rarity, i.name].filter(Boolean).join(" · "), x: r.left + r.width / 2, y: r.top - 4 };
  }
  const shownTables = $derived.by(() => {
    const s = q.trim().toLowerCase();
    const all = [...Object.keys(editor.projectPools).filter((n) => n !== selected), ...Object.keys(game.index?.pools ?? {}).filter((n) => !editor.projectPools[n]).sort()];
    return all.filter((n) => !s || n.toLowerCase().includes(s)).slice(0, 300);
  });
  const inTable = $derived(new Set([...(cfg?.fill ?? []), ...(cfg?.pool ?? [])].map((e: any) => (e.pool !== undefined ? "pool:" + e.pool : descName(e.item)))));

  function addItem(i: ItemInfo) {
    if (!cfg) return;
    const list = (cfg[target] ??= []);
    const existing = list.find((e: any) => e.item !== undefined && descName(e.item) === i.name);
    if (existing) existing.item = withCount(existing.item, descCount(existing.item) + 1);
    else {
      const count = i.name === "money" ? 50 : 1;
      list.push(target === "pool" ? { weight: 1, item: [i.name, count] } : { item: [i.name, count] });
    }
    touch();
  }
  function addTable(n: string) {
    if (!cfg) return;
    (cfg[target] ??= []).push(target === "pool" ? { weight: 1, pool: n } : { pool: n });
    touch();
  }
  const rarityColor: Record<string, string> = { common: "#8a8a8a", uncommon: "#3fbf3f", rare: "#3f8fff", legendary: "#b05cff", essential: "#e6c200" };

  // ── try it ──
  let simTier = $state(1);
  let sim = $state<[string, number][][] | null>(null);
  let simErr = $state("");
  async function simulate() {
    if (!selected) return;
    simErr = "";
    try {
      const projectPools: Record<string, any> = {};
      for (const p of editor.poolPaths) Object.assign(projectPools, $state.snapshot(editor.files[p]));
      sim = await api.simulateLoot(projectPools, selected, simTier, 12);
    } catch (e) {
      simErr = String(e);
      sim = null;
    }
  }
</script>

<div class="page">
  <aside class="pools">
    <div class="label">Loot tables</div>
    {#each names as n}
      <button class="pool" class:on={selected === n} onclick={() => {
        selected = n;
        tier = 1;
      }} title={n}>{n}</button>
    {/each}
    <button class="gold small" onclick={addPool}><PixelIcon name="plus" scale={2} /> New loot table</button>
    <p class="muted small">Give a container a table by right-clicking or double-clicking it on the map. Table names are shared with the game and all other mods, so SME prefixes them with your mod's name.</p>
  </aside>

  {#if selected && levels && cfg}
    <section class="detail">
      {#if back}
        <button class="small backbtn" onclick={() => editor.openMap(back.path, back.dungeon)}>← Back to {back.path.split("/").pop()?.replace(/\.json$/, "")}</button>
      {/if}
      <div class="bar">
        <PixelIcon name="chest" scale={3} color="var(--gold)" />
        <input class="pname" value={selected} onchange={(e) => renamePool(selected!, (e.currentTarget as HTMLInputElement).value)} title="Rename (containers in open maps are updated)" />
        <span class="muted small">used by {usedBy} container{usedBy === 1 ? "" : "s"} in open maps</span>
        <div class="spacer"></div>
        <button class="red small" title="Delete this loot table" onclick={() => deletePool(selected!)}><PixelIcon name="trash" scale={2} /></button>
      </div>

      <div class="box">
        <div class="row">
          <h4>Planet tier</h4>
          <span class="muted small">A planet's threat level is the armor tier you need there. Loot can differ between easy and dangerous planets.</span>
        </div>
        <div class="tiers">
          {#each TIERS as t, k}
            {@const v = versionOfTier[k]}
            <button class="tier" class:on={tier === t.threat} style:--vc={VCOLORS[v % VCOLORS.length]} onclick={() => (tier = t.threat)} title="Threat {t.threat}: {t.planets}">
              <Thumb image={iconOf(t.item)} size={24} />
              <span class="tname">Tier {t.threat} · {t.material}</span>
              <span class="tplan muted">{t.planets}</span>
              {#if levels.length > 1}<span class="vtag">{String.fromCharCode(65 + v)}</span>{/if}
            </button>
          {/each}
        </div>
        <div class="row">
          {#if levels.length > 1}
            <span>Editing version <b style:color={VCOLORS[vIdx % VCOLORS.length]}>{String.fromCharCode(65 + vIdx)}</b>, used on {range} planets.</span>
          {:else}
            <span class="muted">The same loot on every planet.</span>
          {/if}
          <div class="spacer"></div>
          {#if isVersionStart && isFirstVersion}
            <span class="muted small">Pick a higher tier to give dangerous planets different loot.</span>
          {:else if isVersionStart}
            <button class="small" onclick={mergeVersion}>Use the previous version from tier {tier} on</button>
          {:else}
            <button class="small" onclick={splitHere} title="Copies the current loot so you can change it for harder planets">Different loot from tier {tier} on</button>
          {/if}
        </div>
      </div>

      <!-- clicking a box just chooses where palette items go (also available as buttons in the item panel) -->
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="box target" class:active={target === "fill"} onclick={() => (target = "fill")}>
        <h4><input type="radio" checked={target === "fill"} /> Always in the container</h4>
        {#each cfg.fill ?? [] as f, i}
          <div class="entry">
            {#if f.pool !== undefined}
              <PixelIcon name="chest" scale={2} color="var(--gold)" />
              <span class="ename">everything from <b>{f.pool}</b></span>
            {:else}
              <Thumb image={iconOf(descName(f.item))} size={28} />
              <span class="ename" title={descName(f.item)}>{labelOf(descName(f.item))}</span>
              <span class="muted">×</span>
              <input class="num" type="number" min="1" value={descCount(f.item)} onchange={(e) => {
                f.item = withCount(f.item, Number((e.currentTarget as HTMLInputElement).value) || 1);
                touch();
              }} />
            {/if}
            <button class="small flat" title="Remove" onclick={(e) => {
              e.stopPropagation();
              cfg.fill.splice(i, 1);
              touch();
            }}><PixelIcon name="x" scale={2} /></button>
          </div>
        {:else}
          <p class="muted small">Nothing yet. Select this box, then click items on the right.</p>
        {/each}
      </div>

      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="box target" class:active={target === "pool"} onclick={() => (target = "pool")}>
        <h4><input type="radio" checked={target === "pool"} /> Random picks</h4>
        <div class="row picks">
          <span>Each container gets</span>
          <input class="num" type="number" min="0" value={rounds.min} onchange={(e) => setRounds(Number((e.currentTarget as HTMLInputElement).value), Math.max(rounds.max, Number((e.currentTarget as HTMLInputElement).value)))} />
          <span>to</span>
          <input class="num" type="number" min="0" value={rounds.max} onchange={(e) => setRounds(Math.min(rounds.min, Number((e.currentTarget as HTMLInputElement).value)), Number((e.currentTarget as HTMLInputElement).value))} />
          <span>random picks from this list.</span>
          {#if rounds.custom}<span class="muted small" title={JSON.stringify(cfg.poolRounds)}>(uneven odds from the original file; changing a number makes them even)</span>{/if}
        </div>
        {#each cfg.pool ?? [] as p, i}
          {@const w = Number(p.weight) || 0}
          <div class="entry">
            {#if p.pool !== undefined}
              <PixelIcon name="chest" scale={2} color="var(--gold)" />
              <span class="ename">one roll of <b>{p.pool}</b></span>
            {:else}
              <Thumb image={iconOf(descName(p.item))} size={28} />
              <span class="ename" title={descName(p.item)}>{labelOf(descName(p.item))}</span>
              <span class="muted">×</span>
              <input class="num" type="number" min="1" value={descCount(p.item)} onchange={(e) => {
                p.item = withCount(p.item, Number((e.currentTarget as HTMLInputElement).value) || 1);
                touch();
              }} />
            {/if}
            <select value={rarityOf(w)} onchange={(e) => {
              const r = RARITY.find((x) => x.label === (e.currentTarget as HTMLSelectElement).value);
              if (r) {
                p.weight = r.w;
                touch();
              }
            }}>
              {#each RARITY as r}<option>{r.label}</option>{/each}
              {#if rarityOf(w) === "custom"}<option value="custom">weight {w}</option>{/if}
            </select>
            <span class="chance" title="Chance to find it in a container">{Math.round(chanceInChest(w) * 100)}%</span>
            <button class="small flat" title="Remove" onclick={(e) => {
              e.stopPropagation();
              cfg.pool.splice(i, 1);
              touch();
            }}><PixelIcon name="x" scale={2} /></button>
          </div>
        {:else}
          <p class="muted small">Nothing yet. Select this box, then click items on the right.</p>
        {/each}
        {#if (cfg.pool ?? []).length}
          <p class="muted small">The % is the chance to find that entry in a container. "Common" entries are picked more often than "Rare" ones.</p>
        {/if}
      </div>

      <details class="box">
        <summary>More options</summary>
        <label class="chk"><input type="checkbox" checked={cfg.allowDuplication ?? true} onchange={(e) => {
          if ((e.currentTarget as HTMLInputElement).checked) delete cfg.allowDuplication;
          else cfg.allowDuplication = false;
          touch();
        }} /> The same random pick may appear more than once</label>
        <div class="row">
          <span>Weapon/armor level variation</span>
          <input class="num" type="number" value={cfg.levelVariance?.[0] ?? 0} onchange={(e) => {
            cfg.levelVariance = [Number((e.currentTarget as HTMLInputElement).value), cfg.levelVariance?.[1] ?? 0];
            touch();
          }} />
          <span>to</span>
          <input class="num" type="number" value={cfg.levelVariance?.[1] ?? 0} onchange={(e) => {
            cfg.levelVariance = [cfg.levelVariance?.[0] ?? 0, Number((e.currentTarget as HTMLInputElement).value)];
            touch();
          }} />
          <span class="muted small">tiers around the planet's (e.g. −1 to 1)</span>
        </div>
      </details>

      <div class="box">
        <div class="row">
          <h4>Try it</h4>
          <span class="muted">on a</span>
          <select bind:value={simTier}>
            {#each TIERS as t}<option value={t.threat}>tier {t.threat} ({t.material}) planet</option>{/each}
          </select>
          <button class="gold small" onclick={simulate}><PixelIcon name="spark" scale={2} /> Open 12 containers</button>
        </div>
        {#if simErr}<p class="bad">{simErr}</p>{/if}
        {#if sim}
          <div class="sim">
            {#each sim as chest}
              <div class="chest">
                {#each chest as [n, c]}
                  <span class="it" title="{labelOf(n)} ×{c}">
                    <Thumb image={iconOf(n)} size={24} />
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

    <aside class="items">
      <div class="ihead">
        <span class="label">Add to</span>
        <div class="seg">
          <button class="small" class:gold={target === "fill"} onclick={() => (target = "fill")}>Always</button>
          <button class="small" class:gold={target === "pool"} onclick={() => (target = "pool")}>Random</button>
        </div>
      </div>
      <input type="search" placeholder="Search all items…" bind:value={q} />
      <div class="sortrow">
        <span class="muted small">Sort by</span>
        <select bind:value={sortBy}>
          <option value="name">Name</option>
          <option value="rarity">Rarity</option>
        </select>
        <button class="small" onclick={() => (descending = !descending)} title="Toggle ascending / descending">
          {#if sortBy === "name"}{descending ? "Z → A" : "A → Z"}{:else}{descending ? "Rarest first" : "Commonest first"}{/if}
        </button>
      </div>
      <div class="groups">
        {#each ITEM_GROUPS as g}
          <button class="small" class:gold={group === g && !q} onclick={() => {
            group = g;
            q = "";
          }}>{GROUP_LABEL[g]}</button>
        {/each}
        <button class="small" class:gold={group === "tables"} onclick={() => (group = "tables")} title="Put a whole other loot table in (e.g. the game's 'money' or 'weapon' tables)">Other loot tables</button>
      </div>
      {#if group === "tables"}
        <div class="tables">
          {#each shownTables as n}
            <button class="tablebtn" class:has={inTable.has("pool:" + n)} onclick={() => addTable(n)}>
              <PixelIcon name="chest" scale={2} color={editor.projectPools[n] ? "var(--gold)" : "var(--muted)"} /> {n}
            </button>
          {/each}
        </div>
      {:else}
        <VirtualGrid items={shownItems} cell={46} key={(i) => i.name} resetOn={`${group}|${q}|${sortBy}|${descending}`}>
          {#snippet children(i)}
            <button
              class="icell"
              class:has={inTable.has(i.name)}
              style:--rc={rarityColor[i.rarity] ?? "#555"}
              onclick={() => addItem(i)}
              onpointerenter={(e) => showTip(e, i)}
              onpointerleave={() => (tip = null)}
            >
              <Thumb image={"a:" + i.icon} size={32} />
            </button>
          {/snippet}
        </VirtualGrid>
        {#if !shownItems.length}<p class="muted small pad">No items found.</p>{/if}
      {/if}
    </aside>
    {#if tip}
      <div class="tip" style:left="{tip.x}px" style:top="{tip.y}px">
        <div>{tip.text}</div>
        {#if tip.sub}<div class="muted small">{tip.sub}</div>{/if}
      </div>
    {/if}
  {:else}
    <section class="detail"><p class="muted">No loot table yet. Create one on the left, or right-click a container on the map.</p></section>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .pools {
    width: 220px;
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
    text-align: left;
  }
  .pool.on {
    background: var(--btn-gold-fill);
  }
  .detail {
    flex: 1;
    min-width: 0;
    padding: 12px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .backbtn {
    align-self: flex-start;
  }
  .bar,
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .pname {
    font-size: 18px;
    flex: 1;
    min-width: 160px;
    max-width: 360px;
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
  .box.target {
    cursor: pointer;
  }
  .box.target.active {
    border-color: var(--gold);
  }
  h4 {
    margin: 0;
    font-weight: normal;
    color: var(--gold);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  summary {
    color: var(--gold);
    cursor: pointer;
  }
  .tiers {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 4px;
  }
  .tier {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: 6px;
    align-items: center;
    justify-items: start;
    text-align: left;
    padding: 4px 6px 4px 8px;
    box-shadow: inset 4px 0 0 var(--vc);
  }
  .tier :global(.thumb) {
    grid-row: span 2;
  }
  .tier.on {
    background: #3a3000;
    border-color: var(--gold);
  }
  .tname,
  .tplan {
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tname {
    font-size: 14px;
  }
  .tplan {
    font-size: 11px;
    line-height: 1.1;
  }
  .vtag {
    position: absolute;
    top: 2px;
    right: 4px;
    font-size: 12px;
    color: var(--vc);
  }
  .entry {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .ename {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .num {
    width: 64px;
    padding: 3px 6px;
  }
  .chance {
    width: 44px;
    text-align: right;
    color: var(--gold);
  }
  .picks span {
    white-space: nowrap;
  }
  .chk {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .items {
    width: 320px;
    flex: none;
    border-left: 2px solid #000;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 0 0;
    min-height: 0;
  }
  .items > input {
    margin: 0 8px;
  }
  .ihead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
  }
  .seg {
    display: flex;
    gap: 2px;
  }
  .sortrow {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 0 8px;
  }
  .sortrow select {
    flex: 1;
  }
  .tip {
    position: fixed;
    z-index: 60;
    transform: translate(-50%, -100%);
    pointer-events: none;
    white-space: nowrap;
    padding: 4px 8px;
    background: #101010;
    border: 2px solid #000;
    box-shadow: inset 0 0 0 2px #3c3c3c;
  }
  .groups {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    padding: 0 8px;
  }
  .icell {
    padding: 2px;
    min-width: 0;
    background: rgba(0, 0, 0, 0.5);
    border: 2px solid transparent;
    box-shadow: inset 0 -3px 0 var(--rc);
  }
  .icell:hover {
    border-color: var(--btn-hi);
  }
  .icell.has,
  .tablebtn.has {
    background: #3a3000;
  }
  .tables {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0 8px;
  }
  .tablebtn {
    justify-content: flex-start;
    background: none;
    border: none;
    box-shadow: none;
    padding: 3px 6px;
    font-size: 14px;
  }
  .tablebtn:hover {
    background: #2a2a2a;
  }
  .sim {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
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
  .pad {
    padding: 8px;
  }
</style>
