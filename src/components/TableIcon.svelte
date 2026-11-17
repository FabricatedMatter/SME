<script lang="ts">
  // A loot table's icon: one of the items it gives, with a small chest badge so it doesn't look
  // like a single item. A plain chest when nothing in it has an icon.
  import { tableIcon } from "../lib/loot";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Thumb from "./Thumb.svelte";

  let { name, size = 24, own = false }: { name: string; size?: number; own?: boolean } = $props();
  const icon = $derived(tableIcon(name));
</script>

<span class="ticon" style:width="{size}px" style:height="{size}px">
  {#if icon}
    <Thumb image={icon} {size} />
    <span class="badge"><PixelIcon name="chest" scale={1} color={own ? "var(--gold)" : "var(--muted)"} /></span>
  {:else}
    <PixelIcon name="chest" scale={size >= 24 ? 2 : 1} color={own ? "var(--gold)" : "var(--muted)"} />
  {/if}
</span>

<style>
  .ticon {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }
  .badge {
    position: absolute;
    right: -3px;
    bottom: -3px;
    display: grid;
    background: #101010;
    padding: 1px;
    line-height: 0;
  }
</style>
