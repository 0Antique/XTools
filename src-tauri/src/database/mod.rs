mod schema;

use crate::launcher::Application;
use rusqlite::{params, Connection};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    conn.execute_batch(schema::SCHEMA)
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn load_applications(conn: &Connection) -> Result<Vec<Application>, String> {
    let mut statement = conn
        .prepare(
            "SELECT a.id,a.name,a.normalized_name,a.pinyin,a.pinyin_initials,a.source,
        a.launch_type,a.launch_target,a.arguments,a.working_directory,a.icon_path,a.unique_key,
        COALESCE(u.launch_count,0),u.last_launched_at
        FROM applications a LEFT JOIN app_usage u ON a.id=u.app_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Application {
                id: row.get(0)?,
                name: row.get(1)?,
                normalized_name: row.get(2)?,
                pinyin: row.get(3)?,
                pinyin_initials: row.get(4)?,
                source: row.get(5)?,
                launch_type: row.get(6)?,
                launch_target: row.get(7)?,
                arguments: row.get(8)?,
                working_directory: row.get(9)?,
                icon_path: row.get(10)?,
                unique_key: row.get(11)?,
                launch_count: row.get(12)?,
                last_launched_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn replace_applications(conn: &mut Connection, apps: &[Application]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM applications", [])
        .map_err(|e| e.to_string())?;
    for a in apps {
        tx.execute("INSERT INTO applications(id,name,normalized_name,pinyin,pinyin_initials,source,launch_type,launch_target,arguments,working_directory,icon_path,unique_key,last_seen_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![a.id,a.name,a.normalized_name,a.pinyin,a.pinyin_initials,a.source,a.launch_type,a.launch_target,a.arguments,a.working_directory,a.icon_path,a.unique_key,crate::now_ms()])
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn record_launch(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("INSERT INTO app_usage(app_id,launch_count,last_launched_at) VALUES(?1,1,?2)
        ON CONFLICT(app_id) DO UPDATE SET launch_count=launch_count+1,last_launched_at=excluded.last_launched_at",
        params![id,crate::now_ms()]).map_err(|e| e.to_string())?;
    Ok(())
}
