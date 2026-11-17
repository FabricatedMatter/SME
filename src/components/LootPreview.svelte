<script lang="ts">
  // What a loot table contains (for one planet tier) and a few typical rolls.
  import { game } from "../lib/game.svelte";
  import { chanceIn, descCount, descName, loadTable, rollTable, roundsOf, TIERS, versionFor } from "../lib/loot";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Thumb from "./Thumb.svelte";

  let { name }: { name: string } = $props();

  let levels = $state<any[] | null>(null);
  let loading = $state(true);
  let tier = $state(1);
  let rolls = $state<[string, number][][] | null>(null);
  let rollErr = $state("");

  $effect(() => {
    const n = name;
    loading = true;
    levels = null;
    rolls = null;
    void loadTable(n).then((l) => {
      if (n !== name) return;
      levels = l;
      loading = false;
    });
  });
  $effect(() => {
    void name;
    void tier;
    void reroll();
  });
  async function reroll() {
    rollErr = "";
    try {
      rolls = await rollTable(name, tier, 3);
    } catch (e) {
      rollErr = String(e);
      rolls = null;
    }
  }

  const cfg = $derived(levels?.length ? levels[versionFor(levels, tier)][1] : null);
  const rounds = $derived(roundsOf(cfg));
  const itemByName = $derived(new Map(game.items.map((i) => [i.name, i])));
  const icon = (n: string) => {
    const it = itemByName.get(n);
    return it?.icon ? "a:" + it.icon : null;
  };
  const label = (n: string) => itemByName.get(n)?.shortdescription || n;
  const differs = $derived((levels?.length ?? 0) > 1);
</script>

<div class="preview">
  <div class="row">
    <span class="muted small">On a</span>
    <select bind:value={tier}>
      {#each TIERS as t}<option value={t.threat}>tier {t.threat} ({t.material})</option>{/each}
    </select>
    <span class="muted small">planet{differs ? " (this table changes with tier)" : ""}</span>
  </div>

  {#if loading}
    <p class="muted small">Loading…</p>
  {:else if !cfg}
    <p class="muted small">Can't read this table's contents.</p>
  {:else}
    {#if (cfg.fill ?? []).length}
      <div class="label">Always</div>
      <div class="icons">
        {#each cfg.fill as f}
          {#if f.pool !== undefined}
            <span class="chip" title="everything from {f.pool}"><PixelIcon name="chest" scale={1} /> {f.pool}</span>
          {:else}
            {@const n = descName(f.item)}
            <span class="it" title="{label(n)} ×{descCount(f.item)}"><Thumb image={icon(n)} size={24} />{#if descCount(f.item) > 1}<span class="cnt">{descCount(f.item)}</span>{/if}</span>
          {/if}
        {/each}
      </div>
    {/if}
    {#if (cfg.pool ?? []).length}
      <div class="label">Random: {rounds.min === rounds.max ? rounds.min : `${rounds.min}–${rounds.max}`} pick{rounds.max === 1 ? "" : "s"} from</div>
      <div class="icons">
        {#each [...cfg.pool].sort((a, b) => (Number(b.weight) || 0) - (Number(a.weight) || 0)).slice(0, 24) as p}
          {@const pct = Math.round(chanceIn(cfg, Number(p.weight) || 0) * 100)}
          {#if p.pool !== undefined}
            <span class="chip" title="one roll of {p.pool} · {pct}%"><PixelIcon name="chest" scale={1} /> {p.pool} <b>{pct}%</b></span>
          {:else}
            {@const n = descName(p.item)}
            <span class="it" title="{label(n)} ×{descCount(p.item)} · {pct}% chance"><Thumb image={icon(n)} size={24} /><span class="pct">{pct}%</span></span>
          {/if}
        {/each}
        {#if cfg.pool.length > 24}<span class="muted small">+{cfg.pool.length - 24} more</span>{/if}
      </div>
    {/if}
    {#if !(cfg.fill ?? []).length && !(cfg.pool ?? []).length}<p class="muted small">This table is empty.</p>{/if}
  {/if}

  <div class="row">
    <div class="label">Typical containers</div>
    <div class="spacer"></div>
    <button class="small" onclick={reroll}><PixelIcon name="spark" scale={2} /> Roll again</button>
  </div>
  {#if rollErr}<p class="bad small">{rollErr}</p>{/if}
  {#if rolls}
    {#each rolls as chest}
      <div class="chest">
        {#each chest as [n, c]}
          <span class="it" title="{label(n)} ×{c}"><Thumb image={icon(n)} size={24} />{#if c > 1}<span class="cnt">{c}</span>{/if}</span>
        {:else}
          <span class="muted small">empty</span>
        {/each}
      </div>
    {/each}
  {/if}
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .label {
    color: var(--gold);
    font-size: 13px;
  }
  .icons,
  .chest {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .chest {
    background: #0a0a0a;
    border: 2px solid #3a3a3a;
    padding: 3px;
    min-height: 34px;
  }
  .it {
    position: relative;
    background: #1c1c1c;
    padding: 2px;
  }
  .cnt,
  .pct {
    position: absolute;
    right: 0;
    bottom: -2px;
    font-size: 10px;
    text-shadow: 1px 1px 0 #000;
  }
  .pct {
    color: var(--gold);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 12px;
    background: #1c1c1c;
    padding: 2px 4px;
  }
  .small {
    font-size: 13px;
  }
  .bad {
    color: var(--bad);
  }
</style>
