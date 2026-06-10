use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::scanner::VideoFile;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub rating: u8,
    pub tags: Vec<String>,
    pub width: u32,
    pub height: u32,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=10000;
             PRAGMA temp_store=memory;

             CREATE TABLE IF NOT EXISTS files (
                 id       INTEGER PRIMARY KEY AUTOINCREMENT,
                 path     TEXT UNIQUE NOT NULL,
                 width    INTEGER NOT NULL DEFAULT 1920,
                 height   INTEGER NOT NULL DEFAULT 1080,
                 duration REAL    NOT NULL DEFAULT 0.0,
                 size     INTEGER NOT NULL DEFAULT 0,
                 modified INTEGER NOT NULL DEFAULT 0,
                 rating   INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
             );

             CREATE INDEX IF NOT EXISTS idx_files_path   ON files(path);
             CREATE INDEX IF NOT EXISTS idx_files_rating ON files(rating);

             CREATE TABLE IF NOT EXISTS tags (
                 id   INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT UNIQUE NOT NULL
             );

             CREATE TABLE IF NOT EXISTS file_tags (
                 file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                 tag_id  INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
                 PRIMARY KEY (file_id, tag_id)
             );

             CREATE INDEX IF NOT EXISTS idx_file_tags_file ON file_tags(file_id);",
        )?;

        Ok(Self { conn })
    }

    pub fn upsert_file(&self, file: &VideoFile) -> Result<()> {
        self.conn.execute(
            "INSERT INTO files (path, width, height, duration, size, modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET
               size     = excluded.size,
               modified = excluded.modified",
            params![
                file.path,
                file.width,
                file.height,
                file.duration,
                file.size as i64,
                file.modified as i64
            ],
        )?;
        Ok(())
    }

    pub fn update_dimensions(&self, path: &str, width: u32, height: u32) -> Result<()> {
        self.conn.execute(
            "UPDATE files SET width=?1, height=?2 WHERE path=?3",
            params![width, height, path],
        )?;
        Ok(())
    }

    pub fn set_rating(&self, path: &str, rating: u8) -> Result<()> {
        self.conn.execute(
            "UPDATE files SET rating=?1 WHERE path=?2",
            params![rating, path],
        )?;
        Ok(())
    }

    pub fn add_tag(&self, path: &str, tag: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![tag],
        )?;

        let tag_id: i64 = self.conn.query_row(
            "SELECT id FROM tags WHERE name=?1",
            params![tag],
            |r| r.get(0),
        )?;

        if let Ok(file_id) = self.conn.query_row(
            "SELECT id FROM files WHERE path=?1",
            params![path],
            |r| r.get::<_, i64>(0),
        ) {
            self.conn.execute(
                "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
                params![file_id, tag_id],
            )?;
        }

        Ok(())
    }

    pub fn remove_tag(&self, path: &str, tag: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM file_tags WHERE
             file_id = (SELECT id FROM files WHERE path=?1) AND
             tag_id  = (SELECT id FROM tags  WHERE name=?2)",
            params![path, tag],
        )?;
        Ok(())
    }

    pub fn get_file_info(&self, path: &str) -> Result<Option<FileInfo>> {
        let result = self.conn.query_row(
            "SELECT rating, width, height FROM files WHERE path=?1",
            params![path],
            |r| Ok((r.get::<_, u8>(0)?, r.get::<_, u32>(1)?, r.get::<_, u32>(2)?)),
        );

        match result {
            Ok((rating, width, height)) => {
                let mut stmt = self.conn.prepare(
                    "SELECT t.name FROM tags t
                     JOIN file_tags ft ON ft.tag_id = t.id
                     JOIN files f ON f.id = ft.file_id
                     WHERE f.path = ?1",
                )?;

                let tags: Vec<String> = stmt
                    .query_map(params![path], |r| r.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();

                Ok(Some(FileInfo {
                    path: path.to_string(),
                    rating,
                    tags,
                    width,
                    height,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_ratings_map(&self, paths: &[String]) -> Result<std::collections::HashMap<String, (u8, u32, u32)>> {
        if paths.is_empty() {
            return Ok(Default::default());
        }
        let mut map = std::collections::HashMap::with_capacity(paths.len());
        let mut stmt = self
            .conn
            .prepare("SELECT path, rating, width, height FROM files WHERE path=?1")?;
        for path in paths {
            if let Ok((r, w, h)) = stmt.query_row(params![path], |row| {
                Ok((row.get::<_, u8>(0)?, row.get::<_, u32>(1)?, row.get::<_, u32>(2)?))
            }) {
                map.insert(path.clone(), (r, w, h));
            }
        }
        Ok(map)
    }
}
