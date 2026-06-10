mod database;
mod scanner;
mod video_server;
mod watcher;

use database::{Database, FileInfo};
use parking_lot::Mutex;
use scanner::VideoFile;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

// ── State ─────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub video_port: u16,
    _watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

/// Holds the pending update between check and install
struct PendingUpdate(std::sync::Mutex<Option<tauri_plugin_updater::Update>>);

#[derive(Serialize, Clone)]
struct UpdateInfo {
    version: String,
    body: String,
}

// ── Update commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater_builder()
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        body: u.body.clone().unwrap_or_default(),
    });

    *state.0.lock().unwrap() = update;
    Ok(info)
}

#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<(), String> {
    let update = state.0.lock().unwrap().take();
    let Some(update) = update else {
        return Err("保留中のアップデートがありません".into());
    };

    let app_emit = app.clone();
    let downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let downloaded2 = downloaded.clone();

    update
        .download_and_install(
            move |chunk, content_len| {
                let current = downloaded2.fetch_add(chunk as u64, std::sync::atomic::Ordering::Relaxed)
                    + chunk as u64;
                let percent = content_len
                    .filter(|&t| t > 0)
                    .map(|t| (current * 100 / t) as u32)
                    .unwrap_or(0);
                let _ = app_emit.emit("update-progress", percent);
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}

// ── Background auto-check ─────────────────────────────────────────────────────

async fn auto_check_update(app: tauri::AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    let updater = match app.updater_builder().build() {
        Ok(u) => u,
        Err(_) => return,
    };

    let update = match updater.check().await {
        Ok(Some(u)) => u,
        _ => return,
    };

    let info = UpdateInfo {
        version: update.version.clone(),
        body: update.body.clone().unwrap_or_default(),
    };

    if let Some(state) = app.try_state::<PendingUpdate>() {
        *state.0.lock().unwrap() = Some(update);
    }

    let _ = app.emit("update-available", info);
}

// ── Other commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

#[tauri::command]
async fn scan_folder(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<VideoFile>, String> {
    let files = scanner::scan_directory(&path).map_err(|e| e.to_string())?;

    let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let enriched = {
        let db = state.db.lock();
        for file in &files {
            let _ = db.upsert_file(file);
        }
        db.get_ratings_map(&paths).unwrap_or_default()
    };

    let mut result = files;
    for f in &mut result {
        if let Some((rating, w, h)) = enriched.get(&f.path) {
            f.rating = *rating;
            if *w > 0 && *h > 0 {
                f.width = *w;
                f.height = *h;
            }
        }
    }

    Ok(result)
}

#[tauri::command]
fn get_video_port(state: tauri::State<'_, AppState>) -> u16 {
    state.video_port
}

#[tauri::command]
fn watch_folder(
    path: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let w = watcher::start_watching(&path, app).map_err(|e| e.to_string())?;
    *state._watcher.lock() = Some(w);
    Ok(())
}

#[tauri::command]
async fn update_dimensions(
    path: String,
    width: u32,
    height: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .lock()
        .update_dimensions(&path, width, height)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_rating(
    path: String,
    rating: u8,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .lock()
        .set_rating(&path, rating)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_tag(
    path: String,
    tag: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.db.lock().add_tag(&path, &tag).map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_tag(
    path: String,
    tag: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .lock()
        .remove_tag(&path, &tag)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_file_info(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<FileInfo>, String> {
    state
        .db
        .lock()
        .get_file_info(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_in_explorer(path: String, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;

    #[cfg(target_os = "windows")]
    {
        let path_win = path.replace("/", "\\");
        app.shell()
            .command("explorer")
            .args(["/select,", &path_win])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        app.shell()
            .command("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .unwrap_or(std::path::Path::new("/"))
            .to_str()
            .unwrap_or("/")
            .to_owned();
        app.shell()
            .command("xdg-open")
            .args([&parent])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("vviewer.db");

            let db = Database::new(&db_path)
                .map_err(|e| format!("DB init failed: {e}"))?;

            let video_port = video_server::start();

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
                video_port,
                _watcher: Mutex::new(None),
            });
            app.manage(PendingUpdate(std::sync::Mutex::new(None)));

            // Kick off background update check
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(auto_check_update(handle));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            scan_folder,
            get_video_port,
            watch_folder,
            update_dimensions,
            set_rating,
            add_tag,
            remove_tag,
            get_file_info,
            open_in_explorer,
            check_update,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vViewer");
}
