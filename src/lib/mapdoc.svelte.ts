// An open dungeon part (Tiled map) with editing helpers and undo/redo.
// Coordinates: tile x right, tile y DOWN from the top row (Tiled's order). Objects keep Tiled
// pixel coordinates; see tileAnchor()/placeObjectAt() for the game's object positioning rule.

import { game, refKey } from "./game.svelte";
import type { Doc, Obj, ObjGroup, Props, TileLayer, TileRef } from "./types";

export type LayerName = "front" | "back";

interface Snapshot {
  width: number;
  height: number;
  palette: (TileRef | null)[];
  layers: { name: string; data: number[] }[];
  groups: string;
  properties: string;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Default object groups, matching vanilla dungeon files. */
export const GROUPS = {
  objects: "objects",
  anchors: "anchors etc",
  spawns: "monsters & npcs",
  wires: "wiring - lights & guns",
  items: "items",
  mods: "mods",
};

function num(v: any): number {
  const n = typeof v === "string" ? parseFloat(v) : Number(v);
  return Number.isFinite(n) ? n : 0;
}

export function newDoc(width: number, height: number): Doc {
  const n = width * height;
  return {
    width,
    height,
    palette: [null],
    layers: [
      { name: "back", data: new Array(n).fill(0), opacity: 0.5, visible: true, properties: {} },
      { name: "front", data: new Array(n).fill(0), opacity: 1, visible: true, properties: {} },
    ],
    groups: [
      { name: GROUPS.mods, color: "#5555ff", properties: {}, visible: true, objects: [] },
      { name: GROUPS.objects, color: "#ff0000", properties: {}, visible: true, objects: [] },
      { name: GROUPS.wires, color: "#ffff00", properties: {}, visible: true, objects: [] },
      { name: GROUPS.spawns, color: "#ff0000", properties: {}, visible: true, objects: [] },
      { name: GROUPS.anchors, color: null, properties: {}, visible: true, objects: [] },
      { name: GROUPS.items, color: null, properties: {}, visible: true, objects: [] },
    ],
    properties: {},
    tilesets: [],
    nextObjectId: 1,
  };
}

export class MapDoc {
  path: string;
  doc = $state.raw<Doc>(null as unknown as Doc);
  /** increments on every change; views subscribe to it */
  rev = $state(0);
  dirty = $state(false);
  warnings = $state<string[]>([]);
  private undoStack: Snapshot[] = [];
  private redoStack: Snapshot[] = [];
  private pending: Snapshot | null = null;
  private palIndex = new Map<string, number>();
  /** dirty tile rect since last render (null = everything) */
  dirtyRect: Rect | null | undefined = undefined;
  canUndo = $state(false);
  canRedo = $state(false);

  constructor(path: string, doc: Doc, warnings: string[] = []) {
    this.path = path;
    this.doc = doc;
    this.warnings = warnings;
    for (const name of ["back", "front"] as const) {
      if (!doc.layers.some((l) => l.name === name)) {
        doc.layers.splice(name === "back" ? 0 : doc.layers.length, 0, {
          name,
          data: new Array(doc.width * doc.height).fill(0),
          opacity: name === "back" ? 0.5 : 1,
          visible: true,
          properties: {},
        });
      }
    }
    this.reindex();
    this.dirtyRect = null;
  }

  private reindex() {
    this.palIndex.clear();
    this.doc.palette.forEach((r, i) => r && this.palIndex.set(refKey(r), i));
  }

  get width() {
    return this.doc.width;
  }
  get height() {
    return this.doc.height;
  }

  layer(name: LayerName): TileLayer {
    return this.doc.layers.find((l) => l.name === name)!;
  }

  inside(x: number, y: number) {
    return x >= 0 && y >= 0 && x < this.doc.width && y < this.doc.height;
  }

  get(name: LayerName, x: number, y: number): number {
    if (!this.inside(x, y)) return 0;
    return this.layer(name).data[y * this.doc.width + x];
  }

  ref(idx: number): TileRef | null {
    return this.doc.palette[idx] ?? null;
  }

  intern(ref: TileRef | null): number {
    if (!ref) return 0;
    const k = refKey(ref);
    let i = this.palIndex.get(k);
    if (i === undefined) {
      i = this.doc.palette.length;
      this.doc.palette.push({ ...ref });
      this.palIndex.set(k, i);
    }
    return i;
  }

  // ── undo ──
  private snap(): Snapshot {
    const d = this.doc;
    return {
      width: d.width,
      height: d.height,
      palette: d.palette.slice(),
      layers: d.layers.map((l) => ({ name: l.name, data: l.data.slice() })),
      groups: JSON.stringify(d.groups),
      properties: JSON.stringify(d.properties),
    };
  }

  private restore(s: Snapshot) {
    const d = this.doc;
    d.width = s.width;
    d.height = s.height;
    d.palette = s.palette.slice();
    for (const l of s.layers) {
      const t = d.layers.find((x) => x.name === l.name);
      if (t) t.data = l.data.slice();
    }
    d.groups = JSON.parse(s.groups);
    d.properties = JSON.parse(s.properties);
    this.reindex();
  }

  /** Start an undoable edit (a stroke, a move…). Safe to call repeatedly. */
  begin() {
    if (!this.pending) this.pending = this.snap();
  }

  /** Finish the edit started with begin(). */
  commit() {
    if (!this.pending) return;
    this.undoStack.push(this.pending);
    if (this.undoStack.length > 200) this.undoStack.shift();
    this.redoStack = [];
    this.pending = null;
    this.syncUndo();
  }

  /** One-shot undoable change. */
  edit(fn: () => void, rect: Rect | null = null) {
    this.begin();
    fn();
    this.commit();
    this.touch(rect);
  }

  undo() {
    const s = this.undoStack.pop();
    if (!s) return;
    this.redoStack.push(this.snap());
    this.restore(s);
    this.syncUndo();
    this.touch(null);
  }

  redo() {
    const s = this.redoStack.pop();
    if (!s) return;
    this.undoStack.push(this.snap());
    this.restore(s);
    this.syncUndo();
    this.touch(null);
  }

  private syncUndo() {
    this.canUndo = this.undoStack.length > 0;
    this.canRedo = this.redoStack.length > 0;
  }

  /** Marks a tile rect (or everything) as changed. */
  touch(rect: Rect | null, modified = true) {
    const r = this.dirtyRect;
    if (rect === null) this.dirtyRect = null; // everything
    else if (r === undefined) this.dirtyRect = { ...rect };
    else if (r !== null) {
      const x0 = Math.min(r.x, rect.x),
        y0 = Math.min(r.y, rect.y);
      const x1 = Math.max(r.x + r.w, rect.x + rect.w),
        y1 = Math.max(r.y + r.h, rect.y + rect.h);
      this.dirtyRect = { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    if (modified) this.dirty = true;
    this.rev++;
  }

  /** Objects changed (block layers untouched). */
  touchObjects() {
    this.dirty = true;
    this.rev++;
  }

  markSaved() {
    this.dirty = false;
  }

  // ── tiles ──
  set(name: LayerName, x: number, y: number, idx: number): boolean {
    if (!this.inside(x, y)) return false;
    const l = this.layer(name);
    const i = y * this.doc.width + x;
    if (l.data[i] === idx) return false;
    l.data[i] = idx;
    this.touch({ x, y, w: 1, h: 1 });
    return true;
  }

  fillRect(name: LayerName, r: Rect, idx: number) {
    for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) if (this.inside(x, y)) this.layer(name).data[y * this.doc.width + x] = idx;
    this.touch(r);
  }

  flood(name: LayerName, x: number, y: number, idx: number) {
    if (!this.inside(x, y)) return;
    const l = this.layer(name);
    const w = this.doc.width;
    const target = l.data[y * w + x];
    if (target === idx) return;
    const stack = [[x, y]];
    let x0 = x,
      y0 = y,
      x1 = x,
      y1 = y;
    while (stack.length) {
      const [cx, cy] = stack.pop()!;
      if (!this.inside(cx, cy) || l.data[cy * w + cx] !== target) continue;
      l.data[cy * w + cx] = idx;
      x0 = Math.min(x0, cx);
      y0 = Math.min(y0, cy);
      x1 = Math.max(x1, cx);
      y1 = Math.max(y1, cy);
      stack.push([cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]);
    }
    this.touch({ x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 });
  }

  // ── objects ──
  group(name: string): ObjGroup {
    let g = this.doc.groups.find((g) => g.name === name);
    if (!g) {
      g = { name, color: null, properties: {}, visible: true, objects: [] };
      this.doc.groups.push(g);
    }
    return g;
  }

  allObjects(): { group: ObjGroup; obj: Obj }[] {
    return this.doc.groups.flatMap((group) => group.objects.map((obj) => ({ group, obj })));
  }

  findObject(id: number): { group: ObjGroup; obj: Obj } | undefined {
    return this.allObjects().find((o) => o.obj.id === id);
  }

  nextId(): number {
    const max = Math.max(0, ...this.allObjects().map((o) => o.obj.id));
    this.doc.nextObjectId = Math.max(this.doc.nextObjectId, max + 1);
    return this.doc.nextObjectId++;
  }

  addObject(groupName: string, o: Omit<Obj, "id">): Obj {
    const obj: Obj = { ...o, id: this.nextId() };
    this.group(groupName).objects.push(obj);
    this.touchObjects();
    return obj;
  }

  removeObject(id: number) {
    for (const g of this.doc.groups) g.objects = g.objects.filter((o) => o.id !== id);
    this.touchObjects();
  }

  /** Merged properties of an object as the game sees them (object > tile > group). */
  objectProps(group: ObjGroup, o: Obj): Props {
    return { ...group.properties, ...(o.tile ? (game.props(this.ref(o.tile)) ?? {}) : {}), ...o.properties };
  }

  /** The tile the game anchors a tile-object at, in editor coordinates. */
  tileAnchor(group: ObjGroup, o: Obj): { x: number; y: number } {
    const p = this.objectProps(group, o);
    const ix = Math.trunc(num(p.imagePositionX) / 8);
    const iy = Math.trunc(num(p.imagePositionY) / 8);
    const px = Math.trunc(o.x / 8) - ix;
    const py = Math.trunc(o.y / 8) + iy;
    // world y = height - py; editor row = height - 1 - worldY = py - 1
    return { x: px, y: py - 1 };
  }

  /** Tiled pixel position that anchors a tile with these props at editor tile (x, y). */
  static placeAt(props: Props, x: number, y: number): { x: number; y: number } {
    const ix = Math.trunc(num(props.imagePositionX) / 8);
    const iy = Math.trunc(num(props.imagePositionY) / 8);
    return { x: (x + ix) * 8, y: (y + 1 - iy) * 8 };
  }

  /** Objects whose drawn image / rect covers editor pixel (px, py), topmost first. */
  objectsAt(px: number, py: number, imgSize: (o: Obj) => [number, number]): { group: ObjGroup; obj: Obj }[] {
    const hits: { group: ObjGroup; obj: Obj }[] = [];
    for (const e of this.allObjects()) {
      if (!e.group.visible) continue;
      const o = e.obj;
      if (o.tile) {
        const [w, h] = imgSize(o);
        if (px >= o.x && px < o.x + w && py >= o.y - h && py < o.y) hits.push(e);
      } else if (o.polyline) {
        for (const p of o.polyline) {
          const qx = o.x + p[0],
            qy = o.y + p[1];
          if (Math.abs(px - qx) <= 5 && Math.abs(py - qy) <= 5) {
            hits.push(e);
            break;
          }
        }
      } else if (px >= o.x && px < o.x + Math.max(8, o.width) && py >= o.y && py < o.y + Math.max(8, o.height)) hits.push(e);
    }
    return hits.reverse();
  }

  // ── map size ──
  /** Resizes the map; (dx, dy) is where the old content's top-left lands in the new map. */
  resize(w: number, h: number, dx: number, dy: number) {
    this.begin();
    const d = this.doc;
    for (const l of d.layers) {
      const nd = new Array(w * h).fill(0);
      for (let y = 0; y < d.height; y++)
        for (let x = 0; x < d.width; x++) {
          const nx = x + dx,
            ny = y + dy;
          if (nx >= 0 && ny >= 0 && nx < w && ny < h) nd[ny * w + nx] = l.data[y * d.width + x];
        }
      l.data = nd;
    }
    for (const g of d.groups)
      for (const o of g.objects) {
        o.x += dx * 8;
        o.y += dy * 8;
      }
    d.width = w;
    d.height = h;
    this.commit();
    this.touch(null);
  }
}
