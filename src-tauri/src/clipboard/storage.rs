use super::parser::CapturedContent;
use super::ClipboardItem;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const COLUMNS: &str = "id, item_type, text_content, data_path, preview, created_at, is_favorite";

fn item_from_row(row: &Row<'_>) -> rusqlite::Result<ClipboardItem> {
    Ok(ClipboardItem {
        id: row.get(0)?,
        item_type: row.get(1)?,
        text_content: row.get(2)?,
        data_path: row.get(3)?,
        preview: row.get(4)?,
        created_at: row.get(5)?,
        is_favorite: row.get::<_, i64>(6)? != 0,
    })
}

pub fn list(conn: &Connection, query: &str) -> Result<Vec<ClipboardItem>, String> {
    // User input is a literal substring, not a SQL LIKE expression.
    let escaped = query
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let sql = format!(
        "SELECT {COLUMNS} FROM clipboard_items \
         WHERE preview LIKE ?1 ESCAPE '\\' OR text_content LIKE ?1 ESCAPE '\\' \
         ORDER BY created_at DESC, id DESC"
    );
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([pattern], item_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub(crate) fn get(conn: &Connection, id: i64) -> Result<ClipboardItem, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM clipboard_items WHERE id = ?1"),
        [id],
        item_from_row,
    )
    .optional()
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "剪贴板记录已不存在".into())
}

pub(crate) fn record(
    conn: &mut Connection,
    content: CapturedContent,
    image_dir: &Path,
    limit: usize,
) -> Result<i64, String> {
    let (item_type, text_content, data_path, preview, hash) = prepare(content, image_dir)?;
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let latest: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(created_at), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    // Distinct updates in one millisecond must still move an older duplicate to
    // the front. The timestamp remains milliseconds since the Unix epoch.
    let created_at = now.max(latest.saturating_add(1));
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT id FROM clipboard_items WHERE content_hash = ?1",
            [&hash],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let id = if let Some(id) = existing {
        transaction
            .execute(
                "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
                params![created_at, id],
            )
            .map_err(|error| error.to_string())?;
        id
    } else {
        transaction
            .execute(
                "INSERT INTO clipboard_items \
                 (item_type, text_content, data_path, preview, content_hash, created_at, is_favorite) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                params![item_type, text_content, data_path, preview, hash, created_at],
            )
            .map_err(|error| error.to_string())?;
        transaction.last_insert_rowid()
    };
    let removed = trim(&transaction, limit)?;
    transaction.commit().map_err(|error| error.to_string())?;
    clean_images(removed, image_dir)?;
    Ok(id)
}

fn preview(text: &str) -> String {
    let normalized = text.replace('\r', "");
    let mut chars = normalized.chars();
    let mut result: String = chars.by_ref().take(240).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

type PreparedContent = (String, Option<String>, Option<String>, String, String);

fn prepare(content: CapturedContent, image_dir: &Path) -> Result<PreparedContent, String> {
    let mut digest = Sha256::new();
    match content {
        CapturedContent::Text(text) => {
            digest.update(b"text\0");
            digest.update(text.as_bytes());
            let hash = format!("{:x}", digest.finalize());
            let summary = preview(&text);
            Ok(("text".into(), Some(text), None, summary, hash))
        }
        CapturedContent::Files(paths) => {
            if paths.is_empty() || paths.iter().any(|path| path.contains('\0')) {
                return Err("剪贴板文件路径无效".into());
            }
            let json = serde_json::to_string(&paths).map_err(|error| error.to_string())?;
            digest.update(b"files\0");
            digest.update(json.as_bytes());
            let hash = format!("{:x}", digest.finalize());
            let summary = preview(&paths.join("\n"));
            Ok(("files".into(), Some(json), None, summary, hash))
        }
        CapturedContent::Image {
            width,
            height,
            rgba,
        } => {
            let expected = (width as usize)
                .checked_mul(height as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or("剪贴板图片尺寸无效")?;
            if width == 0 || height == 0 || rgba.len() != expected {
                return Err("剪贴板图片数据不完整".into());
            }
            digest.update(b"image\0");
            digest.update(width.to_le_bytes());
            digest.update(height.to_le_bytes());
            digest.update(&rgba);
            let hash = format!("{:x}", digest.finalize());
            fs::create_dir_all(image_dir).map_err(|error| error.to_string())?;
            let path = image_dir.join(format!("{hash}.png"));
            if !path.exists() {
                let image =
                    image::RgbaImage::from_raw(width, height, rgba).ok_or("剪贴板图片数据无效")?;
                // Save a complete file before publishing its path in SQLite.
                let temporary = image_dir.join(format!("{hash}.tmp"));
                if let Err(error) = image.save_with_format(&temporary, image::ImageFormat::Png) {
                    let _ = fs::remove_file(&temporary);
                    return Err(format!("保存剪贴板图片失败：{error}"));
                }
                if let Err(error) = fs::rename(&temporary, &path) {
                    let _ = fs::remove_file(&temporary);
                    return Err(format!("保存剪贴板图片失败：{error}"));
                }
            }
            Ok((
                "image".into(),
                None,
                Some(path.to_string_lossy().into_owned()),
                format!("图片 {width} × {height}"),
                hash,
            ))
        }
    }
}

fn trim(transaction: &Transaction<'_>, limit: usize) -> Result<Vec<String>, String> {
    let limit = limit.clamp(1, 10_000) as i64;
    let mut statement = transaction
        .prepare("SELECT id, data_path FROM clipboard_items WHERE is_favorite = 0 ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?1")
        .map_err(|error| error.to_string())?;
    let removed = statement
        .query_map([limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    let mut images = Vec::new();
    for (id, path) in removed {
        transaction
            .execute("DELETE FROM clipboard_items WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        if let Some(path) = path {
            images.push(path);
        }
    }
    Ok(images)
}

fn clean_images(paths: Vec<String>, image_dir: &Path) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let canonical_dir = image_dir
        .canonicalize()
        .map_err(|error| error.to_string())?;
    for path in paths {
        let path = PathBuf::from(path);
        if !path.exists() {
            continue;
        }
        let canonical = path.canonicalize().map_err(|error| error.to_string())?;
        // Only XTools-managed PNGs are eligible. File clipboard paths never
        // become data_path, and a modified database cannot delete user files.
        let managed_name = canonical
            .file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
            });
        if canonical.parent() != Some(canonical_dir.as_path())
            || canonical
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("png")
            || !managed_name
        {
            continue;
        }
        fs::remove_file(canonical).map_err(|error| format!("清理剪贴板图片失败：{error}"))?;
    }
    Ok(())
}

pub fn enforce_limit(conn: &mut Connection, limit: usize, image_dir: &Path) -> Result<(), String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let removed = trim(&transaction, limit)?;
    transaction.commit().map_err(|error| error.to_string())?;
    clean_images(removed, image_dir)
}

pub fn favorite(
    conn: &mut Connection,
    id: i64,
    favorite: bool,
    limit: usize,
    image_dir: &Path,
) -> Result<(), String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    if transaction
        .execute(
            "UPDATE clipboard_items SET is_favorite = ?1 WHERE id = ?2",
            params![favorite, id],
        )
        .map_err(|error| error.to_string())?
        == 0
    {
        return Err("剪贴板记录已不存在".into());
    }
    let removed = trim(&transaction, limit)?;
    transaction.commit().map_err(|error| error.to_string())?;
    clean_images(removed, image_dir)
}

pub fn delete(conn: &mut Connection, id: i64, image_dir: &Path) -> Result<(), String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let path: Option<String> = transaction
        .query_row(
            "SELECT data_path FROM clipboard_items WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    transaction
        .execute("DELETE FROM clipboard_items WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    clean_images(path.into_iter().collect(), image_dir)
}

pub fn clear(
    conn: &mut Connection,
    include_favorites: bool,
    image_dir: &Path,
) -> Result<(), String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let condition = if include_favorites {
        "1 = 1"
    } else {
        "is_favorite = 0"
    };
    let mut statement = transaction
        .prepare(&format!(
            "SELECT data_path FROM clipboard_items WHERE {condition} AND data_path IS NOT NULL"
        ))
        .map_err(|error| error.to_string())?;
    let paths = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    transaction
        .execute(
            &format!("DELETE FROM clipboard_items WHERE {condition}"),
            [],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    clean_images(paths, image_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE clipboard_items (id INTEGER PRIMARY KEY AUTOINCREMENT, item_type TEXT NOT NULL, text_content TEXT, data_path TEXT, preview TEXT, content_hash TEXT UNIQUE, created_at INTEGER NOT NULL, is_favorite INTEGER NOT NULL DEFAULT 0);").unwrap();
        connection
    }

    #[test]
    fn retention_protects_favorites_and_dedup_moves_existing_item_to_front() {
        let mut conn = database();
        let dir = std::env::temp_dir();
        let favorite_id = record(&mut conn, CapturedContent::Text("收藏".into()), &dir, 2).unwrap();
        favorite(&mut conn, favorite_id, true, 2, &dir).unwrap();
        let first = record(&mut conn, CapturedContent::Text("one".into()), &dir, 2).unwrap();
        record(&mut conn, CapturedContent::Text("two".into()), &dir, 2).unwrap();
        assert_eq!(
            record(&mut conn, CapturedContent::Text("one".into()), &dir, 2).unwrap(),
            first
        );
        assert_eq!(list(&conn, "").unwrap()[0].id, first);
        record(&mut conn, CapturedContent::Text("three".into()), &dir, 2).unwrap();
        let rows = list(&conn, "").unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .any(|item| item.id == favorite_id && item.is_favorite));
        assert!(rows.iter().any(|item| item.id == first));
        assert!(!rows
            .iter()
            .any(|item| item.text_content.as_deref() == Some("two")));
        clear(&mut conn, false, &dir).unwrap();
        assert_eq!(list(&conn, "").unwrap().len(), 1);
        clear(&mut conn, true, &dir).unwrap();
        assert!(list(&conn, "").unwrap().is_empty());
    }

    #[test]
    fn search_treats_percent_underscore_and_unicode_as_literal_content() {
        let mut conn = database();
        let dir = std::env::temp_dir();
        record(
            &mut conn,
            CapturedContent::Text("项目_100%".into()),
            &dir,
            100,
        )
        .unwrap();
        record(
            &mut conn,
            CapturedContent::Text("another item".into()),
            &dir,
            100,
        )
        .unwrap();
        assert_eq!(list(&conn, "_").unwrap().len(), 1);
        assert_eq!(list(&conn, "%").unwrap().len(), 1);
        assert_eq!(list(&conn, "项目").unwrap().len(), 1);
    }

    #[test]
    fn file_lists_preserve_paths_without_copying_files_and_unfavorite_enforces_limit() {
        let mut conn = database();
        let dir = std::env::temp_dir();
        let paths = vec![r"D:\论文\FedAvg.pdf".into(), r"D:\项目".into()];
        let id = record(&mut conn, CapturedContent::Files(paths.clone()), &dir, 1).unwrap();
        favorite(&mut conn, id, true, 1, &dir).unwrap();
        let item = get(&conn, id).unwrap();
        assert_eq!(item.item_type, "files");
        assert_eq!(
            serde_json::from_str::<Vec<String>>(item.text_content.as_deref().unwrap()).unwrap(),
            paths
        );
        assert!(item.data_path.is_none());
        let same = record(&mut conn, CapturedContent::Files(paths), &dir, 1).unwrap();
        assert_eq!(same, id);
        assert!(get(&conn, id).unwrap().is_favorite);
        record(
            &mut conn,
            CapturedContent::Text("newer ordinary entry".into()),
            &dir,
            1,
        )
        .unwrap();
        favorite(&mut conn, id, false, 1, &dir).unwrap();
        assert!(get(&conn, id).is_err());
        assert_eq!(list(&conn, "").unwrap().len(), 1);
    }

    #[test]
    fn image_dedup_reuses_png_and_deleting_item_removes_only_managed_image() {
        let mut conn = database();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "xtools-clipboard-test-{}-{unique}",
            std::process::id()
        ));
        let content = || CapturedContent::Image {
            width: 1,
            height: 1,
            rgba: vec![255, 128, 0, 255],
        };
        let id = record(&mut conn, content(), &dir, 100).unwrap();
        assert_eq!(record(&mut conn, content(), &dir, 100).unwrap(), id);
        let item = get(&conn, id).unwrap();
        let png = item.data_path.unwrap();
        assert_eq!(
            image::open(&png).unwrap().into_rgba8().as_raw(),
            &[255, 128, 0, 255]
        );
        let keep = dir.join("keep.txt");
        fs::write(&keep, "not managed").unwrap();
        delete(&mut conn, id, &dir).unwrap();
        assert!(!Path::new(&png).exists());
        assert!(keep.exists());
        fs::remove_file(keep).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn retention_and_clear_remove_ordinary_pngs_but_keep_favorite_pngs() {
        let mut conn = database();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "xtools-clipboard-retention-{}-{unique}",
            std::process::id()
        ));
        let image = |red| CapturedContent::Image {
            width: 1,
            height: 1,
            rgba: vec![red, 0, 0, 255],
        };
        let favorite_id = record(&mut conn, image(128), &dir, 1).unwrap();
        favorite(&mut conn, favorite_id, true, 1, &dir).unwrap();
        let favorite_png = get(&conn, favorite_id).unwrap().data_path.unwrap();
        let ordinary_id = record(&mut conn, image(255), &dir, 1).unwrap();
        let ordinary_png = get(&conn, ordinary_id).unwrap().data_path.unwrap();
        record(
            &mut conn,
            CapturedContent::Text("newest ordinary item".into()),
            &dir,
            1,
        )
        .unwrap();
        assert!(!Path::new(&ordinary_png).exists());
        assert!(Path::new(&favorite_png).exists());
        clear(&mut conn, false, &dir).unwrap();
        assert!(Path::new(&favorite_png).exists());
        clear(&mut conn, true, &dir).unwrap();
        assert!(!Path::new(&favorite_png).exists());
        fs::remove_dir(dir).unwrap();
    }
}
