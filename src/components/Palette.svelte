<script lang="ts">
  import { editor, isAnchorBrush } from "../lib/editor.svelte";
  import { game, type Brush, type BrushKind } from "../lib/game.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Thumb from "./Thumb.svelte";
  import VirtualGrid from "./VirtualGrid.svelte";

  type Tab = "fav" | "material" | "platform" | "object" | "liquid" | "marker" | "spawn";
  const TABS: { id: Tab; label: string; icon: string }[] = [
    { id: "fav", label: "Favourites & recent", icon: "heart" },
    { id: "material", label: "Blocks", icon: "rect" },
    { id: "platform", label: "Platforms", icon: "line" },
    { id: "object", label: "Objects", icon: "chest" },
    { id: "liquid", label: "Liquids", icon: "bucket" },
    { id: "marker", label: "Markers & anchors", icon: "warn" },
    { id: "spawn", label: "NPCs, monsters & items", icon: "npc" },
  ];

  let tab = $state<Tab>("material");
  let query = $state("");
  let facetKind = $state("category");
  let facet = $state("");

  const facetsOf = $derived.by(() => {
    const out: Record<string, Set<string>> = { category: new Set(), race: new Set(), type: new Set(), colony: new Set() };
    for (const b of game.brushes) if (b.kind === "object") for (const f of b.facets) out[f.split(":")[0]]?.add(f.split(":")[1]);
    return Object.fromEntries(Object.entries(out).map(([k, v]) => [k, [...v].sort()]));
  });

  function matches(b: Brush, q: string): boolean {
    if (!q) return true;
    const hay = `${b.label} ${b.name} ${b.facets.join(" ")}`.toLowerCase();
    return q.split(/\s+/).every((w) => hay.includes(w));
  }

  const list = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let src: Brush[];
    if (tab === "fav") {
      const by = new Map(game.brushes.map((b) => [b.key, b]));
      const keys = [...editor.favorites, ...editor.recentBrushes.filter((k) => !editor.favorites.includes(k))];
      src = keys.map((k) => by.get(k)).filter((b): b is Brush => !!b);
    } else if (tab === "spawn") src = [];
    else {
      const kinds: BrushKind[] = tab === "marker" ? ["marker"] : tab === "object" ? ["object"] : tab === "material" ? ["material", "other"] : [tab];
      src = game.brushes.filter((b) => kinds.includes(b.kind));
      if (tab === "object" && facet) src = src.filter((b) => b.facets.includes(`${facetKind}:${facet}`));
    }
    const filtered = src.filter((b) => matches(b, q));
    if (tab === "material" || tab === "object") filtered.sort((a, b) => a.label.localeCompare(b.label));
    return filtered;
  });

  const favSet = $derived(new Set(editor.favorites));
  const cell = $derived(tab === "object" ? 52 : 38); // px per cell incl. gap
  const thumb = $derived(tab === "object" ? 40 : 28);

  // ── spawn brushes ──
  type SpawnKind = "npc" | "monster" | "stagehand" | "item" | "liquid" | "custom";
  let spawnKind = $state<SpawnKind>("npc");
  let npcSpecies = $state("human");
  let npcType = $state("villager");
  let monster = $state("");
  let stagehand = $state("");
  let item = $state("");
  let itemCount = $state(1);
  let liquid = $state("water");
  let liquidQuantity = $state(1);
  let custom = $state('{\n  "dungeonid": "65525"\n}');
  let customError = $state("");

  function applySpawn() {
    let p: Record<string, string> = {};
    customError = "";
    switch (spawnKind) {
      case "npc":
        p = { npc: npcSpecies, typeName: npcType };
        break;
      case "monster":
        p = { monster };
        break;
      case "stagehand":
        p = { stagehand };
        break;
      case "item":
        p = { item, count: String(itemCount) };
        break;
      case "liquid":
        p = { liquid, quantity: String(liquidQuantity) };
        break;
      case "custom":
        try {
          const j = JSON.parse(custom);
          p = Object.fromEntries(Object.entries(j).map(([k, v]) => [k, typeof v === "string" ? v : JSON.stringify(v)]));
        } catch (e) {
          customError = String(e);
          return;
        }
    }
    editor.spawnBrush = p;
    editor.tool = "spawn";
    editor.layer = "objects";
  }

  function pick(b: Brush) {
    editor.setBrush(b);
    if (isAnchorBrush(b)) editor.layer = "objects";
  }
</script>

<div class="palette">
  <div class="tabs">
    {#each TABS as t}
      <button class="tab" class:on={tab === t.id} title={t.label} onclick={() => (tab = t.id)}>
        <PixelIcon name={t.icon} scale={2} />
      </button>
    {/each}
  </div>
  <div class="tabname label">{TABS.find((t) => t.id === tab)?.label}</div>

  {#if tab === "spawn"}
    <div class="spawn">
      <p class="muted small">Pick what to spawn, then drag a rectangle on the map. The game spawns it inside that area.</p>
      <div class="kinds">
        {#each ["npc", "monster", "stagehand", "item", "liquid", "custom"] as k}
          <button class="small" class:gold={spawnKind === k} onclick={() => (spawnKind = k as SpawnKind)}>{k}</button>
        {/each}
      </div>
      {#if spawnKind === "npc"}
        <label>Species <input list="dl-species" bind:value={npcSpecies} /></label>
        <label>NPC type <input list="dl-npctypes" bind:value={npcType} /></label>
      {:else if spawnKind === "monster"}
        <label>Monster type <input list="dl-monsters" bind:value={monster} placeholder="e.g. poptop" /></label>
      {:else if spawnKind === "stagehand"}
        <label>Stagehand <input list="dl-stagehands" bind:value={stagehand} /></label>
      {:else if spawnKind === "item"}
        <label>Item <input list="dl-items" bind:value={item} /></label>
        <label>Count <input type="number" min="1" bind:value={itemCount} /></label>
      {:else if spawnKind === "liquid"}
        <label>Liquid <select bind:value={liquid}>{#each Object.keys(game.catalog?.liquids ?? {}) as l}<option>{l}</option>{/each}</select></label>
        <label>Quantity (1 = full tile) <input type="number" step="0.1" min="0" bind:value={liquidQuantity} /></label>
      {:else}
        <p class="muted small">Any Tiled brush properties as JSON (e.g. <code>dungeonid</code>, <code>vehicle</code>, <code>radioMessage</code>).</p>
        <textarea rows="5" bind:value={custom}></textarea>
        {#if customError}<span class="bad small">{customError}</span>{/if}
      {/if}
      <button class="gold" onclick={applySpawn}>Use this spawn brush</button>
      {#if editor.tool === "spawn"}
        <p class="small">Active: <code>{JSON.stringify(editor.spawnBrush)}</code></p>
      {/if}
      <datalist id="dl-species">{#each game.index?.species ?? [] as s}<option value={s}></option>{/each}</datalist>
      <datalist id="dl-npctypes">{#each game.index?.npcTypes ?? [] as s}<option value={s}></option>{/each}</datalist>
      <datalist id="dl-monsters">{#each game.index?.monsterTypes ?? [] as s}<option value={s}></option>{/each}</datalist>
      <datalist id="dl-stagehands">{#each game.index?.stagehands ?? [] as s}<option value={s}></option>{/each}</datalist>
      {#if spawnKind === "item"}<datalist id="dl-items">{#each game.items.slice(0, 4000) as s}<option value={s.name}>{s.shortdescription}</option>{/each}</datalist>{/if}
    </div>
  {:else}
    <div class="filters">
      <input type="search" placeholder="Search {list.length} …" bind:value={query} />
      {#if tab === "object"}
        <div class="row">
          <select bind:value={facetKind} onchange={() => (facet = "")}>
            <option value="category">Category</option>
            <option value="race">Race</option>
            <option value="type">Type</option>
            <option value="colony">Colony tag</option>
          </select>
          <select bind:value={facet}>
            <option value="">all</option>
            {#each facetsOf[facetKind] ?? [] as f}<option>{f}</option>{/each}
          </select>
        </div>
      {/if}
    </div>
    {#if tab === "marker"}
      <div class="markers">
        {#each list as b (b.key)}
          <button class="marker" class:on={editor.brush?.key === b.key} onclick={() => pick(b)} title={b.desc}>
            <Thumb image={b.image} size={24} />
            <span class="mlabel">
              <span>{b.label}</span>
              <span class="muted small">{b.desc}</span>
            </span>
          </button>
        {/each}
      </div>
    {:else}
      <VirtualGrid items={list} {cell} key={(b) => b.key} resetOn={`${tab}|${query}|${facet}`}>
        {#snippet children(b)}
              <button
                class="cell"
                class:on={editor.brush?.key === b.key}
                class:fav={favSet.has(b.key)}
                title="{b.label}{b.name && b.name !== b.label ? ` (${b.name})` : ''}{b.desc ? '\n' + b.desc : ''}\nRight-click: favourite"
                onclick={() => pick(b)}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  editor.toggleFavorite(b.key);
                }}
              >
                <Thumb image={b.image} size={thumb} />
              </button>
        {/snippet}
      </VirtualGrid>
      {#if !list.length}<p class="muted small pad">Nothing found.</p>{/if}
    {/if}
  {/if}
  {#if editor.brush}
    <div class="current">
      <Thumb image={editor.brush.image} size={32} flip={editor.flip} />
      <div class="ctext">
        <div>{editor.brush.label}</div>
        <div class="muted small">{editor.brush.kind}{editor.brush.name && editor.brush.kind !== "marker" ? ` · ${editor.brush.name}` : ""}</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .palette {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 6px 6px 0;
  }
  .tab {
    flex: 1;
    padding: 4px 0;
  }
  .tab.on {
    background: var(--btn-gold-fill);
    box-shadow:
      inset 0 2px 0 var(--btn-gold-hi),
      inset 0 -2px 0 var(--btn-gold-lo);
  }
  .tabname {
    padding: 6px 8px 2px;
  }
  .filters {
    padding: 4px 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .filters input,
  .filters select {
    width: 100%;
    min-width: 0;
  }
  .row {
    display: flex;
    gap: 4px;
  }
  .cell {
    padding: 2px;
    min-width: 0;
    background: rgba(0, 0, 0, 0.5);
    border: 2px solid transparent;
    box-shadow: none;
  }
  .cell:hover:not(:disabled) {
    background: #2a2a2a;
    box-shadow: none;
    border-color: var(--btn-hi);
  }
  .cell.on {
    border-color: var(--gold);
    background: #3a3000;
  }
  .cell.fav {
    box-shadow: inset 0 -3px 0 #e04a6a;
  }
  .markers {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 6px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .marker {
    justify-content: flex-start;
    text-align: left;
    white-space: normal;
    gap: 8px;
    padding: 4px 6px;
  }
  .marker.on {
    background: var(--btn-gold-fill);
  }
  .mlabel {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .small {
    font-size: 13px;
  }
  .pad {
    padding: 8px;
  }
  .spawn {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }
  .spawn label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--muted);
  }
  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .bad {
    color: var(--bad);
  }
  .current {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-top: 2px solid #000;
    background: rgba(0, 0, 0, 0.4);
  }
  .ctext {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
