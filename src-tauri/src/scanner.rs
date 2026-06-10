use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "flv", "ts", "mts", "m2ts",
    "3gp", "ogv", "rm", "rmvb", "divx",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub modified: u64,
    pub rating: u8,
    pub tags: Vec<String>,
}

impl VideoFile {
    fn from_path(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }

        let ext = path.extension()?.to_str()?.to_lowercase();
        if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
            return None;
        }

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Some(VideoFile {
            path: path.to_string_lossy().into_owned(),
            filename: path.file_name()?.to_string_lossy().into_owned(),
            size: metadata.len(),
            // Dimensions default to 16:9 — the frontend updates them on first play
            width: 1920,
            height: 1080,
            duration: 0.0,
            modified,
            rating: 0,
            tags: vec![],
        })
    }
}

pub fn scan_directory(path: &str) -> anyhow::Result<Vec<VideoFile>> {
    let entries: Vec<_> = WalkDir::new(path)
        .max_depth(10)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let files: Vec<VideoFile> = entries
        .par_iter()
        .filter_map(|entry| VideoFile::from_path(entry.path()))
        .collect();

    Ok(files)
}
