// Draws a MapDoc. Block layers are rendered with joint tiling into cached offscreen canvases
// (1 canvas pixel = 1 game pixel) and only the dirty region is redrawn after an edit; objects,
// liquids, markers and wires are cheap and drawn every frame on top.

import { game, MISC, MARKERS } from "../game.svelte";
import { getHued, getImage } from "../images";
import type { MapDoc, Rect } from "../mapdoc.svelte";
import type { Obj, ObjGroup, Props } from "../types";
import { cellPieces, type CellLook, type DrawPiece, type TileQuery } from "./tiling";

interface Look extends CellLook {
  liquid: string | null;
  /** image to draw instead of tiling (unknown/meta materials, markers, stray object tiles) */
  image: string | null;
  marker: boolean;
  /** opacity for the marker image (faint for air / keep-background) */
  markerAlpha: number;
}

const NONE: Look = { mat: null, hue: 0, color: 0, mod: null, modHue: 0, liquid: null, image: null, marker: false, markerAlpha: 1 };

/** Opacity of marker tiles by miscellaneous.json index: air & keep-background are faint. */
const MARKER_ALPHA: Record<number, number> = { 0: 0.2, 1: 0.12, 11: 0.2, 8: 0.3, 9: 0.3, 10: 0.3 };

function n(v: any): number {
  const x = typeof v === "string" ? parseFloat(v) : Number(v);
  return Number.isFinite(x) ? x : 0;
}

export interface ViewOpts {
  showBack: boolean;
  showFront: boolean;
  showObjects: boolean;
  showMarkers: boolean;
  showWires: boolean;
  showGrid: boolean;
  activeLayer: "front" | "back" | "objects";
  preview: boolean;
}

export class MapRenderer {
  md: MapDoc;
  back = document.createElement("canvas");
  front = document.createElement("canvas");
  private looks: Look[] = [];
  private lookVersion = -1;

  constructor(md: MapDoc) {
    this.md = md;
  }

  look(idx: number): Look {
    if (this.lookVersion !== game.version) {
      this.looks = [];
      this.lookVersion = game.version;
    }
    let l = this.looks[idx];
    if (l) return l;
    const ref = this.md.ref(idx);
    const p = game.props(ref);
    if (!ref || !p) l = NONE;
    else {
      // "surface" brushes become the planet's own block; preview them as dirt
      const mat = p.material ? String(p.material) : p.surface !== undefined ? "dirt" : null;
      const hasProfile = mat ? !!game.material(mat) : false;
      const isMarker = ref.tileset === MISC;
      l = {
        markerAlpha: isMarker ? (MARKER_ALPHA[ref.index] ?? 0.9) : 0.9,
        mat: hasProfile ? mat : null,
        hue: n(p.hueshift),
        color: n(p.colorVariant),
        mod: p.mod && game.mod(String(p.mod)) ? String(p.mod) : null,
        modHue: n(p.modhueshift),
        liquid: p.liquid ? String(p.liquid) : null,
        image: (hasProfile && !isMarker) || p.liquid ? null : game.image(ref),
        marker: isMarker || (!!mat && !hasProfile),
      };
    }
    this.looks[idx] = l;
    return l;
  }

  private query: TileQuery = {
    cell: (x, y, back) => {
      if (!this.md.inside(x, y)) return null;
      const l = this.look(this.md.get(back ? "back" : "front", x, y));
      return l.mat || l.mod ? l : null;
    },
    material: (name) => game.material(name),
    mod: (name) => game.mod(name),
  };

  /** Re-renders whatever the document marked dirty. Returns true if anything was drawn. */
  update(): boolean {
    const md = this.md;
    const W = md.width * 8,
      H = md.height * 8;
    let rect: Rect | null | undefined = md.dirtyRect;
    if (this.back.width !== W || this.back.height !== H) {
      this.back.width = this.front.width = W;
      this.back.height = this.front.height = H;
      rect = null;
    }
    if (rect === undefined) return false;
    md.dirtyRect = undefined;
    // Edge pieces overhang into neighbouring cells, and a change alters the joins of its
    // neighbours too, so the cleared/redrawn region has to reach 2 tiles past the edit;
    // otherwise erasing leaves the old overhangs ("outlines") behind.
    const grow = 2;
    const r = rect
      ? (() => {
          const x0 = Math.max(0, rect.x - grow),
            y0 = Math.max(0, rect.y - grow);
          const x1 = Math.min(md.width, rect.x + rect.w + grow),
            y1 = Math.min(md.height, rect.y + rect.h + grow);
          return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
        })()
      : { x: 0, y: 0, w: md.width, h: md.height };
    // front changes alter background shadows, so always redraw both layers
    this.renderLayer(this.back, true, r);
    this.renderLayer(this.front, false, r);
    return true;
  }

  /** Full redraw (e.g. after images finished loading). */
  invalidate() {
    this.md.dirtyRect = null;
  }

  private renderLayer(cv: HTMLCanvasElement, back: boolean, r: Rect) {
    const ctx = cv.getContext("2d")!;
    ctx.imageSmoothingEnabled = false;
    const px = r.x * 8,
      py = r.y * 8,
      pw = r.w * 8,
      ph = r.h * 8;
    ctx.save();
    ctx.beginPath();
    ctx.rect(px, py, pw, ph);
    ctx.clip();
    ctx.clearRect(px, py, pw, ph);
    const pieces: DrawPiece[] = [];
    const pad = 2;
    const fallback: { x: number; y: number; image: string; hue: number }[] = [];
    for (let y = r.y - pad; y < r.y + r.h + pad; y++)
      for (let x = r.x - pad; x < r.x + r.w + pad; x++) {
        if (!this.md.inside(x, y)) continue;
        const l = this.look(this.md.get(back ? "back" : "front", x, y));
        if (l.mat || l.mod) cellPieces(this.query, x, y, back, pieces);
        if (!l.mat && l.image && !l.marker) fallback.push({ x, y, image: l.image, hue: l.hue });
      }
    pieces.sort((a, b) => a.z - b.z);
    for (const p of pieces) {
      const img = getHued("a:" + p.piece.texture, p.hue);
      if (!img) continue;
      ctx.drawImage(img, p.sx, p.sy, p.piece.size[0], p.piece.size[1], p.px, p.py, p.piece.size[0], p.piece.size[1]);
    }
    for (const f of fallback) {
      const img = getHued(f.image, f.hue);
      if (img) ctx.drawImage(img, f.x * 8, f.y * 8, 8, 8);
    }
    if (back) {
      // the game tints background tiles with rendering.config's backgroundLayerColor;
      // darkening with source-atop keeps transparent pixels transparent
      const c = game.catalog?.background_color ?? [128, 128, 128];
      ctx.globalCompositeOperation = "source-atop";
      ctx.fillStyle = `rgba(0,0,0,${1 - (c[0] + c[1] + c[2]) / 3 / 255})`;
      ctx.fillRect(px, py, pw, ph);
    }
    ctx.restore();
  }

  // ── per-frame drawing ──

  objectImage(o: Obj): string | null {
    return o.tile ? game.image(this.md.ref(o.tile)) : null;
  }

  objectSize(o: Obj): [number, number] {
    const ref = this.objectImage(o);
    const img = ref ? getImage(ref) : null;
    if (o.width > 0 && o.height > 0) return [o.width, o.height];
    return img ? [img.width, img.height] : [8, 8];
  }

  isMarkerObj(o: Obj): boolean {
    const r = o.tile ? this.md.ref(o.tile) : null;
    return !!r && r.tileset === MISC;
  }

  /** Draws the whole map in map-pixel space (caller sets the transform). */
  draw(ctx: CanvasRenderingContext2D, v: ViewOpts) {
    const md = this.md;
    const W = md.width * 8,
      H = md.height * 8;
    ctx.imageSmoothingEnabled = false;
    // map area
    ctx.fillStyle = "rgba(10, 12, 30, 0.85)";
    ctx.fillRect(0, 0, W, H);

    if (v.showBack) ctx.drawImage(this.back, 0, 0);
    const objs = md.allObjects().filter((e) => e.group.visible && e.obj.visible);
    if (v.showObjects) for (const e of objs) if (e.obj.tile && !this.isMarkerObj(e.obj)) this.drawTileObject(ctx, e.obj);
    if (v.showFront) {
      ctx.globalAlpha = v.activeLayer === "back" && !v.preview ? 0.35 : 1;
      ctx.drawImage(this.front, 0, 0);
      ctx.globalAlpha = 1;
    }
    this.drawLiquids(ctx);
    if (v.preview) return;
    if (v.showMarkers) {
      this.drawTileMarkers(ctx, v);
      for (const e of objs) if (e.obj.tile && this.isMarkerObj(e.obj)) this.drawTileObject(ctx, e.obj, 0.9);
      for (const e of objs) if (!e.obj.tile && !e.obj.polyline) this.drawRectObject(ctx, e.group, e.obj);
    }
    if (v.showWires) for (const e of objs) if (e.obj.polyline && !e.obj.tile) this.drawWire(ctx, e.obj);
    if (v.showGrid) this.drawGrid(ctx);
    ctx.strokeStyle = "rgba(255,207,42,0.8)";
    ctx.lineWidth = 1;
    ctx.strokeRect(-0.5, -0.5, W + 1, H + 1);
  }

  drawTileObject(ctx: CanvasRenderingContext2D, o: Obj, alpha = 1) {
    const ref = this.objectImage(o);
    const img = ref ? getImage(ref) : null;
    const [w, h] = this.objectSize(o);
    if (!img) {
      ctx.fillStyle = "rgba(255,72,72,0.5)";
      ctx.fillRect(o.x, o.y - h, w, h);
      return;
    }
    ctx.globalAlpha = alpha;
    if (o.flipX) {
      ctx.save();
      ctx.translate(o.x + w, o.y - h);
      ctx.scale(-1, 1);
      ctx.drawImage(img, 0, 0, w, h);
      ctx.restore();
    } else ctx.drawImage(img, o.x, o.y - h, w, h);
    ctx.globalAlpha = 1;
  }

  private drawLiquids(ctx: CanvasRenderingContext2D) {
    const md = this.md;
    for (const name of ["back", "front"] as const) {
      const data = md.layer(name).data;
      for (let i = 0; i < data.length; i++) {
        if (!data[i]) continue;
        const l = this.look(data[i]);
        if (!l.liquid) continue;
        ctx.fillStyle = game.liquidColor(l.liquid);
        ctx.fillRect((i % md.width) * 8, Math.floor(i / md.width) * 8, 8, 8);
      }
    }
  }

  private drawTileMarkers(ctx: CanvasRenderingContext2D, v: ViewOpts) {
    const md = this.md;
    for (const name of ["back", "front"] as const) {
      if ((name === "back" && !v.showBack) || (name === "front" && !v.showFront)) continue;
      const data = md.layer(name).data;
      const layerAlpha = name === "back" ? 0.6 : 1;
      for (let i = 0; i < data.length; i++) {
        if (!data[i]) continue;
        const l = this.look(data[i]);
        if (!l.marker || !l.image) continue;
        ctx.globalAlpha = layerAlpha * l.markerAlpha;
        const img = getImage(l.image);
        if (img) ctx.drawImage(img, (i % md.width) * 8, Math.floor(i / md.width) * 8, 8, 8);
      }
    }
    ctx.globalAlpha = 1;
  }

  static rectLabel(p: Props): { text: string; color: string } {
    if (p.npc) return { text: `NPC ${p.npc}${p.typeName ? " · " + p.typeName : ""}`, color: "#6be24a" };
    if (p.monster) return { text: `Monster ${p.monster}`, color: "#ff4848" };
    if (p.stagehand) return { text: `Stagehand ${p.stagehand}`, color: "#66c8ff" };
    if (p.item) return { text: `Item ${p.item}${p.count ? " ×" + p.count : ""}`, color: "#ffcf2a" };
    if (p.liquid) return { text: `Liquid ${p.liquid}`, color: "#66c8ff" };
    if (p.vehicle) return { text: `Vehicle ${p.vehicle}`, color: "#ff9b2a" };
    if (p.dungeonid) return { text: `Dungeon id ${p.dungeonid}`, color: "#c6f" };
    if (p.material || p.front || p.back) return { text: `Material ${p.material ?? p.front ?? p.back}`, color: "#a2a2a2" };
    if (p.mod) return { text: `Mod ${p.mod}`, color: "#8c4" };
    const k = Object.keys(p).filter((k) => !k.startsWith("//"));
    return { text: k.length ? k.join(", ") : "brush", color: "#a2a2a2" };
  }

  private drawRectObject(ctx: CanvasRenderingContext2D, g: ObjGroup, o: Obj) {
    const p = { ...g.properties, ...o.properties };
    const { text, color } = MapRenderer.rectLabel(p);
    const w = Math.max(8, o.width),
      h = Math.max(8, o.height);
    ctx.fillStyle = color + "33";
    ctx.fillRect(o.x, o.y, w, h);
    ctx.strokeStyle = color;
    ctx.lineWidth = 1;
    ctx.strokeRect(o.x + 0.5, o.y + 0.5, w - 1, h - 1);
    ctx.font = "6px monospace";
    ctx.fillStyle = color;
    ctx.save();
    ctx.beginPath();
    ctx.rect(o.x, o.y, w, h);
    ctx.clip();
    ctx.fillText(text, o.x + 2, o.y + 7);
    ctx.restore();
  }

  private drawWire(ctx: CanvasRenderingContext2D, o: Obj) {
    const pts = o.polyline!;
    ctx.strokeStyle = "#ff3b3b";
    ctx.lineWidth = 1;
    ctx.beginPath();
    pts.forEach((p, i) => (i ? ctx.lineTo(o.x + p[0], o.y + p[1]) : ctx.moveTo(o.x + p[0], o.y + p[1])));
    ctx.stroke();
    ctx.fillStyle = "#ffcf2a";
    for (const p of pts) ctx.fillRect(o.x + p[0] - 1.5, o.y + p[1] - 1.5, 3, 3);
  }

  private drawGrid(ctx: CanvasRenderingContext2D) {
    const md = this.md;
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.lineWidth = 0.25;
    ctx.beginPath();
    for (let x = 0; x <= md.width; x++) {
      ctx.moveTo(x * 8, 0);
      ctx.lineTo(x * 8, md.height * 8);
    }
    for (let y = 0; y <= md.height; y++) {
      ctx.moveTo(0, y * 8);
      ctx.lineTo(md.width * 8, y * 8);
    }
    ctx.stroke();
  }
}

export { MARKERS };
