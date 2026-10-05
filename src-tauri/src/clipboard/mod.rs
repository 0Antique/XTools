mod listener;
#[cfg(windows)]
mod native;
mod parser;
pub mod storage;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use listener::{start_listener, stop_listener};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: i64,
    pub item_type: String,
    pub text_content: Option<String>,
    pub data_path: Option<String>,
    pub preview: String,
    pub created_at: i64,
    pub is_favorite: bool,
}

pub fn list(conn: &Connection, query: &str) -> Result<Vec<ClipboardItem>, String> {
    storage::list(conn, query)
}

pub fn favorite(
    conn: &mut Connection,
    id: i64,
    favorite: bool,
    limit: usize,
    image_dir: &Path,
) -> Result<(), String> {
    storage::favorite(conn, id, favorite, limit, image_dir)
}

pub fn delete(conn: &mut Connection, id: i64, image_dir: &Path) -> Result<(), String> {
    storage::delete(conn, id, image_dir)
}

pub fn clear(
    conn: &mut Connection,
    include_favorites: bool,
    image_dir: &Path,
) -> Result<(), String> {
    storage::clear(conn, include_favorites, image_dir)
}

/// Only republishes the original content. The command caller hides the launcher;
/// no keystrokes are simulated and no automatic paste is performed.
pub fn copy(conn: &Connection, id: i64) -> Result<(), String> {
    let item = storage::get(conn, id)?;
    parser::write_item(&item)
}

pub(crate) fn write_text(text: &str) -> Result<(), String> {
    parser::set_text(text)
}

pub fn enforce_limit(conn: &mut Connection, limit: usize, image_dir: &Path) -> Result<(), String> {
    storage::enforce_limit(conn, limit, image_dir)
}
