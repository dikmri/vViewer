use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tauri::{AppHandle, Emitter};

pub fn start_watching(path: &str, app: AppHandle) -> anyhow::Result<RecommendedWatcher> {
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                        let paths: Vec<String> = event
                            .paths
                            .iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect();
                        let _ = app.emit("files-changed", paths);
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )?;

    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
    Ok(watcher)
}
