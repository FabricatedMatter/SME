<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import Thumb from "./Thumb.svelte";

  // Autocompleting input for item names (or treasure pool names when kind = "pool").
  let { value = $bindable(""), kind = "item", onchange }: { value: string; kind?: "item" | "pool"; onchange?: (v: string) => void } = $props();
  let open = $state(false);
  let q = $state("");
  let active = $state(0);

  const itemByName = $derived(new Map(game.items.map((i) => [i.name, i])));
  const current = $derived(kind === "item" ? itemByName.get(value) : undefined);

  const results = $derived.by(() => {
    const s = q.trim().toLowerCase();
    if (kind === "pool") {
      const names = [...Object.keys(editor.projectPools), ...Object.keys(game.index?.pools ?? {})];
      return names.filter((n) => !s || n.toLowerCase().includes(s)).slice(0, 40).map((n) => ({ name: n, label: editor.projectPools[n] ? "project" : "vanilla", icon: null as string | null }));
    }
    const out = [];
    for (const i of game.items) {
      if (!s || i.name.toLowerCase().includes(s) || i.shortdescription.toLowerCase().includes(s)) {
        out.push({ name: i.name, label: i.shortdescription, icon: i.icon ? "a:" + i.icon : null });
        if (out.length >= 40) break;
      }
    }
    return out;
  });

  function choose(n: string) {
    value = n;
    q = n;
    open = false;
    onchange?.(n);
  }
</script>

<div class="picker">
  {#if kind === "item"}<Thumb image={current?.icon ? "a:" + current.icon : null} size={20} />{/if}
  <input
    value={open ? q : value}
    placeholder={kind === "item" ? "item name" : "pool name"}
    class:unknown={kind === "item" && value && game.items.length > 0 && !current}
    title={current?.shortdescription ?? value}
    onfocus={() => {
      q = value;
      open = true;
      active = 0;
    }}
    oninput={(e) => {
      q = (e.currentTarget as HTMLInputElement).value;
      open = true;
      active = 0;
    }}
    onblur={() =>
      setTimeout(() => {
        if (open && q !== value) choose(q.trim());
        open = false;
      }, 150)}
    onkeydown={(e) => {
      if (e.key === "ArrowDown") active = Math.min(results.length - 1, active + 1);
      else if (e.key === "ArrowUp") active = Math.max(0, active - 1);
      else if (e.key === "Enter") choose(results[active]?.name ?? q.trim());
      else if (e.key === "Escape") open = false;
    }}
  />
  {#if open && results.length}
    <div class="list">
      {#each results as r, i}
        <button class="opt" class:act={i === active} onmousedown={(e) => {
          e.preventDefault();
          choose(r.name);
        }}>
          {#if kind === "item"}<Thumb image={r.icon} size={20} />{/if}
          <span class="n">{r.name}</span>
          <span class="muted">{r.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 3px 6px;
  }
  input.unknown {
    color: var(--warn);
  }
  .list {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 20;
    max-height: 300px;
    overflow-y: auto;
    background: #111;
    border: 2px solid #000;
    box-shadow: 0 0 0 2px #3a3a3a;
    min-width: 320px;
  }
  .opt {
    display: flex;
    width: 100%;
    justify-content: flex-start;
    gap: 8px;
    border: none;
    box-shadow: none;
    background: none;
    padding: 2px 6px;
    text-shadow: none;
  }
  .opt.act,
  .opt:hover:not(:disabled) {
    background: #3a3000;
    box-shadow: none;
  }
  .n {
    color: var(--text);
  }
  .muted {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
