<script lang="ts">
  import { onMount } from "svelte";
  import { editor, isAnchorBrush } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import { onImagesLoaded, getImage } from "../lib/images";
  import { GROUPS, MapDoc, type Rect } from "../lib/mapdoc.svelte";
  import { MapRenderer } from "../lib/render/mapRenderer";
  import type { Obj, TileRef } from "../lib/types";

  let { md }: { md: MapDoc } = $props();

  let wrap: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let size = $state({ w: 800, h: 600 });
  let pan = $state({ x: 40, y: 40 });
  let hover = $state<{ x: number; y: number } | null>(null);
  let hoverPx = { x: 0, y: 0 };
  const renderer = $derived(new MapRenderer(md));

  // drag state
  type Drag =
    | { kind: "pan"; sx: number; sy: number; px: number; py: number }
    | { kind: "paint"; last: { x: number; y: number }; idx: number; layer: "front" | "back" }
    | { kind: "shape"; tool: "line" | "rect"; start: { x: number; y: number } }
    | { kind: "move"; start: { x: number; y: number }; orig: Map<number, [number, number]>; moved: boolean }
    | { kind: "select"; start: { x: number; y: number } }
    | { kind: "spawn"; start: { x: number; y: number } }
    | { kind: "eraseobj" }
    | { kind: "anchors"; last: string };
  let drag = $state.raw<Drag | null>(null);
  let shapeEnd = $state<{ x: number; y: number } | null>(null);
  let tileSel = $state<Rect | null>(null);
  let wirePts = $state<{ x: number; y: number }[]>([]);
  let clipboard: { w: number; h: number; front: (TileRef | null)[]; back: (TileRef | null)[]; objects: { group: string; obj: Obj; ref: TileRef | null }[] } | null = null;
  let pasting = $state(false);
  let spaceDown = false;

  const zoom = $derived(editor.zoom);

  function toTile(ev: { clientX: number; clientY: number }) {
    const r = canvas.getBoundingClientRect();
    const mx = (ev.clientX - r.left - pan.x) / zoom;
    const my = (ev.clientY - r.top - pan.y) / zoom;
    hoverPx = { x: mx, y: my };
    return { x: Math.floor(mx / 8), y: Math.floor(my / 8) };
  }

  const paintLayer = () => (editor.layer === "back" ? "back" : "front") as "front" | "back";

  function brushIdx(): number {
    return md.intern(editor.brush?.ref ?? null);
  }

  function line(a: { x: number; y: number }, b: { x: number; y: number }, fn: (x: number, y: number) => void) {
    let x0 = a.x,
      y0 = a.y;
    const dx = Math.abs(b.x - x0),
      dy = -Math.abs(b.y - y0);
    const sx = x0 < b.x ? 1 : -1,
      sy = y0 < b.y ? 1 : -1;
    let err = dx + dy;
    for (;;) {
      fn(x0, y0);
      if (x0 === b.x && y0 === b.y) break;
      const e2 = 2 * err;
      if (e2 >= dy) {
        err += dy;
        x0 += sx;
      }
      if (e2 <= dx) {
        err += dx;
        y0 += sy;
      }
    }
  }

  function rectOf(a: { x: number; y: number }, b: { x: number; y: number }): Rect {
    const x = Math.min(a.x, b.x),
      y = Math.min(a.y, b.y);
    return { x, y, w: Math.abs(a.x - b.x) + 1, h: Math.abs(a.y - b.y) + 1 };
  }

  const objSize = (o: Obj) => renderer.objectSize(o);

  function placeObjectAt(t: { x: number; y: number }) {
    const b = editor.brush;
    if (!b) return;
    const anchor = isAnchorBrush(b);
    const props = b.props;
    const pos = MapDoc.placeAt(props, t.x, t.y);
    const img = b.image ? getImage(b.image) : null;
    const w = img ? img.width : 8,
      h = img ? img.height : 8;
    md.begin();
    const o = md.addObject(anchor ? GROUPS.anchors : GROUPS.objects, {
      tile: md.intern(b.ref),
      flipX: anchor ? false : editor.flip,
      x: pos.x,
      y: pos.y,
      width: w,
      height: h,
      name: "",
      type: "",
      properties: {},
      visible: true,
    });
    md.commit();
    if (!anchor) editor.selected = [o.id];
  }

  /** wire nodes of every placed object, in editor tile coordinates */
  const nodes = $derived.by(() => {
    void md.rev;
    const out: { x: number; y: number; input: boolean }[] = [];
    for (const { group, obj } of md.allObjects()) {
      if (!obj.tile) continue;
      const p = md.objectProps(group, obj);
      const info = p.object ? game.objects[String(p.object)] : null;
      if (!info) continue;
      const a = md.tileAnchor(group, obj);
      for (const [list, input] of [
        [info.input_nodes, true],
        [info.output_nodes, false],
      ] as const)
        for (const n of list) out.push({ x: a.x + (obj.flipX ? -n[0] : n[0]), y: a.y - n[1], input });
    }
    return out;
  });

  function finishWire() {
    if (wirePts.length >= 2) {
      const p0 = wirePts[0];
      md.begin();
      md.addObject(GROUPS.wires, {
        tile: null,
        flipX: false,
        x: p0.x * 8 + 4,
        y: p0.y * 8 + 4,
        width: 0,
        height: 0,
        polyline: wirePts.map((p) => [(p.x - p0.x) * 8, (p.y - p0.y) * 8]),
        name: "",
        type: "",
        properties: {},
        visible: true,
      });
      md.commit();
    }
    wirePts = [];
  }

  function onDown(ev: PointerEvent) {
    canvas.setPointerCapture(ev.pointerId);
    const t = toTile(ev);
    if (ev.button === 1 || ev.button === 2 || spaceDown) {
      if (ev.button === 2 && editor.tool === "wire" && wirePts.length) {
        finishWire();
        return;
      }
      drag = { kind: "pan", sx: ev.clientX, sy: ev.clientY, px: pan.x, py: pan.y };
      return;
    }
    if (ev.button !== 0) return;
    if (pasting && clipboard) {
      paste(t);
      if (!ev.shiftKey) pasting = false;
      return;
    }
    const tool = editor.tool;
    const objLayer = editor.layer === "objects";
    if (tool === "pick") {
      const hit = md.objectsAt(hoverPx.x, hoverPx.y, objSize)[0];
      const ref = objLayer && hit?.obj.tile ? md.ref(hit.obj.tile) : md.ref(md.get(paintLayer(), t.x, t.y));
      const b = game.brush(ref) ?? (hit?.obj.tile ? game.brush(md.ref(hit.obj.tile)) : undefined);
      if (b) editor.setBrush(b);
      return;
    }
    if (tool === "select") {
      const hit = md.objectsAt(hoverPx.x, hoverPx.y, objSize)[0];
      if (hit) {
        const id = hit.obj.id;
        if (ev.shiftKey) editor.selected = editor.selected.includes(id) ? editor.selected.filter((x) => x !== id) : [...editor.selected, id];
        else if (!editor.selected.includes(id)) editor.selected = [id];
        const orig = new Map<number, [number, number]>();
        for (const sid of editor.selected) {
          const e = md.findObject(sid);
          if (e) orig.set(sid, [e.obj.x, e.obj.y]);
        }
        drag = { kind: "move", start: t, orig, moved: false };
      } else {
        editor.selected = [];
        drag = { kind: "select", start: t };
        tileSel = { x: t.x, y: t.y, w: 1, h: 1 };
      }
      return;
    }
    if (tool === "wire") {
      wirePts = [...wirePts, t];
      if (ev.detail >= 2) finishWire();
      return;
    }
    if (tool === "spawn") {
      drag = { kind: "spawn", start: t };
      shapeEnd = t;
      return;
    }
    if (tool === "erase" && objLayer) {
      md.begin();
      drag = { kind: "eraseobj" };
      eraseObjAt();
      return;
    }
    const b = editor.brush;
    if (tool !== "erase" && !b) return;
    if (tool === "pencil" && b && (b.kind === "object" || (objLayer && isAnchorBrush(b)))) {
      placeObjectAt(t);
      if (isAnchorBrush(b)) drag = { kind: "anchors", last: `${t.x},${t.y}` };
      return;
    }
    const idx = tool === "erase" ? 0 : brushIdx();
    if (tool === "fill") {
      md.edit(() => md.flood(paintLayer(), t.x, t.y, idx));
      return;
    }
    if (tool === "line" || tool === "rect") {
      drag = { kind: "shape", tool, start: t };
      shapeEnd = t;
      return;
    }
    md.begin();
    md.set(paintLayer(), t.x, t.y, idx);
    drag = { kind: "paint", last: t, idx, layer: paintLayer() };
  }

  function eraseObjAt() {
    const hit = md.objectsAt(hoverPx.x, hoverPx.y, objSize)[0];
    if (hit) md.removeObject(hit.obj.id);
  }

  function onMove(ev: PointerEvent) {
    const t = toTile(ev);
    if (!hover || hover.x !== t.x || hover.y !== t.y) hover = t;
    if (!drag) return;
    switch (drag.kind) {
      case "pan":
        pan = { x: drag.px + ev.clientX - drag.sx, y: drag.py + ev.clientY - drag.sy };
        break;
      case "paint": {
        const d = drag;
        line(d.last, t, (x, y) => md.set(d.layer, x, y, d.idx));
        d.last = t;
        break;
      }
      case "shape":
      case "spawn":
        shapeEnd = t;
        break;
      case "select":
        tileSel = rectOf(drag.start, t);
        break;
      case "eraseobj":
        eraseObjAt();
        break;
      case "anchors": {
        const k = `${t.x},${t.y}`;
        if (k !== drag.last) {
          drag.last = k;
          if (!md.objectsAt(t.x * 8 + 4, t.y * 8 + 4, objSize).some((h) => renderer.isMarkerObj(h.obj) && md.ref(h.obj.tile!)?.index === editor.brush?.ref.index))
            placeObjectAt(t);
        }
        break;
      }
      case "move": {
        const dx = t.x - drag.start.x,
          dy = t.y - drag.start.y;
        if (dx || dy || drag.moved) {
          if (!drag.moved) md.begin();
          drag.moved = true;
          for (const [id, [ox, oy]] of drag.orig) {
            const e = md.findObject(id);
            if (e) {
              e.obj.x = ox + dx * 8;
              e.obj.y = oy + dy * 8;
            }
          }
          md.touchObjects();
        }
        break;
      }
    }
  }

  function onUp(ev: PointerEvent) {
    const t = toTile(ev);
    const d = drag;
    drag = null;
    if (!d) return;
    if (d.kind === "paint" || d.kind === "eraseobj") md.commit();
    else if (d.kind === "move" && d.moved) md.commit();
    else if (d.kind === "shape") {
      const idx = editor.tool === "erase" ? 0 : brushIdx();
      const layer = paintLayer();
      md.begin();
      if (d.tool === "rect") md.fillRect(layer, rectOf(d.start, t), idx);
      else line(d.start, t, (x, y) => md.set(layer, x, y, idx));
      md.commit();
      shapeEnd = null;
    } else if (d.kind === "spawn") {
      const r = rectOf(d.start, t);
      const p = { ...editor.spawnBrush };
      const group = p.item ? GROUPS.items : p.npc || p.monster || p.stagehand ? GROUPS.spawns : GROUPS.anchors;
      md.begin();
      const o = md.addObject(group, {
        tile: null,
        flipX: false,
        x: r.x * 8,
        y: r.y * 8,
        width: r.w * 8,
        height: r.h * 8,
        name: "",
        type: "",
        properties: p,
        visible: true,
      });
      md.commit();
      editor.selected = [o.id];
      shapeEnd = null;
    } else if (d.kind === "select" && tileSel) {
      // select objects inside the rectangle too
      const r = tileSel;
      editor.selected = md
        .allObjects()
        .filter(({ group, obj }) => {
          const a = obj.tile ? md.tileAnchor(group, obj) : { x: Math.floor(obj.x / 8), y: Math.floor(obj.y / 8) };
          return a.x >= r.x && a.x < r.x + r.w && a.y >= r.y && a.y < r.y + r.h;
        })
        .map((e) => e.obj.id);
      if (r.w === 1 && r.h === 1 && !editor.selected.length) tileSel = null;
    }
  }

  function onWheel(ev: WheelEvent) {
    ev.preventDefault();
    const r = canvas.getBoundingClientRect();
    const cx = ev.clientX - r.left,
      cy = ev.clientY - r.top;
    const old = zoom;
    const nz = Math.max(0.5, Math.min(12, ev.deltaY < 0 ? (old < 1 ? old * 2 : old + 1) : old <= 1 ? old / 2 : old - 1));
    pan = { x: cx - ((cx - pan.x) * nz) / old, y: cy - ((cy - pan.y) * nz) / old };
    editor.zoom = nz;
  }

  export function centerOn(x: number, y: number) {
    pan = { x: size.w / 2 - (x * 8 + 4) * zoom, y: size.h / 2 - (y * 8 + 4) * zoom };
    hover = { x, y };
  }

  export function fit() {
    const z = Math.max(0.5, Math.min(8, Math.floor(Math.min((size.w - 40) / (md.width * 8), (size.h - 40) / (md.height * 8)) * 2) / 2));
    editor.zoom = z;
    pan = { x: (size.w - md.width * 8 * z) / 2, y: (size.h - md.height * 8 * z) / 2 };
  }

  // ── clipboard ──
  export function copy(cut = false) {
    const r = tileSel;
    if (!r) return false;
    const grab = (name: "front" | "back") => {
      const out: (TileRef | null)[] = [];
      for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) out.push(md.ref(md.get(name, x, y)));
      return out;
    };
    const objects = md
      .allObjects()
      .filter((e) => editor.selected.includes(e.obj.id))
      .map((e) => ({ group: e.group.name, obj: JSON.parse(JSON.stringify(e.obj)) as Obj, ref: e.obj.tile ? md.ref(e.obj.tile) : null }));
    for (const o of objects) {
      o.obj.x -= r.x * 8;
      o.obj.y -= r.y * 8;
    }
    clipboard = { w: r.w, h: r.h, front: grab("front"), back: grab("back"), objects };
    if (cut) {
      md.begin();
      md.fillRect("front", r, 0);
      md.fillRect("back", r, 0);
      for (const o of objects) md.removeObject(o.obj.id);
      md.commit();
      editor.selected = [];
    }
    editor.toast(`${cut ? "Cut" : "Copied"} ${r.w}×${r.h}${objects.length ? ` + ${objects.length} objects` : ""}`);
    return true;
  }

  export function startPaste() {
    if (clipboard) pasting = true;
  }

  function paste(t: { x: number; y: number }) {
    const c = clipboard!;
    md.begin();
    for (let y = 0; y < c.h; y++)
      for (let x = 0; x < c.w; x++) {
        const i = y * c.w + x;
        // empty cells in the clipboard don't overwrite
        if (c.front[i]) md.set("front", t.x + x, t.y + y, md.intern(c.front[i]));
        if (c.back[i]) md.set("back", t.x + x, t.y + y, md.intern(c.back[i]));
      }
    const ids: number[] = [];
    for (const o of c.objects) {
      const n = md.addObject(o.group, { ...JSON.parse(JSON.stringify(o.obj)), tile: o.ref ? md.intern(o.ref) : null, x: o.obj.x + t.x * 8, y: o.obj.y + t.y * 8 });
      ids.push(n.id);
    }
    md.commit();
    editor.selected = ids;
  }

  export function deleteSelection() {
    if (editor.selected.length) {
      md.begin();
      for (const id of editor.selected) md.removeObject(id);
      md.commit();
      editor.selected = [];
    } else if (tileSel) {
      md.begin();
      md.fillRect(paintLayer(), tileSel, 0);
      md.commit();
    }
  }

  export function cancel() {
    pasting = false;
    wirePts = [];
    tileSel = null;
    shapeEnd = null;
    drag = null;
  }

  export function finishWirePublic() {
    finishWire();
  }

  // ── drawing ──
  let raf = 0;
  function requestDraw() {
    if (!raf) raf = requestAnimationFrame(draw);
  }

  function draw() {
    raf = 0;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== Math.round(size.w * dpr) || canvas.height !== Math.round(size.h * dpr)) {
      canvas.width = Math.round(size.w * dpr);
      canvas.height = Math.round(size.h * dpr);
    }
    renderer.update();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.setTransform(zoom * dpr, 0, 0, zoom * dpr, pan.x * dpr, pan.y * dpr);
    const s = editor.show;
    renderer.draw(ctx, {
      showBack: s.back,
      showFront: s.front,
      showObjects: s.objects,
      showMarkers: s.markers,
      showWires: s.wires,
      showGrid: s.grid && zoom >= 2,
      activeLayer: editor.layer,
      preview: s.preview,
    });
    if (s.preview) return;
    // selection highlight
    for (const id of editor.selected) {
      const e = md.findObject(id);
      if (!e) continue;
      const o = e.obj;
      ctx.strokeStyle = "#ffcf2a";
      ctx.lineWidth = 1 / zoom;
      if (o.tile) {
        const [w, h] = objSize(o);
        ctx.strokeRect(o.x, o.y - h, w, h);
        const a = md.tileAnchor(e.group, o);
        ctx.fillStyle = "rgba(255,207,42,0.5)";
        ctx.fillRect(a.x * 8 + 2, a.y * 8 + 2, 4, 4);
      } else if (o.polyline) {
        for (const p of o.polyline) ctx.strokeRect(o.x + p[0] - 3, o.y + p[1] - 3, 6, 6);
      } else ctx.strokeRect(o.x - 1, o.y - 1, Math.max(8, o.width) + 2, Math.max(8, o.height) + 2);
    }
    if (tileSel) {
      ctx.setLineDash([2, 2]);
      ctx.strokeStyle = "#66c8ff";
      ctx.lineWidth = 1 / zoom;
      ctx.strokeRect(tileSel.x * 8, tileSel.y * 8, tileSel.w * 8, tileSel.h * 8);
      ctx.setLineDash([]);
    }
    // wire tool: nodes + pending wire
    if (editor.tool === "wire") {
      for (const n of nodes) {
        ctx.fillStyle = n.input ? "rgba(255,72,72,0.9)" : "rgba(107,226,74,0.9)";
        ctx.fillRect(n.x * 8 + 2, n.y * 8 + 2, 4, 4);
      }
    }
    if (wirePts.length) {
      ctx.strokeStyle = "#ffcf2a";
      ctx.lineWidth = 1;
      ctx.beginPath();
      wirePts.forEach((p, i) => (i ? ctx.lineTo(p.x * 8 + 4, p.y * 8 + 4) : ctx.moveTo(p.x * 8 + 4, p.y * 8 + 4)));
      if (hover) ctx.lineTo(hover.x * 8 + 4, hover.y * 8 + 4);
      ctx.stroke();
    }
    // hover / ghost
    if (hover) {
      const b = editor.brush;
      const tool = editor.tool;
      if (pasting && clipboard) {
        ctx.strokeStyle = "#66c8ff";
        ctx.strokeRect(hover.x * 8, hover.y * 8, clipboard.w * 8, clipboard.h * 8);
      } else if (shapeEnd && drag && (drag.kind === "shape" || drag.kind === "spawn")) {
        const st = drag.start;
        ctx.fillStyle = "rgba(255,207,42,0.25)";
        if (drag.kind === "shape" && drag.tool === "line") line(st, shapeEnd, (x, y) => ctx.fillRect(x * 8, y * 8, 8, 8));
        else {
          const r = rectOf(st, shapeEnd);
          ctx.fillRect(r.x * 8, r.y * 8, r.w * 8, r.h * 8);
        }
      } else if (tool === "pencil" && b && (b.kind === "object" || (editor.layer === "objects" && isAnchorBrush(b))) && b.image) {
        const img = getImage(b.image);
        if (img) {
          const pos = MapDoc.placeAt(b.props, hover.x, hover.y);
          ctx.globalAlpha = 0.6;
          if (editor.flip && !isAnchorBrush(b)) {
            ctx.save();
            ctx.translate(pos.x + img.width, pos.y - img.height);
            ctx.scale(-1, 1);
            ctx.drawImage(img, 0, 0);
            ctx.restore();
          } else ctx.drawImage(img, pos.x, pos.y - img.height);
          ctx.globalAlpha = 1;
        }
      }
      ctx.strokeStyle = tool === "erase" ? "#ff4848" : "rgba(255,255,255,0.8)";
      ctx.lineWidth = 1 / zoom;
      ctx.strokeRect(hover.x * 8, hover.y * 8, 8, 8);
    }
  }

  $effect(() => {
    // redraw on any relevant change
    void md.rev;
    void zoom;
    void pan.x;
    void pan.y;
    void hover;
    void size.w;
    void size.h;
    void JSON.stringify(editor.show);
    void editor.layer;
    void editor.selected;
    void editor.tool;
    void editor.brush;
    void editor.flip;
    void tileSel;
    void shapeEnd;
    void wirePts;
    void pasting;
    void game.version;
    requestDraw();
  });

  $effect(() => {
    void renderer;
    renderer.invalidate();
    requestDraw();
  });

  onMount(() => {
    const ro = new ResizeObserver(() => {
      size = { w: wrap.clientWidth, h: wrap.clientHeight };
    });
    ro.observe(wrap);
    size = { w: wrap.clientWidth, h: wrap.clientHeight };
    const off = onImagesLoaded(() => {
      renderer.invalidate();
      requestDraw();
    });
    const kd = (e: KeyboardEvent) => {
      if (e.code === "Space" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) spaceDown = true;
    };
    const ku = (e: KeyboardEvent) => {
      if (e.code === "Space") spaceDown = false;
    };
    window.addEventListener("keydown", kd);
    window.addEventListener("keyup", ku);
    requestAnimationFrame(() => fit());
    return () => {
      ro.disconnect();
      off();
      window.removeEventListener("keydown", kd);
      window.removeEventListener("keyup", ku);
    };
  });
</script>

<div class="wrap" bind:this={wrap}>
  <canvas
    bind:this={canvas}
    style:width="{size.w}px"
    style:height="{size.h}px"
    class:pan={!!drag && drag.kind === "pan"}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointerleave={() => (hover = null)}
    onwheel={onWheel}
    oncontextmenu={(e) => e.preventDefault()}
  ></canvas>
  {#if hover}
    <div class="coords">
      x {hover.x} · y {hover.y} <span class="muted">(world y {md.height - 1 - hover.y})</span> · {Math.round(zoom * 100)}%
    </div>
  {/if}
  {#if pasting}<div class="hint">Click to paste (shift: keep pasting) · Esc to cancel</div>{/if}
  {#if editor.tool === "wire"}<div class="hint">Click wire nodes (red = input, green = output) · double-click or right-click to finish · Esc to cancel</div>{/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: rgba(0, 0, 0, 0.35);
  }
  canvas {
    display: block;
    cursor: crosshair;
    image-rendering: pixelated;
  }
  canvas.pan {
    cursor: grabbing;
  }
  .coords {
    position: absolute;
    left: 8px;
    bottom: 6px;
    background: rgba(0, 0, 0, 0.7);
    padding: 1px 6px;
    font-size: 13px;
    pointer-events: none;
  }
  .hint {
    position: absolute;
    left: 50%;
    top: 8px;
    transform: translateX(-50%);
    background: rgba(0, 0, 0, 0.75);
    border: 2px solid var(--btn-gold-mid);
    padding: 2px 10px;
    pointer-events: none;
    white-space: nowrap;
  }
</style>
