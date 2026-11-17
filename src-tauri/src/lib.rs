//! SME: Starbound Microdungeon Editor, Tauri backend.

pub mod assets;
pub mod catalog;
pub mod frames;
pub mod install;
pub mod pak;
pub mod project;
pub mod sbjson;
pub mod tmx;
pub mod treasure;
pub mod validate;

use anyhow::anyhow;
use assets::{Assets, Source};
use catalog::Catalog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Settings {
    pub starbound_dir: String,
    pub author: String,
    pub recent: Vec<String>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("sme")
}
fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("sme")
}

fn load_settings() -> Settings {
    std::fs::read(config_dir().join("settings.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn store_settings(s: &Settings) -> anyhow::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(config_dir().join("settings.json"), serde_json::to_vec_pretty(s)?)?;
    Ok(())
}

#[derive(Default)]
pub struct AppState {
    settings: Mutex<Settings>,
    assets: RwLock<Option<Arc<Assets>>>,
    load_error: Mutex<Option<String>>,
    catalog: Mutex<Option<Arc<Catalog>>>,
    catalog_json: Mutex<Option<Arc<Vec<u8>>>>,
    objects: Mutex<Option<Arc<BTreeMap<String, catalog::ObjectInfo>>>>,
    items: Mutex<Option<Arc<Vec<treasure::ItemInfo>>>>,
    project: Mutex<Option<PathBuf>>,
}

impl AppState {
    /// (Re)opens the vanilla assets plus the project overlay.
    fn reload(&self) {
        let s = self.settings.lock().unwrap().clone();
        let project = self.project.lock().unwrap().clone();
        let res = assets::find_install(Some(&s.starbound_dir))
            .ok_or_else(|| anyhow!("Starbound install not found. Set its folder in Settings."))
            .and_then(|dir| Assets::open(&dir));
        match res {
            Ok(mut a) => {
                if let Some(p) = project {
                    a.sources.push(Source::Dir(p));
                }
                *self.assets.write().unwrap() = Some(Arc::new(a));
                *self.load_error.lock().unwrap() = None;
            }
            Err(e) => {
                *self.assets.write().unwrap() = None;
                *self.load_error.lock().unwrap() = Some(e.to_string());
            }
        }
        *self.catalog.lock().unwrap() = None;
        *self.catalog_json.lock().unwrap() = None;
        *self.objects.lock().unwrap() = None;
        *self.items.lock().unwrap() = None;
    }

    fn assets(&self) -> Res<Arc<Assets>> {
        self.assets.read().unwrap().clone().ok_or_else(|| {
            self.load_error.lock().unwrap().clone().unwrap_or_else(|| "Starbound assets not loaded".into())
        })
    }

    fn catalog(&self) -> Res<Arc<Catalog>> {
        if let Some(c) = self.catalog.lock().unwrap().clone() {
            return Ok(c);
        }
        let c = Arc::new(catalog::build(&*self.assets()?));
        *self.catalog.lock().unwrap() = Some(c.clone());
        Ok(c)
    }

    fn objects(&self) -> Res<Arc<BTreeMap<String, catalog::ObjectInfo>>> {
        if let Some(o) = self.objects.lock().unwrap().clone() {
            return Ok(o);
        }
        let o = Arc::new(catalog::build_objects(&*self.assets()?));
        *self.objects.lock().unwrap() = Some(o.clone());
        Ok(o)
    }

    fn items(&self) -> Res<Arc<Vec<treasure::ItemInfo>>> {
        if let Some(o) = self.items.lock().unwrap().clone() {
            return Ok(o);
        }
        let o = Arc::new(treasure::build_items(&*self.assets()?));
        *self.items.lock().unwrap() = Some(o.clone());
        Ok(o)
    }

    fn tilecounts(&self) -> Res<HashMap<String, u32>> {
        Ok(self.catalog()?.tilesets.iter().map(|t| (t.path.clone(), t.tilecount)).collect())
    }

    fn project(&self) -> Res<PathBuf> {
        self.project.lock().unwrap().clone().ok_or_else(|| "no project is open".to_string())
    }
}

type State<'a> = tauri::State<'a, Arc<AppState>>;

#[derive(Serialize)]
struct Status {
    install: Option<String>,
    error: Option<String>,
    project: Option<String>,
}

#[tauri::command]
fn status(st: State) -> Status {
    Status {
        install: st.assets.read().unwrap().as_ref().map(|a| a.install.to_string_lossy().into_owned()),
        error: st.load_error.lock().unwrap().clone(),
        project: st.project.lock().unwrap().as_ref().map(|p| p.to_string_lossy().into_owned()),
    }
}

/// A project/dungeon path passed on the command line (`sme path/to/mod`).
#[tauri::command]
fn startup_path() -> Option<String> {
    std::env::args().skip(1).find(|a| !a.starts_with('-') && Path::new(a).exists())
}

/// `--view=spawn|mod|loot|dungeon` to open a page directly (handy for screenshots).
#[tauri::command]
fn startup_view() -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix("--view=").map(String::from))
}

#[tauri::command]
fn selftest_mode() -> bool {
    std::env::args().any(|a| a == "--selftest")
}

#[tauri::command]
fn get_settings(st: State) -> Settings {
    st.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(st: State, settings: Settings) -> Res<Status> {
    let changed = st.settings.lock().unwrap().starbound_dir != settings.starbound_dir;
    store_settings(&settings).map_err(err)?;
    *st.settings.lock().unwrap() = settings;
    if changed {
        st.reload();
    }
    Ok(status(st))
}

#[tauri::command]
async fn catalog(st: State<'_>) -> Res<tauri::ipc::Response> {
    if let Some(j) = st.catalog_json.lock().unwrap().clone() {
        return Ok(tauri::ipc::Response::new((*j).clone()));
    }
    let c = st.catalog()?;
    let j = Arc::new(serde_json::to_vec(&*c).map_err(err)?);
    *st.catalog_json.lock().unwrap() = Some(j.clone());
    Ok(tauri::ipc::Response::new((*j).clone()))
}

#[tauri::command]
async fn object_db(st: State<'_>) -> Res<tauri::ipc::Response> {
    let o = st.objects()?;
    Ok(tauri::ipc::Response::new(serde_json::to_vec(&*o).map_err(err)?))
}

#[tauri::command]
async fn item_db(st: State<'_>) -> Res<tauri::ipc::Response> {
    let o = st.items()?;
    Ok(tauri::ipc::Response::new(serde_json::to_vec(&*o).map_err(err)?))
}

/// Names of every vanilla treasure pool, dungeon, biome, npc type, monster type and stagehand.
#[tauri::command]
async fn game_index(st: State<'_>) -> Res<Value> {
    let a = st.assets()?;
    Ok(treasure::game_index(&a))
}

#[tauri::command]
fn ui_font(st: State) -> Res<tauri::ipc::Response> {
    let cached = cache_dir().join("hobo.ttf");
    if let Ok(b) = std::fs::read(&cached) {
        return Ok(tauri::ipc::Response::new(b));
    }
    let bytes = st.assets()?.read("/hobo.ttf").map_err(err)?;
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(&cached, &bytes);
    Ok(tauri::ipc::Response::new(bytes))
}

/// Reads any game asset as JSON (vanilla or project overlay).
#[tauri::command]
async fn asset_json(st: State<'_>, path: String) -> Res<Value> {
    st.assets()?.json(&path).map_err(err)
}

// ───────────────────────── project ─────────────────────────

fn remember_recent(st: &AppState, p: &Path) {
    let mut s = st.settings.lock().unwrap();
    let ps = p.to_string_lossy().into_owned();
    s.recent.retain(|r| r != &ps);
    s.recent.insert(0, ps);
    s.recent.truncate(12);
    let _ = store_settings(&s);
}

#[tauri::command]
async fn open_project(st: State<'_>, dir: String) -> Res<project::ProjectInfo> {
    let mut root = PathBuf::from(&dir);
    if root.is_file() {
        // a .dungeon / map file: open its mod, or its folder as a loose project
        root = project::find_mod_root(&root).unwrap_or_else(|| root.parent().unwrap_or(Path::new("/")).to_path_buf());
    }
    let info = project::scan(&root).map_err(err)?;
    *st.project.lock().unwrap() = Some(root.clone());
    remember_recent(&st, &root);
    st.reload();
    Ok(info)
}

#[tauri::command]
async fn rescan_project(st: State<'_>) -> Res<project::ProjectInfo> {
    project::scan(&st.project()?).map_err(err)
}

#[tauri::command]
fn close_project(st: State) {
    *st.project.lock().unwrap() = None;
    st.reload();
}

#[tauri::command]
async fn create_project(st: State<'_>, spec: project::NewProject) -> Res<project::ProjectInfo> {
    let root = project::create(&spec).map_err(err)?;
    open_project(st, root.to_string_lossy().into_owned()).await
}

#[derive(Serialize)]
struct ProjectPreview {
    root: String,
    exists: bool,
    title: String,
    dungeons: usize,
    /// a few representative maps (project-relative), anchor parts first
    maps: Vec<String>,
}

/// Summary of a (not necessarily open) project folder for the welcome screen's cards.
#[tauri::command]
async fn project_preview(dir: String) -> Res<ProjectPreview> {
    let root = PathBuf::from(&dir);
    let folder = root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| dir.clone());
    let Ok(info) = project::scan(&root) else {
        return Ok(ProjectPreview { root: dir, exists: false, title: folder, dungeons: 0, maps: vec![] });
    };
    let title = ["friendlyName", "name"]
        .iter()
        .find_map(|k| info.metadata.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from))
        .unwrap_or(folder);
    let join = |base: &str, f: &str| {
        let mut parts: Vec<&str> = base.split('/').collect();
        parts.pop();
        for s in f.split('/') {
            match s {
                ".." => {
                    parts.pop();
                }
                "" | "." => {}
                s => parts.push(s),
            }
        }
        parts.join("/")
    };
    let part_maps = |base: &str, parts: &[Value]| -> Vec<String> {
        parts
            .iter()
            .filter_map(|p| p.get("def").and_then(|d| d.get(1)))
            .flat_map(|f| match f {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
                _ => vec![],
            })
            .map(|f| join(base, &f))
            .collect()
    };
    let mut maps = Vec::new();
    let mut dungeons = 0;
    for d in &info.dungeons {
        dungeons += 1;
        if let Some(parts) = d.json.get("parts").and_then(Value::as_array) {
            maps.extend(part_maps(&d.path, parts).into_iter().take(2));
        }
    }
    for p in info.patches.iter().filter(|p| p.path.ends_with(".dungeon.patch")) {
        let ops: Vec<&Value> = p.json.as_array().map(|a| a.iter().flat_map(|o| o.as_array().map(|x| x.iter().collect()).unwrap_or_else(|| vec![o])).collect()).unwrap_or_default();
        let parts: Vec<Value> = ops
            .iter()
            .filter(|o| o.get("op").and_then(Value::as_str) == Some("add") && o.get("path").and_then(Value::as_str).is_some_and(|s| s.starts_with("/parts/")))
            .filter_map(|o| o.get("value").cloned())
            .collect();
        dungeons += parts.len();
        maps.extend(part_maps(&p.path, &parts));
    }
    if maps.is_empty() {
        maps = info.maps.clone();
    }
    maps.retain(|m| root.join(m).is_file());
    maps.dedup();
    maps.truncate(6);
    Ok(ProjectPreview { root: dir, exists: true, title, dungeons, maps })
}

/// Reads a map from any project folder (used by the welcome screen previews).
#[tauri::command]
async fn read_map_in(st: State<'_>, root: String, path: String) -> Res<tmx::Imported> {
    let f = project::safe_join(Path::new(&root), &path).map_err(err)?;
    let v = sbjson::parse(&std::fs::read(&f).map_err(|e| format!("{path}: {e}"))?).map_err(err)?;
    let counts = st.tilecounts()?;
    tmx::import(&v, &|p| counts.get(p).copied()).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
fn forget_recent(st: State, dir: String) {
    let mut s = st.settings.lock().unwrap();
    s.recent.retain(|r| r != &dir);
    let _ = store_settings(&s);
}

/// Unpacks a `.pak` (e.g. a Steam workshop `contents.pak`) into a new folder inside
/// `parent` and opens it as a project. Returns the project info.
#[tauri::command]
async fn import_pak(st: State<'_>, pak: String, parent: String) -> Res<project::ProjectInfo> {
    let p = pak::Pak::open(Path::new(&pak)).map_err(err)?;
    let base = PathBuf::from(&parent);
    let name = pak::suggested_dir_name(&p);
    let mut dest = base.join(&name);
    let mut i = 2;
    while dest.exists() {
        dest = base.join(format!("{name}_{i}"));
        i += 1;
    }
    pak::unpack(&p, &dest).map_err(err)?;
    open_project(st, dest.to_string_lossy().into_owned()).await
}

#[tauri::command]
async fn read_map(st: State<'_>, path: String) -> Res<tmx::Imported> {
    let root = st.project()?;
    let f = project::safe_join(&root, &path).map_err(err)?;
    let v = sbjson::parse(&std::fs::read(&f).map_err(|e| format!("{path}: {e}"))?).map_err(err)?;
    let counts = st.tilecounts()?;
    tmx::import(&v, &|p| counts.get(p).copied()).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
async fn write_map(st: State<'_>, path: String, doc: tmx::Doc) -> Res<()> {
    let root = st.project()?;
    let f = project::safe_join(&root, &path).map_err(err)?;
    let depth = path.trim_start_matches('/').matches('/').count();
    let rel_root = "../".repeat(depth);
    let counts = st.tilecounts()?;
    let v = tmx::export(&doc, &|p| counts.get(p).copied(), &rel_root).map_err(err)?;
    project::write_atomic(&f, sbjson::pretty(&v).as_bytes()).map_err(err)
}

#[tauri::command]
async fn read_json(st: State<'_>, path: String) -> Res<Value> {
    let f = project::safe_join(&st.project()?, &path).map_err(err)?;
    sbjson::parse(&std::fs::read(&f).map_err(|e| format!("{path}: {e}"))?).map_err(err)
}

#[tauri::command]
async fn write_json(st: State<'_>, path: String, value: Value) -> Res<()> {
    let f = project::safe_join(&st.project()?, &path).map_err(err)?;
    project::write_atomic(&f, sbjson::pretty(&value).as_bytes()).map_err(err)
}

#[tauri::command]
async fn delete_file(st: State<'_>, path: String) -> Res<()> {
    let f = project::safe_join(&st.project()?, &path).map_err(err)?;
    if f.is_file() {
        std::fs::remove_file(&f).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
async fn rename_file(st: State<'_>, from: String, to: String) -> Res<()> {
    let root = st.project()?;
    let a = project::safe_join(&root, &from).map_err(err)?;
    let b = project::safe_join(&root, &to).map_err(err)?;
    if b.exists() {
        return Err(format!("{to} already exists"));
    }
    if let Some(d) = b.parent() {
        std::fs::create_dir_all(d).map_err(err)?;
    }
    std::fs::rename(a, b).map_err(err)
}

#[tauri::command]
async fn file_exists(st: State<'_>, path: String) -> Res<bool> {
    Ok(project::safe_join(&st.project()?, &path).map_err(err)?.exists())
}

#[tauri::command]
async fn validate(st: State<'_>, input: validate::Input) -> Res<Vec<validate::Problem>> {
    let a = st.assets()?;
    let cat = st.catalog()?;
    let objects = st.objects()?;
    Ok(validate::run(&a, &cat, &objects, &input))
}

#[tauri::command]
async fn simulate_loot(st: State<'_>, pools: Value, pool: String, level: f64, rolls: u32) -> Res<Vec<Vec<(String, u64)>>> {
    let a = st.assets()?;
    treasure::simulate(&a, &pools, &pool, level, rolls).map_err(err)
}

#[tauri::command]
async fn mods_folder(st: State<'_>) -> Res<String> {
    Ok(st.assets()?.install.join("mods").to_string_lossy().into_owned())
}

#[tauri::command]
async fn install_mod(st: State<'_>, method: String) -> Res<String> {
    let a = st.assets()?;
    let root = st.project()?;
    install::install(&root, &a.install.join("mods"), &method).map_err(err)
}

#[tauri::command]
async fn uninstall_mod(st: State<'_>) -> Res<String> {
    let a = st.assets()?;
    let root = st.project()?;
    install::uninstall(&root, &a.install.join("mods")).map_err(err)
}

#[tauri::command]
async fn install_state(st: State<'_>) -> Res<Option<String>> {
    let a = st.assets()?;
    let root = st.project()?;
    Ok(install::state(&root, &a.install.join("mods")))
}

#[tauri::command]
async fn export_pak(st: State<'_>, out: String) -> Res<usize> {
    let root = st.project()?;
    pak::pack_dir(&root, Path::new(&out), &["sme.json"]).map_err(err)
}

#[tauri::command]
async fn starbound_log(st: State<'_>) -> Res<String> {
    let a = st.assets()?;
    let p = a.install.join("storage/starbound.log");
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let lines: Vec<&str> = s.lines().collect();
    Ok(lines[lines.len().saturating_sub(400)..].join("\n"))
}

// ───────────────────────── asset protocol ─────────────────────────

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "json" | "config" => "application/json",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

/// `sme://localhost/install/<path under install>` and `sme://localhost/asset/<asset path>`.
fn serve(app: &tauri::AppHandle, uri: &str) -> tauri::http::Response<Vec<u8>> {
    let st = app.state::<Arc<AppState>>();
    let path = uri.split("://").nth(1).map(|r| r.split_once('/').map(|x| x.1).unwrap_or("")).unwrap_or("");
    let path = pct_decode(path.split('?').next().unwrap_or(""));
    let result: Result<Vec<u8>, String> = (|| {
        let a = st.assets()?;
        if let Some(r) = path.strip_prefix("install/") {
            if r.split('/').any(|s| s == "..") {
                return Err("bad path".into());
            }
            std::fs::read(a.install.join(r)).map_err(err)
        } else if let Some(r) = path.strip_prefix("asset/") {
            a.read(&format!("/{r}")).map_err(err)
        } else {
            Err("unknown route".into())
        }
    })();
    let builder = tauri::http::Response::builder().header("Access-Control-Allow-Origin", "*");
    match result {
        Ok(b) => builder.header("Content-Type", mime(&path)).header("Cache-Control", "max-age=3600").body(b).unwrap(),
        Err(e) => builder.status(404).body(e.into_bytes()).unwrap(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::default());
    *state.settings.lock().unwrap() = load_settings();
    state.reload();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .register_uri_scheme_protocol("sme", |ctx, req| serve(ctx.app_handle(), &req.uri().to_string()))
        .invoke_handler(tauri::generate_handler![
            status,
            startup_path,
            selftest_mode,
            startup_view,
            get_settings,
            set_settings,
            catalog,
            object_db,
            item_db,
            game_index,
            ui_font,
            asset_json,
            open_project,
            rescan_project,
            close_project,
            create_project,
            import_pak,
            project_preview,
            read_map_in,
            forget_recent,
            read_map,
            write_map,
            read_json,
            write_json,
            delete_file,
            rename_file,
            file_exists,
            validate,
            simulate_loot,
            mods_folder,
            install_mod,
            uninstall_mod,
            install_state,
            export_pak,
            starbound_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running SME");
}
