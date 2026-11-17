<script lang="ts">
  // Which loot table(s) a container uses: assign an existing one, create a new one (opens the
  // loot tab), edit or remove. Used by the map's container menu and by the inspector.
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import { createLootTable, openLootTable, poolsOf, setPools } from "../lib/loot";
  import type { MapDoc } from "../lib/mapdoc.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import type { Obj } from "../lib/types";
  import LootPreview from "./LootPreview.svelte";

  let { md, obj, ondone }: { md: MapDoc; obj: Obj; ondone?: () => void } = $props();

  const pools = $derived.by(() => {
    void md.rev;
    return poolsOf(obj);
  });
  let choosing = $state(false);
  /** table being previewed (from the list, or an assigned one) */
  let preview = $state<string | null>(null);
  let q = $state("");
  let searchEl = $state<HTMLInputElement>();
  $effect(() => {
    if (choosing) searchEl?.focus();
  });

  const choices = $derived.by(() => {
    const s = q.trim().toLowerCase();
    const mine = Object.keys(editor.projectPools).filter((n) => !pools.includes(n) && (!s || n.toLowerCase().includes(s)));
    const vanilla = Object.keys(game.index?.pools ?? {})
      .filter((n) => !pools.includes(n) && !editor.projectPools[n] && (!s || n.toLowerCase().includes(s)))
      .sort()
      .slice(0, 80);
    return { mine, vanilla };
  });

  function add(name: string) {
    setPools(md, obj, [...pools, name]);
    choosing = false;
    preview = null;
    q = "";
  }
  function createNew() {
    const hint = md.path.split("/").pop()!.replace(/\.json$/, "");
    const { name } = createLootTable(hint);
    setPools(md, obj, [...pools, name]);
    openLootTable(name);
    ondone?.();
  }
  function edit(name: string) {
    if (openLootTable(name)) ondone?.();
  }
</script>

<div class="assign">
  {#each pools as p, i}
    <div class="row">
      <PixelIcon name="chest" scale={2} color="var(--gold)" />
      <span class="pname" title={p}>{p}</span>
      <button class="small flat" class:on={preview === p && !choosing} title="Show what's in it" onclick={() => {
        choosing = false;
        preview = preview === p ? null : p;
      }}><PixelIcon name="eye" scale={2} /></button>
      {#if editor.projectPools[p]}
        <button class="small gold" onclick={() => edit(p)}>Edit</button>
      {:else if game.index?.pools[p]}
        <span class="muted small" title="A loot table from the base game">game</span>
      {:else}
        <span class="bad small" title="No loot table with this name exists">unknown</span>
      {/if}
      <button class="small flat" title="Remove from this container" onclick={() => setPools(md, obj, pools.filter((_, j) => j !== i))}><PixelIcon name="x" scale={2} /></button>
    </div>
  {:else}
    <p class="muted small">No loot table yet: the container spawns with its default contents (usually empty).</p>
  {/each}

  <div class="acts">
    <button class="small gold" onclick={createNew}><PixelIcon name="plus" scale={2} /> New loot table</button>
    <button class="small" class:on={choosing} onclick={() => {
      choosing = !choosing;
      preview = null;
    }}><PixelIcon name="search" scale={2} /> Use existing…</button>
  </div>

  {#if preview && !choosing}
    <div class="pv"><LootPreview name={preview} /></div>
  {:else if choosing && preview}
    <div class="pvhead">
      <button class="small" onclick={() => (preview = null)}>← Back</button>
      <span class="pname" title={preview}>{preview}</span>
      <button class="small gold" onclick={() => add(preview!)}>Assign</button>
    </div>
    <div class="pv"><LootPreview name={preview} /></div>
  {:else if choosing}
    <input bind:this={searchEl} type="search" placeholder="Search loot tables…" bind:value={q} onkeydown={(e) => e.stopPropagation()} />
    <div class="choices">
      {#if choices.mine.length}<div class="label">Yours</div>{/if}
      {#each choices.mine as n}<button class="choice" onclick={() => (preview = n)}>{n}</button>{/each}
      <div class="label">From the game</div>
      {#each choices.vanilla as n}<button class="choice" onclick={() => (preview = n)}>{n}</button>{/each}
      {#if !choices.mine.length && !choices.vanilla.length}<p class="muted small">No match.</p>{/if}
    </div>
  {/if}
</div>

<style>
  .assign {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acts {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .choices {
    max-height: 220px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .choice {
    justify-content: flex-start;
    background: none;
    border: none;
    box-shadow: none;
    padding: 2px 6px;
    font-size: 14px;
  }
  .choice:hover {
    background: #2a2a2a;
  }
  .pvhead {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .pv {
    max-height: 340px;
    overflow-y: auto;
    padding: 6px;
    background: rgba(0, 0, 0, 0.35);
    border: 2px solid #000;
  }
  .label {
    color: var(--gold);
    font-size: 13px;
    margin-top: 4px;
  }
  .small {
    font-size: 13px;
  }
  .bad {
    color: var(--bad);
  }
</style>
