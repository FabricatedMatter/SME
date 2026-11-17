// Shapes shared with the Rust backend (see src-tauri/src/*.rs).

export type Props = Record<string, any>;

export interface TileRef {
  tileset: string;
  index: number;
}

export interface TileLayer {
  name: string;
  data: number[];
  opacity: number;
  visible: boolean;
  properties: Props;
}

export interface Obj {
  id: number;
  tile?: number | null;
  flipX: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
  polyline?: [number, number][] | null;
  name: string;
  type: string;
  properties: Props;
  visible: boolean;
}

export interface ObjGroup {
  name: string;
  color?: string | null;
  properties: Props;
  visible: boolean;
  objects: Obj[];
}

export interface TilesetRef {
  path: string;
  source?: string | null;
  tilecountHint: number;
}

export interface Doc {
  width: number;
  height: number;
  palette: (TileRef | null)[];
  layers: TileLayer[];
  groups: ObjGroup[];
  properties: Props;
  tilesets: TilesetRef[];
  nextObjectId: number;
}

export interface Imported {
  doc: Doc;
  warnings: string[];
}

export interface TileInfo {
  props: Props;
  image: string | null;
}

export interface TilesetInfo {
  path: string;
  name: string;
  tilecount: number;
  properties: Props;
  tiles: TileInfo[];
  vanilla: boolean;
}

export interface MaterialInfo {
  name: string;
  id: number;
  path: string;
  shortdescription: string;
  category: string;
  template: string | null;
  params: any;
  texture: string | null;
  collision: string;
}

export interface LiquidInfo {
  name: string;
  id: number;
  color: number[];
  texture: string | null;
}

export interface Catalog {
  install: string;
  tilesets: TilesetInfo[];
  materials: Record<string, MaterialInfo>;
  mods: Record<string, MaterialInfo>;
  liquids: Record<string, LiquidInfo>;
  templates: Record<string, any>;
  background_color: number[];
}

export interface ObjectInfo {
  name: string;
  path: string;
  shortdescription: string;
  category: string;
  race: string;
  object_type: string;
  colony_tags: string[];
  container: boolean;
  slot_count: number;
  input_nodes: [number, number][];
  output_nodes: [number, number][];
  icon: string | null;
}

export interface ItemInfo {
  name: string;
  shortdescription: string;
  category: string;
  rarity: string;
  kind: string;
  icon: string | null;
  /** built with random parts / stats when it drops (random-build weapons and shields) */
  random: boolean;
  /** armor and clothes: the folder of the set this piece belongs to */
  set: string | null;
}

export interface FileEntry {
  path: string;
  json: any;
}

export interface ProjectInfo {
  root: string;
  mode: "mod" | "loose";
  metadata: any;
  metadata_file: string;
  dungeons: FileEntry[];
  treasure: FileEntry[];
  patches: FileEntry[];
  maps: string[];
  tilesets: string[];
  sme: any;
  errors: string[];
}

export interface BiomeInfo {
  path: string;
  name: string;
  friendlyName?: string;
  surface: boolean;
  underground: boolean;
  surfaceMicro: any[];
  undergroundMicro: any[];
}

export interface SpawnSite {
  /** planet type, e.g. "tundra" */
  planet: string;
  layer: "surface" | "underground";
  /** the planet's main biome, a secondary biome, or a minibiome patch */
  via: "main" | "secondary" | "minibiome";
  biome: string;
  /** `/biomes/distributions.config:<name>` */
  distribution: string;
}

export interface GameIndex {
  /** `items`: names that show what the table gives, best first (use the first with an icon) */
  pools: Record<string, { file: string; vanilla: boolean; items: string[] }>;
  dungeons: Record<string, { file: string; vanilla: boolean; parts: number }>;
  biomes: BiomeInfo[];
  /** microdungeon name → where it can spawn (see `micro_spawns` in treasure.rs) */
  microSpawns: Record<string, SpawnSite[]>;
  distributions: Record<string, any>;
  npcTypes: string[];
  monsterTypes: string[];
  stagehands: string[];
  species: string[];
}

export interface Problem {
  severity: "error" | "warning" | "info";
  file: string;
  message: string;
  x: number | null;
  y: number | null;
}

export interface Settings {
  starbound_dir: string;
  author: string;
  recent: string[];
}

export interface Status {
  install: string | null;
  error: string | null;
  project: string | null;
}
