<script lang="ts" generics="T">
  // A scrolling grid of fixed-size cells that only puts the rows in view into the DOM, so lists
  // with thousands of entries (objects, items) stay fast.
  import type { Snippet } from "svelte";

  let {
    items,
    cell,
    key,
    children,
    resetOn,
  }: { items: T[]; cell: number; key: (t: T) => string; children: Snippet<[T]>; resetOn?: unknown } = $props();

  let el = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let viewW = $state(300);
  let viewH = $state(400);

  $effect(() => {
    void resetOn;
    if (el) el.scrollTop = 0;
    scrollTop = 0;
  });

  const cols = $derived(Math.max(1, Math.floor((viewW - 12) / cell)));
  const rows = $derived(Math.ceil(items.length / cols));
  const first = $derived(Math.max(0, Math.floor(scrollTop / cell) - 2));
  const last = $derived(Math.min(rows, Math.ceil((scrollTop + viewH) / cell) + 2));
  const visible = $derived(items.slice(first * cols, last * cols));
</script>

<div
  class="vgrid"
  bind:this={el}
  bind:clientWidth={viewW}
  bind:clientHeight={viewH}
  onscroll={(e) => (scrollTop = (e.currentTarget as HTMLDivElement).scrollTop)}
>
  <div class="spacer" style:height="{rows * cell}px">
    <div class="cells" style:top="{first * cell}px" style:grid-template-columns="repeat({cols}, {cell - 2}px)" style:grid-auto-rows="{cell - 2}px">
      {#each visible as it (key(it))}
        {@render children(it)}
      {/each}
    </div>
  </div>
</div>

<style>
  .vgrid {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 6px;
  }
  .spacer {
    position: relative;
  }
  .cells {
    position: absolute;
    left: 0;
    right: 0;
    display: grid;
    gap: 2px;
  }
</style>
