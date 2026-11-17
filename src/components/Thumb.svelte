<script lang="ts">
  import { assetUrl } from "../lib/api";

  // A pixel-perfect preview of a game image, scaled by an integer factor to fit `size`.
  // `image#x,y,w,h` shows only that rectangle (one frame of a sprite sheet).
  // `a@x,y|b@x,y|…` stacks several such images, each centred on its position (pixels, y up),
  // like the game's multi-part inventory icons (random-build weapons).
  let { image, size = 32, flip = false, title = "" }: { image: string | null; size?: number; flip?: boolean; title?: string } = $props();
  // remembered per image, so a reused Thumb showing a new image doesn't keep the old one's
  // failure ("?") or size
  let natOf = $state<{ image: string | null; dims: [number, number] } | null>(null);
  let failedOf = $state<string | null | undefined>(undefined);
  const nat = $derived(natOf && natOf.image === image ? natOf.dims : null);
  const failed = $derived(failedOf === image);

  type Layer = { src: string; crop: [number, number, number, number] | null; x: number; y: number };
  /** canvas-drawn images (frames of a sheet, or stacked layers); null = a plain <img> */
  const layers = $derived.by<Layer[] | null>(() => {
    if (!image || !/[#@|]/.test(image)) return null;
    return image.split("|").map((part) => {
      const [ref, pos] = part.split("@");
      const m = /^(.*)#(\d+),(\d+),(\d+),(\d+)$/.exec(ref);
      const [x, y] = pos ? pos.split(",").map(Number) : [0, 0];
      return { src: m ? m[1] : ref, crop: m ? [+m[2], +m[3], +m[4], +m[5]] : null, x, y };
    });
  });
  // a single cropped frame's size is known up front; stacks are measured once loaded
  const dims = $derived<[number, number] | null>(
    layers?.length === 1 && layers[0].crop ? [layers[0].crop[2], layers[0].crop[3]] : nat,
  );
  const scale = $derived(dims ? Math.max(1, Math.floor(Math.min(size / dims[0], size / dims[1]))) || 1 : 1);
  const fits = $derived(dims ? dims[0] * scale <= size && dims[1] * scale <= size : true);

  let canvas = $state<HTMLCanvasElement>();
  $effect(() => {
    const ls = layers,
      cv = canvas,
      forImage = image;
    if (!ls || !cv) return;
    let stale = false;
    Promise.all(
      ls.map(
        (l) =>
          new Promise<HTMLImageElement>((ok, bad) => {
            const img = new Image();
            img.onload = () => ok(img);
            img.onerror = bad;
            img.src = assetUrl(l.src);
          }),
      ),
    ).then(
      (imgs) => {
        if (stale) return;
        // each layer's rectangle, centred on its position (y up in the game, down on screen)
        const rects = ls.map((l, i) => {
          const [w, h] = l.crop ? [l.crop[2], l.crop[3]] : [imgs[i].naturalWidth, imgs[i].naturalHeight];
          return { x: l.x - w / 2, y: -l.y - h / 2, w, h };
        });
        const x0 = Math.floor(Math.min(...rects.map((r) => r.x)));
        const y0 = Math.floor(Math.min(...rects.map((r) => r.y)));
        const W = Math.ceil(Math.max(...rects.map((r) => r.x + r.w)) - x0);
        const H = Math.ceil(Math.max(...rects.map((r) => r.y + r.h)) - y0);
        cv.width = W;
        cv.height = H;
        const ctx = cv.getContext("2d")!;
        ctx.clearRect(0, 0, W, H);
        ls.forEach((l, i) => {
          const r = rects[i];
          const [sx, sy] = l.crop ? [l.crop[0], l.crop[1]] : [0, 0];
          ctx.drawImage(imgs[i], sx, sy, r.w, r.h, Math.round(r.x - x0), Math.round(r.y - y0), r.w, r.h);
        });
        natOf = { image: forImage, dims: [W, H] };
      },
      () => {
        if (!stale) failedOf = forImage;
      },
    );
    return () => (stale = true);
  });
  // shrink (not pixel-exact) only when even 1× doesn't fit
  const shown = $derived(dims ? (fits ? [dims[0] * scale, dims[1] * scale] : (() => {
    const k = Math.min(size / dims[0], size / dims[1]);
    return [dims[0] * k, dims[1] * k];
  })()) : null);
</script>

<span class="thumb" style:width="{size}px" style:height="{size}px" {title}>
  {#if image && !failed && layers}
    <canvas
      bind:this={canvas}
      style:width={shown ? `${shown[0]}px` : "0"}
      style:height={shown ? `${shown[1]}px` : "0"}
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
        natOf = { image, dims: [i.naturalWidth, i.naturalHeight] };
      }}
      onerror={() => (failedOf = image)}
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
