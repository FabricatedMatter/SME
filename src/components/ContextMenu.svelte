<script module lang="ts">
  export interface MenuItem {
    label: string;
    icon?: string;
    danger?: boolean;
    action: () => void;
  }
</script>

<script lang="ts">
  // A small right-click menu: a list of actions at the mouse position.
  import { onMount } from "svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  let { items, x, y, title = "", onclose }: { items: MenuItem[]; x: number; y: number; title?: string; onclose: () => void } = $props();

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });
  $effect(() => {
    if (!el) return;
    const r = el.getBoundingClientRect();
    pos = { x: Math.max(4, Math.min(x, window.innerWidth - r.width - 4)), y: Math.max(4, Math.min(y, window.innerHeight - r.height - 4)) };
  });
  onMount(() => {
    const down = (e: PointerEvent) => el && !el.contains(e.target as Node) && onclose();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onclose();
    const t = setTimeout(() => window.addEventListener("pointerdown", down, true));
    window.addEventListener("keydown", key);
    return () => {
      clearTimeout(t);
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key);
    };
  });
</script>

<div class="cmenu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" role="menu">
  {#if title}<div class="title">{title}</div>{/if}
  {#each items as it}
    <button class="item" class:danger={it.danger} role="menuitem" onclick={() => {
      onclose();
      it.action();
    }}>{#if it.icon}<PixelIcon name={it.icon} scale={2} />{/if} {it.label}</button>
  {/each}
</div>

<style>
  .cmenu {
    position: fixed;
    z-index: 55;
    min-width: 200px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    background: var(--win-body, #151515);
    border: 2px solid #000;
    box-shadow:
      inset 0 0 0 2px #3c3c3c,
      6px 6px 0 rgba(0, 0, 0, 0.6);
  }
  .title {
    color: var(--gold);
    font-size: 13px;
    padding: 2px 6px 4px;
  }
  .item {
    justify-content: flex-start;
    gap: 8px;
    background: none;
    border: none;
    box-shadow: none;
    padding: 4px 8px;
  }
  .item:hover {
    background: #2a2a2a;
  }
  .danger {
    color: var(--bad);
  }
</style>
