import { invoke } from "@tauri-apps/api/core";
import type {
  Catalog,
  Doc,
  GameIndex,
  Imported,
  ItemInfo,
  ObjectInfo,
  Problem,
  ProjectInfo,
  Settings,
  Status,
} from "./types";

const dec = new TextDecoder();
async function bigJson<T>(cmd: string): Promise<T> {
  const buf = await invoke<ArrayBuffer>(cmd);
  return JSON.parse(dec.decode(buf)) as T;
}

export const api = {
  status: () => invoke<Status>("status"),
  startupView: () => invoke<string | null>("startup_view"),
  selftestMode: () => invoke<boolean>("selftest_mode"),
  startupPath: () => invoke<string | null>("startup_path"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<Status>("set_settings", { settings }),
  catalog: () => bigJson<Catalog>("catalog"),
  objectDb: () => bigJson<Record<string, ObjectInfo>>("object_db"),
  itemDb: () => bigJson<ItemInfo[]>("item_db"),
  gameIndex: () => invoke<GameIndex>("game_index"),
  assetJson: (path: string) => invoke<any>("asset_json", { path }),

  openProject: (dir: string) => invoke<ProjectInfo>("open_project", { dir }),
  rescanProject: () => invoke<ProjectInfo>("rescan_project"),
  closeProject: () => invoke<void>("close_project"),
  createProject: (spec: { dir: string; name: string; friendly_name: string; author: string; description: string }) =>
    invoke<ProjectInfo>("create_project", { spec }),
  importPak: (pak: string, parent: string) => invoke<ProjectInfo>("import_pak", { pak, parent }),
  projectPreview: (dir: string) =>
    invoke<{ root: string; exists: boolean; title: string; dungeons: number; maps: string[] }>("project_preview", { dir }),
  readMapIn: (root: string, path: string) => invoke<Imported>("read_map_in", { root, path }),
  forgetRecent: (dir: string) => invoke<void>("forget_recent", { dir }),

  readMap: (path: string) => invoke<Imported>("read_map", { path }),
  writeMap: (path: string, doc: Doc) => invoke<void>("write_map", { path, doc }),
  readJson: (path: string) => invoke<any>("read_json", { path }),
  writeJson: (path: string, value: any) => invoke<void>("write_json", { path, value }),
  deleteFile: (path: string) => invoke<void>("delete_file", { path }),
  renameFile: (from: string, to: string) => invoke<void>("rename_file", { from, to }),
  fileExists: (path: string) => invoke<boolean>("file_exists", { path }),

  validate: (input: any) => invoke<Problem[]>("validate", { input }),
  simulateLoot: (pools: any, pool: string, level: number, rolls: number) =>
    invoke<[string, number][][]>("simulate_loot", { pools, pool, level, rolls }),

  modsFolder: () => invoke<string>("mods_folder"),
  installMod: (method: "symlink" | "copy" | "pak") => invoke<string>("install_mod", { method }),
  uninstallMod: () => invoke<string>("uninstall_mod"),
  installState: () => invoke<string | null>("install_state"),
  exportPak: (out: string) => invoke<number>("export_pak", { out }),
  starboundLog: () => invoke<string>("starbound_log"),
};

/** URL for the `sme://` protocol. `ref` is `i:<install path>` or `a:<asset path>`. */
export function assetUrl(ref: string): string {
  const isWin = navigator.userAgent.includes("Windows");
  const base = isWin ? "http://sme.localhost/" : "sme://localhost/";
  if (ref.startsWith("i:")) return base + "install/" + encodeURI(ref.slice(2));
  if (ref.startsWith("a:")) return base + "asset/" + encodeURI(ref.slice(2).replace(/^\//, ""));
  return base + "asset/" + encodeURI(ref.replace(/^\//, ""));
}
