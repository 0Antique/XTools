use super::{executor, RenamePreview};
use rusqlite::{params, Connection};

pub fn save(conn: &Connection, rows: &[RenamePreview]) -> Result<(), String> {
    conn.execute(
        "INSERT INTO rename_batches(created_at) VALUES(?1)",
        params![crate::now_ms()],
    )
    .map_err(|e| e.to_string())?;
    let batch = conn.last_insert_rowid();
    for row in rows {
        conn.execute(
            "INSERT INTO rename_items(batch_id,old_path,new_path) VALUES(?1,?2,?3)",
            params![batch, row.old_path, row.new_path],
        )
        .map_err(|e| e.to_string())?;
    }
    // V1 offers exactly one undo operation, including across application restarts.
    conn.execute("DELETE FROM rename_batches WHERE id<>?1", params![batch])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn reverse(rows: &[RenamePreview]) -> Vec<RenamePreview> {
    rows.iter()
        .map(|r| RenamePreview {
            old_path: r.new_path.clone(),
            new_path: r.old_path.clone(),
            old_name: r.new_name.clone(),
            new_name: r.old_name.clone(),
            error: None,
        })
        .collect()
}

pub fn available(conn: &Connection) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM rename_batches WHERE undone=0)",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn undo(conn: &mut Connection) -> Result<usize, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let batch: i64 = tx
        .query_row(
            "SELECT id FROM rename_batches WHERE undone=0 ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "没有可撤销的重命名操作".to_string())?;
    let rows = {
        let mut stmt = tx
            .prepare("SELECT old_path,new_path FROM rename_items WHERE batch_id=?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let iter = stmt
            .query_map(params![batch], |r| {
                let old: String = r.get(0)?;
                let new: String = r.get(1)?;
                Ok(RenamePreview {
                    old_name: std::path::Path::new(&old)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    new_name: std::path::Path::new(&new)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    old_path: old,
                    new_path: new,
                    error: None,
                })
            })
            .map_err(|e| e.to_string())?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    let mut reverse = reverse(&rows);
    executor::validate_plan(&mut reverse);
    if let Some(r) = reverse.iter().find(|r| r.error.is_some()) {
        return Err(format!(
            "撤销冲突：{}，{}",
            r.new_name,
            r.error.as_deref().unwrap_or("对象不存在")
        ));
    }
    tx.execute(
        "UPDATE rename_batches SET undone=1 WHERE id=?1",
        params![batch],
    )
    .map_err(|e| e.to_string())?;
    let count = executor::execute(&reverse)?;
    if let Err(e) = tx.commit() {
        return Err(format!(
            "撤销状态保存失败：{e}；恢复结果：{:?}",
            executor::execute(&rows)
        ));
    }
    Ok(count)
}
