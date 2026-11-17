// Global editor state: the open project, its files, open maps, tools and actions.

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { SvelteSet } from "svelte/reactivity";
import { api } from "./api";
import { game, type Brush } from "./game.svelte";
import { MapDoc, newDoc, GROUPS } from "./mapdoc.svelte";
import { buildPatches, parseSpawns, type ParsedSpawns, type SpawnRule } from "./spawn";
import type { Problem, ProjectInfo, Props } from "./types";

export type Tool = "pan" | "pencil" | "line" | "rect" | "fill" | "erase" | "pick" | "select" | "spawn" | "wire";
export type EditLayer = "front" | "back" | "objects";

export type View =
  | { kind: "welcome" }
  | { kind: "mod" }
  | { kind: "dungeon"; path: string }
  | { kind: "map"; path: string; dungeon?: string }
  | { kind: "loot"; path: string; pool?: string; back?: { path: string; dungeon?: string } }
  | { kind: "spawn" }
  | { kind: "test" };

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "good";
}

const LS = {
  get<T>(k: string, d: T): T {
    try {
      const v = localStorage.getItem("sme." + k);
      return v ? (JSON.parse(v) as T) : d;
    } catch {
      return d;
    }
  },
  set(k: string, v: any) {
    try {
      localStorage.setItem("sme." + k, JSON.stringify(v));
    } catch {
      /* ignore */
    }
  },
};

/** Joins a dungeon file's folder with a relative part file. */
export function joinRel(base: string, rel: string): string {
  const parts = base.split("/");
  parts.pop();
  for (const s of rel.split("/")) {
    if (s === "..") parts.pop();
    else if (s && s !== ".") parts.push(s);
  }
  return parts.join("/");
}

export function partFiles(part: any): string[] {
  const f = part?.def?.[1];
  return typeof f === "string" ? [f] : Array.isArray(f) ? f.filter((x: any) => typeof x === "string") : [];
}

class Editor {
  project = $state.raw<ProjectInfo | null>(null);
  view = $state<View>({ kind: "welcome" });

  /** editable JSON files (dungeons, pools, metadata), keyed by project-relative path */
  files = $state<Record<string, any>>({});
  dirtyFiles = new SvelteSet<string>();
  deletedFiles = new SvelteSet<string>();

  maps = $state.raw<Record<string, MapDoc>>({});
  spawnRules = $state<SpawnRule[]>([]);
  spawnParsed: ParsedSpawns = { rules: [], foreign: {}, untouchable: new Set() };
  spawnDirty = $state(false);

  tool = $state<Tool>("pencil");
  layer = $state<EditLayer>("front");
  brush = $state.raw<Brush | null>(null);
  /** properties for the rectangle spawn tool (npc / monster / stagehand / item…) */
  spawnBrush = $state<Props>({ npc: "human", typeName: "villager" });
  flip = $state(false);
  zoom = $state(3);
  show = $state({ back: true, front: true, objects: true, markers: true, wires: true, grid: true, preview: false });
  selected = $state<number[]>([]);

  problems = $state<Problem[]>([]);
  checking = $state(false);
  toasts = $state<Toast[]>([]);
  favorites = $state<string[]>(LS.get("favorites", []));
  recentBrushes = $state<string[]>(LS.get("recent", []));
  goto = $state<{ path: string; x: number; y: number; n: number } | null>(null);

  // ── helpers ──
  toast(text: string, kind: Toast["kind"] = "info") {
    const id = Date.now() + Math.random();
    this.toasts.push({ id, text, kind });
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), kind === "error" ? 9000 : 4000);
  }

  get dirty(): boolean {
    return this.dirtyFiles.size > 0 || this.deletedFiles.size > 0 || this.spawnDirty || Object.values(this.maps).some((m) => m.dirty);
  }

  get dungeonPaths(): string[] {
    return Object.keys(this.files).filter((p) => p.endsWith(".dungeon")).sort();
  }

  /** Vanilla dungeons the mod extends with JSON patches (`<name>.dungeon.patch`), with the
   *  parts those patches add. Older mods often add their maps to e.g. tundramicrodungeons. */
  get dungeonPatches(): { path: string; target: string; parts: any[]; anchors: string[] }[] {
    return (this.project?.patches ?? [])
      .filter((f) => f.path.endsWith(".dungeon.patch") && !this.deletedFiles.has(f.path))
      .map((f) => {
        const ops: any[] = Array.isArray(f.json) ? f.json.flat() : [];
        const adds = ops.filter((o) => o && o.op === "add" && o.value !== undefined);
        return {
          path: f.path,
          target: f.path.split("/").pop()!.replace(/\.dungeon\.patch$/, ""),
          parts: adds.filter((o) => /^\/parts\/(-|\d+)$/.test(o.path)).map((o) => o.value),
          anchors: adds.filter((o) => /^\/metadata\/anchor\/(-|\d+)$/.test(o.path)).map((o) => o.value),
        };
      });
  }

  /** Map files under dungeons/ that neither a .dungeon nor a dungeon patch refers to. */
  get unlistedMaps(): string[] {
    const used = new Set<string>();
    for (const d of this.dungeonPaths) for (const part of this.files[d]?.parts ?? []) for (const f of partFiles(part)) used.add(joinRel(d, f));
    for (const dp of this.dungeonPatches) for (const part of dp.parts) for (const f of partFiles(part)) used.add(joinRel(dp.path, f));
    return (this.project?.maps ?? []).filter((p) => !used.has(p) && !this.maps[p]?.dirty && !this.deletedFiles.has(p));
  }

  get poolPaths(): string[] {
    return Object.keys(this.files).filter((p) => p.endsWith(".treasurepools")).sort();
  }

  /** All dungeon names defined in the project. */
  get dungeonNames(): string[] {
    return this.dungeonPaths.map((p) => this.files[p]?.metadata?.name).filter(Boolean);
  }

  /** Pools defined in the project: name → file. */
  get projectPools(): Record<string, string> {
    const out: Record<string, string> = {};
    for (const p of this.poolPaths) for (const k of Object.keys(this.files[p] ?? {})) out[k] = p;
    return out;
  }

  setBrush(b: Brush | null) {
    this.brush = b;
    if (!b) return;
    if (this.tool !== "pencil" && this.tool !== "line" && this.tool !== "rect" && this.tool !== "fill") this.tool = "pencil";
    if (b.kind === "object") this.layer = "objects";
    else if (this.layer === "objects" && !isAnchorBrush(b)) this.layer = "front";
    this.recentBrushes = [b.key, ...this.recentBrushes.filter((k) => k !== b.key)].slice(0, 24);
    LS.set("recent", this.recentBrushes);
  }

  toggleFavorite(key: string) {
    this.favorites = this.favorites.includes(key) ? this.favorites.filter((k) => k !== key) : [...this.favorites, key];
    LS.set("favorites", this.favorites);
  }

  // ── project lifecycle ──
  async confirmDiscard(): Promise<boolean> {
    if (!this.dirty) return true;
    return confirm("You have unsaved changes. Discard them?");
  }

  private loadInfo(info: ProjectInfo) {
    this.project = info;
    const files: Record<string, any> = {};
    for (const f of [...info.dungeons, ...info.treasure]) files[f.path] = f.json;
    files[info.metadata_file] = info.metadata ?? null;
    this.files = files;
    this.dirtyFiles.clear();
    this.deletedFiles.clear();
    this.maps = {};
    this.spawnParsed = parseSpawns(info.patches);
    this.spawnRules = this.spawnParsed.rules;
    this.spawnDirty = false;
    this.problems = [];
    for (const e of info.errors) this.toast(e, "error");
  }

  async openFolder(path?: string) {
    if (!(await this.confirmDiscard())) return;
    const dir = path ?? ((await openDialog({ directory: true, title: "Open a mod folder" })) as string | null);
    if (!dir) return;
    await this.openPath(dir);
  }

  async openDungeonFile() {
    if (!(await this.confirmDiscard())) return;
    const f = (await openDialog({
      title: "Open a .dungeon or Tiled map",
      filters: [{ name: "Starbound dungeon, map or .pak", extensions: ["dungeon", "json", "pak"] }],
    })) as string | null;
    if (!f) return;
    await this.openPath(f);
  }

  /** Unpacks a .pak (e.g. a Steam workshop contents.pak) into a new folder and opens it. */
  async importPak(pak?: string) {
    if (!(await this.confirmDiscard())) return;
    const install = game.status?.install;
    const workshop = install ? install.replace(/[/\\]common[/\\][^/\\]+[/\\]?$/, "/workshop/content/211820") : undefined;
    const f =
      pak ??
      ((await openDialog({
        title: "Choose a .pak to unpack (workshop mods are a contents.pak)",
        defaultPath: workshop,
        filters: [{ name: "Starbound pak", extensions: ["pak"] }],
      })) as string | null);
    if (!f) return;
    const parent = (await openDialog({ directory: true, title: "Where should the unpacked mod folder go?" })) as string | null;
    if (!parent) return;
    try {
      const info = await api.importPak(f, parent);
      await game.load();
      this.loadInfo(info);
      this.view = { kind: "mod" };
      this.toast(`Unpacked to ${info.root}`, "good");
    } catch (e) {
      this.toast(String(e), "error");
    }
  }

  async openPath(p: string) {
    if (p.toLowerCase().endsWith(".pak")) return this.importPak(p);
    try {
      const info = await api.openProject(p);
      await game.load();
      this.loadInfo(info);
      // open the requested file directly if it was a file
      const rel = p.startsWith(info.root) ? p.slice(info.root.length).replace(/^[/\\]/, "").replace(/\\/g, "/") : "";
      if (rel.endsWith(".dungeon")) this.view = { kind: "dungeon", path: rel };
      else if (rel.endsWith(".json")) await this.openMap(rel);
      else this.view = info.mode === "mod" ? { kind: "mod" } : this.dungeonPaths[0] ? { kind: "dungeon", path: this.dungeonPaths[0] } : { kind: "mod" };
    } catch (e) {
      this.toast(String(e), "error");
    }
  }

  async createMod(spec: { dir: string; name: string; friendly_name: string; author: string; description: string }) {
    const info = await api.createProject(spec);
    await game.load();
    this.loadInfo(info);
    this.view = { kind: "mod" };
  }

  async close() {
    if (!(await this.confirmDiscard())) return;
    await api.closeProject();
    this.project = null;
    this.files = {};
    this.maps = {};
    this.view = { kind: "welcome" };
    await game.load();
  }

  // ── files ──
  setFile(path: string, json: any) {
    this.files[path] = json;
    this.dirtyFiles.add(path);
    this.deletedFiles.delete(path);
  }

  markDirty(path: string) {
    this.dirtyFiles.add(path);
  }

  removeFile(path: string) {
    delete this.files[path];
    this.dirtyFiles.delete(path);
    this.deletedFiles.add(path);
  }

  // ── deleting (files go away on save) ──
  private leaveIf(pred: (v: View) => boolean) {
    if (pred(this.view)) this.view = { kind: "mod" };
  }

  /** Deletes a map file and closes it. */
  deleteMap(path: string) {
    this.removeFile(path);
    const { [path]: _, ...rest } = this.maps;
    this.maps = rest;
    this.leaveIf((v) => v.kind === "map" && v.path === path);
  }

  /** Map files a dungeon's parts use (relative to the project). */
  dungeonMaps(dpath: string): string[] {
    return [...new Set((this.files[dpath]?.parts ?? []).flatMap((p: any) => partFiles(p).map((f) => joinRel(dpath, f))))] as string[];
  }

  /** Deletes a dungeon, its part maps and its name from spawn rules. */
  deleteDungeon(dpath: string) {
    const name = this.files[dpath]?.metadata?.name;
    for (const mp of this.dungeonMaps(dpath)) this.deleteMap(mp);
    this.removeFile(dpath);
    if (name) {
      for (const r of this.spawnRules) r.dungeons = r.dungeons.filter((n) => n !== name);
      this.spawnRules = this.spawnRules.filter((r) => r.dungeons.length);
      this.spawnDirty = true;
    }
    this.leaveIf((v) => (v.kind === "dungeon" && v.path === dpath) || (v.kind === "map" && v.dungeon === dpath));
  }

  /** Removes a part from its dungeon; its map file is deleted unless another part uses it. */
  deletePart(dpath: string, partName: string) {
    const d = this.files[dpath];
    if (!d) return;
    const part = d.parts.find((p: any) => p.name === partName);
    if (!part) return;
    d.parts = d.parts.filter((p: any) => p !== part);
    d.metadata.anchor = (d.metadata.anchor ?? []).filter((a: string) => a !== partName);
    for (const f of partFiles(part)) {
      const mp = joinRel(dpath, f);
      if (!d.parts.some((p: any) => partFiles(p).some((x) => joinRel(dpath, x) === mp))) this.deleteMap(mp);
    }
    this.markDirty(dpath);
  }

  /** Deletes a dungeon patch file and the maps only it uses. */
  deletePatch(path: string) {
    const dp = this.dungeonPatches.find((p) => p.path === path);
    for (const part of dp?.parts ?? []) for (const f of partFiles(part)) this.deleteMap(joinRel(path, f));
    this.removeFile(path);
  }

  /** Deletes a .treasurepools file. Returns the table names that were in it. */
  deleteLootFile(path: string): string[] {
    const names = Object.keys(this.files[path] ?? {});
    this.removeFile(path);
    this.leaveIf((v) => v.kind === "loot" && v.path === path);
    return names;
  }

  /** Containers in open maps that use any of these loot tables. */
  containersUsing(names: string[]): number {
    let n = 0;
    for (const md of Object.values(this.maps))
      for (const { obj } of md.allObjects()) {
        const p = String(obj.properties.parameters ?? "");
        if (names.some((x) => p.includes(`"${x}"`))) n++;
      }
    return n;
  }

  async openMap(path: string, dungeon?: string) {
    if (!this.maps[path]) {
      try {
        const exists = await api.fileExists(path);
        const md = exists ? await api.readMap(path).then((r) => new MapDoc(path, r.doc, r.warnings)) : new MapDoc(path, newDoc(40, 30));
        if (!exists) md.dirty = true;
        this.maps = { ...this.maps, [path]: md };
        const serious = md.warnings.filter((w) => !/flipped tiles are ignored/.test(w));
        if (serious.length) this.toast(`${path}: ${serious.length} import note(s), see the Map panel`, "info");
      } catch (e) {
        this.toast(String(e), "error");
        return;
      }
    }
    this.selected = [];
    this.view = { kind: "map", path, dungeon };
  }

  /** Creates a new microdungeon (.dungeon + one part map with starter anchors). */
  newDungeon(name: string, opts: { width: number; height: number; anchors: boolean }) {
    const folder = `dungeons/microdungeons/${name}`;
    const dpath = `${folder}/${name}.dungeon`;
    if (this.files[dpath]) throw new Error(`${dpath} already exists`);
    const part = `${name}1`;
    const mpath = `${folder}/${part}.json`;
    this.setFile(dpath, {
      metadata: {
        name,
        species: "generic",
        rules: [],
        anchor: [part],
        gravity: 80,
        maxRadius: 1000000,
        maxParts: 100,
        protected: false,
      },
      parts: [{ name: part, rules: [["maxSpawnCount", [1]]], def: ["tmx", `${part}.json`] }],
    });
    const md = new MapDoc(mpath, newDoc(opts.width, opts.height));
    if (opts.anchors) addStarterAnchors(md);
    md.dirty = true;
    this.maps = { ...this.maps, [mpath]: md };
    return dpath;
  }

  addPart(dpath: string, partName: string) {
    const d = this.files[dpath];
    const file = `${partName}.json`;
    const mpath = joinRel(dpath, file);
    d.parts.push({ name: partName, rules: [["maxSpawnCount", [1]]], def: ["tmx", file] });
    d.metadata.anchor = [...(d.metadata.anchor ?? []), partName];
    this.markDirty(dpath);
    if (!this.maps[mpath]) {
      const md = new MapDoc(mpath, newDoc(40, 30));
      addStarterAnchors(md);
      md.dirty = true;
      this.maps = { ...this.maps, [mpath]: md };
    }
    return mpath;
  }

  // ── saving ──
  async saveAll(): Promise<boolean> {
    if (!this.project) return false;
    try {
      for (const p of [...this.deletedFiles]) {
        await api.deleteFile(p);
        this.deletedFiles.delete(p);
      }
      for (const p of [...this.dirtyFiles]) {
        if (this.files[p] !== undefined && this.files[p] !== null) await api.writeJson(p, $state.snapshot(this.files[p]));
        this.dirtyFiles.delete(p);
      }
      for (const m of Object.values(this.maps)) {
        if (!m.dirty) continue;
        await api.writeMap(m.path, m.doc);
        m.markSaved();
      }
      if (this.spawnDirty) await this.saveSpawns();
      const info = await api.rescanProject();
      this.project = info;
      this.toast("Saved", "good");
      void this.check(true);
      return true;
    } catch (e) {
      this.toast("Save failed: " + e, "error");
      return false;
    }
  }

  private async saveSpawns() {
    const out = buildPatches($state.snapshot(this.spawnRules) as SpawnRule[], this.spawnParsed);
    for (const [p, ops] of Object.entries(out)) {
      if (ops.length) await api.writeJson(p, ops);
      else await api.deleteFile(p);
    }
    const info = await api.rescanProject();
    this.spawnParsed = parseSpawns(info.patches);
    this.spawnDirty = false;
  }

  // ── validation ──
  async check(quiet = false) {
    if (!this.project) return;
    this.checking = true;
    try {
      const patches = Object.entries(buildPatches($state.snapshot(this.spawnRules) as SpawnRule[], this.spawnParsed))
        .filter(([, ops]) => ops.length)
        .map(([path, json]) => ({ path, json }));
      for (const f of this.project.patches) if (!patches.some((p) => p.path === f.path) && this.spawnParsed.untouchable.has(f.path)) patches.push(f);
      // maps: open ones from memory, the rest from disk
      const mapPaths = new Set<string>();
      for (const d of this.dungeonPaths) for (const part of this.files[d]?.parts ?? []) for (const f of partFiles(part)) mapPaths.add(joinRel(d, f));
      for (const dp of this.dungeonPatches) for (const part of dp.parts) for (const f of partFiles(part)) mapPaths.add(joinRel(dp.path, f));
      const maps = [];
      for (const p of mapPaths) {
        if (this.maps[p]) maps.push({ path: p, doc: this.maps[p].doc });
        else if (await api.fileExists(p)) {
          try {
            maps.push({ path: p, doc: (await api.readMap(p)).doc });
          } catch (e) {
            this.problems.push({ severity: "error", file: p, message: String(e), x: null, y: null });
          }
        }
      }
      const files = [...new Set([...Object.keys(this.files), ...Object.keys(this.maps)])];
      this.problems = await api.validate({
        dungeons: this.dungeonPaths.map((path) => ({ path, json: $state.snapshot(this.files[path]) })),
        maps,
        pools: this.poolPaths.map((path) => ({ path, json: $state.snapshot(this.files[path]) })),
        patches,
        files,
      });
      if (!quiet) {
        const e = this.problems.filter((p) => p.severity === "error").length;
        this.toast(e ? `${e} error(s) found` : this.problems.length ? `No errors, ${this.problems.length} warning(s)` : "All good!", e ? "error" : "good");
      }
    } catch (e) {
      this.toast(String(e), "error");
    } finally {
      this.checking = false;
    }
  }
}

/** Anchors placed as objects (overlaying blocks) rather than as tile-layer cells. */
export function isAnchorBrush(b: Brush | null): boolean {
  if (!b || b.kind !== "marker" || !b.ref.tileset.endsWith("/miscellaneous.json")) return false;
  return [3, 4, 5, 6, 7, 12, 13, 14, 15, 16, 17, 20, 21, 22, 23].includes(b.ref.index);
}

/** A row of "needs solid" anchors at the bottom and "needs air" anchors above them. */
function addStarterAnchors(md: MapDoc) {
  const misc = "/tilesets/packed/miscellaneous.json";
  const solid = md.intern({ tileset: misc, index: 5 });
  const air = md.intern({ tileset: misc, index: 4 });
  const g = GROUPS.anchors;
  const y = md.height - 1;
  for (const x of [2, Math.floor(md.width / 2), md.width - 3]) {
    md.addObject(g, { tile: solid, flipX: false, x: x * 8, y: (y + 1) * 8, width: 8, height: 8, name: "", type: "", properties: {}, visible: true });
    md.addObject(g, { tile: air, flipX: false, x: x * 8, y: y * 8, width: 8, height: 8, name: "", type: "", properties: {}, visible: true });
  }
}

export const editor = new Editor();
