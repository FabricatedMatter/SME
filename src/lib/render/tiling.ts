// Material "joint tiling": a port of Starbound's MaterialRenderProfile + TileDrawer
// piece matching. Each material's render template (e.g. /tiles/classicmaterialtemplate.config)
// lists texture pieces and neighbour-match rules; for every tile we collect the pieces whose
// rules match, then draw them in zLevel order. Edge pieces overhang into neighbouring tiles,
// which is what makes blocks join like in the game.

import type { MaterialInfo } from "../types";

export interface PieceDef {
  texture: string; // asset path
  size: [number, number];
  pos: [number, number];
  colorStride: [number, number];
  variantStride: [number, number];
  variants: number;
}

interface RuleEntry {
  type: string;
  inverse: boolean;
  matchHue: boolean;
  id?: number;
  propertyName?: string;
  propertyValue?: any;
}
interface Rule {
  join: "all" | "any";
  entries: RuleEntry[];
}
interface Match {
  all: boolean;
  points: { x: number; y: number; rule: Rule }[];
  pieces: { piece: PieceDef; dx: number; dy: number }[];
  sub: Match[] | null;
  requiredLayer: "foreground" | "background" | null;
  haltOnMatch: boolean;
  haltOnSubMatch: boolean;
}

export interface Profile {
  name: string;
  id: number;
  zLevel: number;
  colorVariants: number;
  lightTransparent: boolean;
  main: Match[];
  ruleProperties: Record<string, any>;
  representative: PieceDef | null;
}

function vec(v: any, d: [number, number] = [0, 0]): [number, number] {
  return Array.isArray(v) && v.length >= 2 ? [Number(v[0]), Number(v[1])] : d;
}

function relTo(base: string, rel: string): string {
  if (rel.startsWith("/")) return rel;
  const parts = base.split("/").filter(Boolean);
  parts.pop();
  for (const s of rel.split("/")) {
    if (s === "..") parts.pop();
    else if (s && s !== ".") parts.push(s);
  }
  return "/" + parts.join("/");
}

/** Builds a render profile from a template merged with the material's renderParameters. */
export function buildProfile(m: MaterialInfo, template: any): Profile | null {
  if (!template) return null;
  const spec = { ...template, ...(m.params ?? {}) };
  const texture = spec.texture ? relTo(m.path, String(spec.texture)) : "";
  const pieces = new Map<string, PieceDef>();
  for (const [name, p] of Object.entries<any>(spec.pieces ?? {})) {
    pieces.set(name, {
      texture: p.texture ? relTo(m.path, String(p.texture)) : texture,
      size: vec(p.textureSize, [8, 8]),
      pos: vec(p.texturePosition),
      colorStride: vec(p.colorStride),
      variantStride: vec(p.variantStride),
      variants: Number(p.variants ?? spec.variants ?? 1) || 1,
    });
  }
  const rules = new Map<string, Rule>();
  for (const [name, r] of Object.entries<any>(spec.rules ?? {})) {
    rules.set(name, {
      join: String(r.join ?? "all").toLowerCase() === "any" ? "any" : "all",
      entries: (r.entries ?? []).map((e: any) => ({
        type: String(e.type ?? "").toLowerCase(),
        inverse: !!e.inverse,
        matchHue: !!e.matchHue,
        id: e.id,
        propertyName: e.propertyName,
        propertyValue: e.propertyValue,
      })),
    });
  }
  const named = new Map<string, Match[]>();
  const parseList = (list: any[]): Match[] =>
    (list ?? []).map((mc: any) => {
      const all = mc.matchAllPoints !== undefined || mc.matchAnyPoints === undefined;
      const pts = mc.matchAllPoints ?? mc.matchAnyPoints ?? [];
      let sub: Match[] | null = null;
      if (typeof mc.subMatches === "string") sub = named.get(mc.subMatches) ?? null;
      else if (Array.isArray(mc.subMatches)) sub = parseList(mc.subMatches);
      return {
        all,
        points: pts
          .map((p: any) => ({ x: Number(p[0][0]), y: Number(p[0][1]), rule: rules.get(p[1])! }))
          .filter((p: any) => p.rule),
        pieces: (mc.pieces ?? [])
          .map((p: any) => ({ piece: pieces.get(p[0])!, dx: Number(p[1][0]), dy: Number(p[1][1]) }))
          .filter((p: any) => p.piece),
        sub,
        requiredLayer: mc.requiredLayer ?? null,
        haltOnMatch: !!mc.haltOnMatch,
        haltOnSubMatch: !!mc.haltOnSubMatch,
      };
    });
  for (const pair of spec.matches ?? []) named.set(pair[0], parseList(pair[1]));
  const lightTransparent = !!(spec.foregroundLightTransparent ?? spec.lightTransparent ?? false);
  return {
    name: m.name,
    id: m.id,
    zLevel: Number(spec.zLevel ?? 0),
    colorVariants: spec.multiColored ? Number(spec.colorVariants ?? 9) : 0,
    lightTransparent,
    main: named.get("main") ?? [],
    ruleProperties: spec.ruleProperties ?? {},
    representative: pieces.get(spec.representativePiece) ?? null,
  };
}

/** What a renderer needs to know about one cell of one layer. */
export interface CellLook {
  mat: string | null; // material name
  hue: number;
  color: number;
  mod: string | null;
  modHue: number;
}

export interface TileQuery {
  /** Look at (x, y) in tile coordinates with y growing downward (editor rows). */
  cell(x: number, y: number, back: boolean): CellLook | null;
  material(name: string): Profile | null;
  mod(name: string): Profile | null;
}

export interface DrawPiece {
  piece: PieceDef;
  /** Top-left draw position in editor pixels (y down). */
  px: number;
  py: number;
  sx: number;
  sy: number;
  z: number;
  hue: number;
}

function hash(x: number, y: number, l: number, salt: number): number {
  let h = (x * 374761393 + y * 668265263 + l * 2147483647 + salt * 1274126177) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return (h ^ (h >>> 16)) >>> 0;
}

function evalMatches(
  list: Match[],
  q: TileQuery,
  x: number,
  y: number,
  back: boolean,
  self: CellLook,
  isMod: boolean,
  out: { piece: PieceDef; dx: number; dy: number }[],
): boolean {
  const ok = (m: Match): boolean => {
    if (m.requiredLayer && (m.requiredLayer === "background") !== back) return false;
    if (m.points.length === 0) return true;
    let valid = m.all;
    for (const pt of m.points) {
      // rule offsets are y-up; editor rows are y-down
      const n = q.cell(x + pt.x, y - pt.y, back);
      let rv = pt.rule.join === "all";
      for (const e of pt.rule.entries) {
        let v = true;
        switch (e.type) {
          case "empty":
            v = isMod ? !n?.mod : !n?.mat;
            break;
          case "connects":
            v = !!n?.mat;
            break;
          case "shadows": {
            if (!back) v = false;
            else {
              const f = q.cell(x + pt.x, y - pt.y, false);
              const prof = f?.mat ? q.material(f.mat) : null;
              v = !!f?.mat && !!prof && !prof.lightTransparent;
            }
            break;
          }
          case "equalsself":
            if (isMod) v = !!n?.mod && n.mod === self.mod && (!e.matchHue || n.modHue === self.modHue);
            else v = !!n?.mat && n.mat === self.mat && (!e.matchHue || n.hue === self.hue);
            break;
          case "equalsid": {
            const nm = isMod ? n?.mod : n?.mat;
            const prof = nm ? (isMod ? q.mod(nm) : q.material(nm)) : null;
            v = !!prof && prof.id === e.id;
            break;
          }
          case "propertyequals": {
            const nm = isMod ? n?.mod : n?.mat;
            const prof = nm ? (isMod ? q.mod(nm) : q.material(nm)) : null;
            v = !!prof && JSON.stringify(prof.ruleProperties[e.propertyName!]) === JSON.stringify(e.propertyValue);
            break;
          }
        }
        if (e.inverse) v = !v;
        if (pt.rule.join === "all") {
          rv = rv && v;
          if (!rv) break;
        } else rv = rv || v;
      }
      if (m.all) {
        valid = valid && rv;
        if (!valid) return false;
      } else valid = valid || rv;
    }
    return valid;
  };
  let any = false;
  for (const m of list) {
    if (!ok(m)) continue;
    any = true;
    out.push(...m.pieces);
    if (m.sub && evalMatches(m.sub, q, x, y, back, self, isMod, out) && m.haltOnSubMatch) break;
    if (m.haltOnMatch) break;
  }
  return any;
}

/** All pieces to draw for the cell at (x, y). */
export function cellPieces(q: TileQuery, x: number, y: number, back: boolean, out: DrawPiece[]) {
  const c = q.cell(x, y, back);
  if (!c) return;
  const layer = back ? 1 : 0;
  const emit = (prof: Profile, isMod: boolean, hue: number, salt: number) => {
    const list: { piece: PieceDef; dx: number; dy: number }[] = [];
    evalMatches(prof.main, q, x, y, back, c, isMod, list);
    const cv = prof.colorVariants > 0 ? c.color % prof.colorVariants : 0;
    const variance = hash(x, y, layer, salt);
    const z = (isMod ? 1e12 : 0) + prof.zLevel * 1e6 + (prof.id & 0xffff) * 16 + (hue % 16);
    for (const { piece, dx, dy } of list) {
      const v = variance % piece.variants;
      out.push({
        piece,
        px: x * 8 + dx,
        py: (y + 1) * 8 - dy - piece.size[1],
        sx: piece.pos[0] + piece.variantStride[0] * v + piece.colorStride[0] * cv,
        sy: piece.pos[1] + piece.variantStride[1] * v + piece.colorStride[1] * cv,
        z,
        hue,
      });
    }
  };
  if (c.mat) {
    const p = q.material(c.mat);
    if (p) emit(p, false, c.hue, 1);
  }
  if (c.mod) {
    const p = q.mod(c.mod);
    if (p) emit(p, true, c.modHue, 2);
  }
}
