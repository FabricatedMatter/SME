<script lang="ts">
  import { onMount } from "svelte";
  import { editor, type Tool } from "../lib/editor.svelte";
  import type { MapDoc } from "../lib/mapdoc.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Inspector from "./Inspector.svelte";
  import MapCanvas from "./MapCanvas.svelte";
  import Palette from "./Palette.svelte";

  let { md }: { md: MapDoc } = $props();
  let canvas: MapCanvas | undefined = $state();

  const TOOLS: { id: Tool; icon: string; label: string; key: string }[] = [
    { id: "pencil", icon: "pencil", label: "Paint / place", key: "B" },
    { id: "line", icon: "line", label: "Line", key: "L" },
    { id: "rect", icon: "rect", label: "Rectangle", key: "R" },
    { id: "fill", icon: "bucket", label: "Fill", key: "G" },
    { id: "erase", icon: "eraser", label: "Erase", key: "E" },
    { id: "pick", icon: "picker", label: "Pick brush", key: "I" },
    { id: "select", icon: "select", label: "Select / move", key: "V" },
    { id: "spawn", icon: "npc", label: "Spawn area (NPC, monster, item…)", key: "N" },
    { id: "wire", icon: "wire", label: "Wire", key: "W" },
  ];

  $effect(() => {
    const g = editor.goto;
    if (g && g.path === md.path && canvas) {
      canvas.centerOn(g.x, g.y);
      editor.goto = null;
    }
  });

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement) return;
    const ctrl = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    if (ctrl && k === "z") {
      e.preventDefault();
      if (e.shiftKey) md.redo();
      else md.undo();
      return;
    }
    if (ctrl && k === "y") {
      e.preventDefault();
      md.redo();
      return;
    }
    if (ctrl && k === "c") {
      canvas?.copy(false);
      return;
    }
    if (ctrl && k === "x") {
      canvas?.copy(true);
      return;
    }
    if (ctrl && k === "v") {
      canvas?.startPaste();
      return;
    }
    if (ctrl) return;
    if (k === "delete" || k === "backspace") {
      canvas?.deleteSelection();
      return;
    }
    if (k === "escape") {
      canvas?.cancel();
      editor.selected = [];
      return;
    }
    if (k === "enter" && editor.tool === "wire") {
      canvas?.finishWirePublic();
      return;
    }
    const tool = TOOLS.find((x) => x.key.toLowerCase() === k);
    if (tool) {
      editor.tool = tool.id;
      if (tool.id === "spawn" || tool.id === "wire") editor.layer = "objects";
      return;
    }
    if (k === "x" || k === "tab") {
      e.preventDefault();
      editor.layer = editor.layer === "front" ? "back" : editor.layer === "back" ? "objects" : "front";
    } else if (k === "f") {
      if (editor.selected.length) {
        md.begin();
        for (const id of editor.selected) {
          const o = md.findObject(id)?.obj;
          if (o?.tile) o.flipX = !o.flipX;
        }
        md.commit();
        md.touchObjects();
      } else editor.flip = !editor.flip;
    } else if (k === "p") editor.show.preview = !editor.show.preview;
    else if (k === "+" || k === "=") editor.zoom = Math.min(12, editor.zoom + 1);
    else if (k === "-") editor.zoom = Math.max(0.5, editor.zoom > 1 ? editor.zoom - 1 : editor.zoom / 2);
    else if (k === "0") canvas?.fit();
  }

  onMount(() => {
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<div class="editor">
  <div class="toolbar">
    <div class="group">
      {#each TOOLS as t}
        <button class="tool" class:on={editor.tool === t.id} title="{t.label} ({t.key})" onclick={() => {
          editor.tool = t.id;
          if (t.id === "spawn" || t.id === "wire") editor.layer = "objects";
        }}>
          <PixelIcon name={t.icon} scale={2} />
        </button>
      {/each}
    </div>
    <div class="group layers" title="Layer to edit (Tab)">
      <button class:gold={editor.layer === "front"} onclick={() => (editor.layer = "front")}>Front</button>
      <button class:gold={editor.layer === "back"} onclick={() => (editor.layer = "back")}>Back</button>
      <button class:gold={editor.layer === "objects"} onclick={() => (editor.layer = "objects")}>Objects</button>
    </div>
    <button class="tool" class:on={editor.flip} title="Place objects facing left (F)" onclick={() => (editor.flip = !editor.flip)}>
      <PixelIcon name="flip" scale={2} />
    </button>
    <div class="group">
      <button class="tool" disabled={!md.canUndo} title="Undo (Ctrl+Z)" onclick={() => md.undo()}><PixelIcon name="undo" scale={2} /></button>
      <button class="tool" disabled={!md.canRedo} title="Redo (Ctrl+Y)" onclick={() => md.redo()}><PixelIcon name="redo" scale={2} /></button>
    </div>
    <div class="spacer"></div>
    <div class="group toggles">
      {#each [["back", "Back"], ["front", "Front"], ["objects", "Objects"], ["markers", "Markers"], ["wires", "Wires"], ["grid", "Grid"]] as [k, l]}
        <label title="Show {l.toLowerCase()}"><input type="checkbox" bind:checked={editor.show[k as keyof typeof editor.show]} />{l}</label>
      {/each}
      <button class="small" class:gold={editor.show.preview} title="Game-like preview without editor overlays (P)" onclick={() => (editor.show.preview = !editor.show.preview)}>
        <PixelIcon name="eye" scale={2} /> Preview
      </button>
      <button class="small" title="Fit map (0)" onclick={() => canvas?.fit()}>Fit</button>
    </div>
  </div>
  <div class="main">
    <MapCanvas {md} bind:this={canvas} />
    <aside class="side">
      <div class="pal"><Palette /></div>
      <div class="ins"><Inspector {md} /></div>
    </aside>
  </div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-bottom: 2px solid #000;
    background: rgba(0, 0, 0, 0.35);
    flex-wrap: wrap;
  }
  .group {
    display: flex;
    gap: 2px;
    align-items: center;
  }
  .tool {
    padding: 4px 6px;
  }
  .tool.on {
    background: var(--btn-gold-fill);
    box-shadow:
      inset 0 2px 0 var(--btn-gold-hi),
      inset 0 4px 0 var(--btn-gold-mid),
      inset 0 -2px 0 var(--btn-gold-lo);
  }
  .layers button {
    padding: 3px 10px 4px;
  }
  .toggles {
    gap: 10px;
  }
  .toggles label {
    display: flex;
    gap: 4px;
    align-items: center;
    font-size: 14px;
    color: var(--muted);
  }
  .toggles input {
    width: 18px;
    height: 18px;
  }
  .spacer {
    flex: 1;
  }
  .main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .side {
    width: 330px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-left: 2px solid #000;
    background: rgba(13, 13, 13, 0.85);
    min-height: 0;
  }
  .pal {
    flex: 3;
    min-height: 0;
    display: flex;
  }
  .ins {
    flex: 2;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-top: 2px solid #000;
  }
</style>
