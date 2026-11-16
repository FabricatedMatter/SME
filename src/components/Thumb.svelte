<script lang="ts">
  import { assetUrl } from "../lib/api";

  // A pixel-perfect preview of a game image, scaled by an integer factor to fit `size`.
  let { image, size = 32, flip = false, title = "" }: { image: string | null; size?: number; flip?: boolean; title?: string } = $props();
  let nat = $state<[number, number] | null>(null);
  let failed = $state(false);
  const scale = $derived(nat ? Math.max(1, Math.floor(Math.min(size / nat[0], size / nat[1]))) || 1 : 1);
  const fits = $derived(nat ? nat[0] * scale <= size && nat[1] * scale <= size : true);
</script>

<span class="thumb" style:width="{size}px" style:height="{size}px" {title}>
  {#if image && !failed}
    <img
      src={assetUrl(image)}
      alt=""
      draggable="false"
      style:transform={flip ? "scaleX(-1)" : ""}
      style:width={nat ? (fits ? nat[0] * scale : Math.min(size, nat[0])) + "px" : "auto"}
      style:max-width="{size}px"
      style:max-height="{size}px"
      onload={(e) => {
        const i = e.currentTarget as HTMLImageElement;
        nat = [i.naturalWidth, i.naturalHeight];
      }}
      onerror={() => (failed = true)}
    />
  {:else}
    <span class="none">?</span>
  {/if}
</span>

<style>
  .thumb {
    display: inline-grid;
    place-items: center;
    flex: none;
    overflow: hidden;
  }
  img {
    image-rendering: pixelated;
    object-fit: contain;
  }
  .none {
    color: var(--dim);
  }
</style>
