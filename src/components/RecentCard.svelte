<script lang="ts">
  // A welcome-screen card for a recent project: a rendered preview of its dungeons (cycling
  // through a few maps on hover), its title and how many dungeons it has.
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { game } from "../lib/game.svelte";
  import { onImagesLoaded } from "../lib/images";
  import { MapDoc } from "../lib/mapdoc.svelte";
  import { MapRenderer } from "../lib/render/mapRenderer";
  import PixelIcon from "../lib/PixelIcon.svelte";

  let { dir, onopen, onforget }: { dir: string; onopen: () => void; onforget: () => void } = $props();

  const W = 236,
    H = 132;
  let canvas = $state<HTMLCanvasElement>();
  let info = $state<{ exists: boolean; title: string; dungeons: number; maps: string[] } | null>(null);
  let docs: (MapRenderer | null)[] = [];
  let shown = $state(0);
  let timer: ReturnType<typeof setInterval> | undefined;

  const PREVIEW = { showBack: true, showFront: true, showObjects: true, showMarkers: false, showWires: false, showGrid: false, activeLayer: "front", preview: true } as const;

  async function renderer(i: number): Promise<MapRenderer | null> {
    if (docs[i] !== undefined) return docs[i];
    try {
      const r = await api.readMapIn(dir, info!.maps[i]);
      docs[i] = new MapRenderer(new MapDoc(info!.maps[i], r.doc));
    } catch {
      docs[i] = null;
    }
    return docs[i];
  }

  async function draw() {
    if (!canvas || !info?.maps.length || !game.catalog) return;
    const r = await renderer(shown);
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, W, H);
    if (!r) return;
    r.update();
    const mw = r.md.width * 8,
      mh = r.md.height * 8;
    const off = new OffscreenCanvas(mw, mh);
    const octx = off.getContext("2d")!;
    r.draw(octx as unknown as CanvasRenderingContext2D, PREVIEW);
    // integer scale when it fits (crisp pixels), otherwise shrink smoothly
    const fit = Math.min(W / mw, H / mh);
    const s = fit >= 1 ? Math.floor(fit) : fit;
    ctx.imageSmoothingEnabled = s < 1;
    const dw = mw * s,
      dh = mh * s;
    ctx.drawImage(off, Math.round((W - dw) / 2), Math.round((H - dh) / 2), Math.round(dw), Math.round(dh));
  }

  onMount(() => {
    void api.projectPreview(dir).then((p) => {
      info = p;
      void draw();
    });
    const off = onImagesLoaded(() => {
      for (const r of docs) r?.invalidate();
      void draw();
    });
    return () => {
      off();
      clearInterval(timer);
    };
  });

  // redraw once the game data arrives (the welcome screen can appear before it)
  $effect(() => {
    void game.catalog;
    void game.version;
    void draw();
  });

  function hover(on: boolean) {
    clearInterval(timer);
    if (!on || !info || info.maps.length < 2) return;
    timer = setInterval(() => {
      shown = (shown + 1) % info!.maps.length;
      void draw();
    }, 1400);
  }
</script>

<div class="card" class:missing={info && !info.exists} role="group" onmouseenter={() => hover(true)} onmouseleave={() => hover(false)}>
  <button class="open" onclick={onopen} disabled={!!info && !info.exists} title={dir}>
    <div class="preview">
      <canvas bind:this={canvas} width={W} height={H}></canvas>
      {#if info && !info.maps.length}
        <span class="empty muted">{info.exists ? "no maps yet" : "folder not found"}</span>
      {/if}
      {#if info && info.maps.length > 1}
        <span class="dots">{#each info.maps as _, i}<i class:on={i === shown}></i>{/each}</span>
      {/if}
    </div>
    <div class="text">
      <span class="title">{info?.title ?? dir.split(/[/\\]/).pop()}</span>
      <span class="muted small">
        {#if info?.exists}{info.dungeons} dungeon{info.dungeons === 1 ? "" : "s"}{#if info.maps[shown]} · {info.maps[shown].split("/").pop()}{/if}{:else if info}missing{/if}
      </span>
    </div>
  </button>
  <button class="forget flat small" title="Remove from recent" onclick={onforget}><PixelIcon name="x" scale={1} /></button>
</div>

<style>
  .card {
    position: relative;
    width: 244px;
  }
  .open {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    padding: 4px;
    width: 100%;
    text-align: left;
  }
  .preview {
    position: relative;
    height: 132px;
    background: rgba(0, 0, 0, 0.55);
    box-shadow: inset 0 0 0 2px #000;
  }
  canvas {
    display: block;
    image-rendering: pixelated;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  .dots {
    position: absolute;
    bottom: 4px;
    left: 0;
    right: 0;
    display: flex;
    justify-content: center;
    gap: 3px;
  }
  .dots i {
    width: 4px;
    height: 4px;
    background: #666;
  }
  .dots i.on {
    background: var(--gold);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 0 2px 2px;
  }
  .title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .small {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .forget {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 2px 4px;
    opacity: 0;
  }
  .card:hover .forget {
    opacity: 1;
  }
  .missing .preview {
    opacity: 0.5;
  }
</style>
