<script lang="ts">
  import { assetUrl } from "../lib/api";

  // A pixel-perfect preview of a game image, scaled by an integer factor to fit `size`.
  // `image#x,y,w,h` shows only that rectangle (one frame of a sprite sheet).
  let { image, size = 32, flip = false, title = "" }: { image: string | null; size?: number; flip?: boolean; title?: string } = $props();
  let nat = $state<[number, number] | null>(null);
  let failed = $state(false);

  const crop = $derived.by(() => {
    const m = /^(.*)#(\d+),(\d+),(\d+),(\d+)$/.exec(image ?? "");
    return m ? { src: m[1], x: +m[2], y: +m[3], w: +m[4], h: +m[5] } : null;
  });
  const dims = $derived<[number, number] | null>(crop ? [crop.w, crop.h] : nat);
  const scale = $derived(dims ? Math.max(1, Math.floor(Math.min(size / dims[0], size / dims[1]))) || 1 : 1);
  const fits = $derived(dims ? dims[0] * scale <= size && dims[1] * scale <= size : true);

  // cropped frames are drawn onto a canvas
  let canvas = $state<HTMLCanvasElement>();
  $effect(() => {
    const c = crop,
      cv = canvas;
    if (!c || !cv) return;
    failed = false;
    const img = new Image();
    img.onload = () => {
      const ctx = cv.getContext("2d")!;
      ctx.clearRect(0, 0, cv.width, cv.height);
      ctx.drawImage(img, c.x, c.y, c.w, c.h, 0, 0, c.w, c.h);
    };
    img.onerror = () => (failed = true);
    img.src = assetUrl(c.src);
  });
  // shrink (not pixel-exact) only when even 1× doesn't fit
  const shown = $derived(dims ? (fits ? [dims[0] * scale, dims[1] * scale] : (() => {
    const k = Math.min(size / dims[0], size / dims[1]);
    return [dims[0] * k, dims[1] * k];
  })()) : null);
</script>

<span class="thumb" style:width="{size}px" style:height="{size}px" {title}>
  {#if image && !failed && crop}
    <canvas
      bind:this={canvas}
      width={crop.w}
      height={crop.h}
      style:width="{shown?.[0]}px"
      style:height="{shown?.[1]}px"
      style:transform={flip ? "scaleX(-1)" : ""}
    ></canvas>
  {:else if image && !failed}
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
  img,
  canvas {
    image-rendering: pixelated;
    object-fit: contain;
  }
  .none {
    color: var(--dim);
  }
</style>
