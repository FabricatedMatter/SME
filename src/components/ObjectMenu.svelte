<script lang="ts">
  // Right-click / double-click menu for a placed object. Containers get their loot here.
  import { onMount } from "svelte";
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import { objectNameOf } from "../lib/loot";
  import type { MapDoc } from "../lib/mapdoc.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import LootAssign from "./LootAssign.svelte";
  import Thumb from "./Thumb.svelte";

  let { md, id, x, y, onclose }: { md: MapDoc; id: number; x: number; y: number; onclose: () => void } = $props();

  const entry = $derived.by(() => {
    void md.rev;
    return md.findObject(id);
  });
  const name = $derived(entry ? objectNameOf(md, entry.obj) : "");
  const info = $derived(name ? game.objects[name] : undefined);
  const image = $derived(entry?.obj.tile ? game.image(md.ref(entry.obj.tile)) : null);

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });
  // keep the menu on screen, also when it grows (e.g. a loot preview opens)
  $effect(() => {
    if (!el) return;
    const place = () => {
      const r = el!.getBoundingClientRect();
      pos = { x: Math.max(4, Math.min(x, window.innerWidth - r.width - 4)), y: Math.max(4, Math.min(y, window.innerHeight - r.height - 4)) };
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(el);
    return () => ro.disconnect();
  });

  onMount(() => {
    const down = (e: PointerEvent) => {
      if (el && !el.contains(e.target as Node)) onclose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onclose();
    // defer so the click that opened the menu doesn't close it
    const t = setTimeout(() => window.addEventListener("pointerdown", down, true));
    window.addEventListener("keydown", key);
    return () => {
      clearTimeout(t);
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key);
    };
  });

  function flip() {
    if (!entry) return;
    md.begin();
    entry.obj.flipX = !entry.obj.flipX;
    md.commit();
    md.touchObjects();
  }
  function remove() {
    md.begin();
    md.removeObject(id);
    md.commit();
    editor.selected = editor.selected.filter((s) => s !== id);
    onclose();
  }
  function properties() {
    editor.selected = [id];
    onclose();
  }
</script>

{#if entry}
  <div class="menu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" role="menu">
    <div class="head">
      <Thumb {image} size={32} flip={entry.obj.flipX} />
      <div class="titles">
        <div class="title">{info?.shortdescription || name || "Object"}</div>
        <div class="muted small">{name}{info?.container ? ` · container, ${info.slot_count} slots` : ""}</div>
      </div>
    </div>
    {#if info?.container}
      <div class="sect">Loot</div>
      <LootAssign {md} obj={entry.obj} ondone={onclose} />
    {/if}
    <div class="sep"></div>
    <button class="item" onclick={properties}><PixelIcon name="select" scale={2} /> Select &amp; show properties</button>
    <button class="item" onclick={flip}><PixelIcon name="flip" scale={2} /> Flip</button>
    <button class="item danger" onclick={remove}><PixelIcon name="trash" scale={2} /> Delete</button>
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 50;
    width: 340px;
    max-height: calc(100vh - 8px);
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--win-body, #151515);
    border: 2px solid #000;
    box-shadow:
      inset 0 0 0 2px #3c3c3c,
      6px 6px 0 rgba(0, 0, 0, 0.6);
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .titles {
    min-width: 0;
  }
  .title {
    color: var(--gold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sect {
    color: var(--gold);
    font-size: 13px;
    margin-top: 2px;
  }
  .sep {
    height: 2px;
    background: #000;
    margin: 2px 0;
  }
  .item {
    justify-content: flex-start;
    gap: 8px;
    background: none;
    border: none;
    box-shadow: none;
    padding: 4px 6px;
  }
  .item:hover {
    background: #2a2a2a;
  }
  .danger {
    color: var(--bad);
  }
  .small {
    font-size: 13px;
  }
</style>
