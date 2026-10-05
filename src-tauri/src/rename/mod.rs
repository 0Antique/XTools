pub mod executor;
mod history;
pub mod rules;

pub use rules::RenameRules;
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePreview {
    pub old_path: String,
    pub new_path: String,
    pub old_name: String,
    pub new_name: String,
    pub error: Option<String>,
}

pub fn preview(paths: &[String], rules: &RenameRules) -> Result<Vec<RenamePreview>, String> {
    rules.validate()?;
    if paths.len() > 10_000 {
        return Err("每批最多支持 10000 个对象".into());
    }
    let mut rows = Vec::with_capacity(paths.len());
    for (index, path) in paths.iter().enumerate() {
        let input = Path::new(path);
        let name = input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut row = RenamePreview {
            old_path: path.clone(),
            new_path: path.clone(),
            old_name: name.clone(),
            new_name: name.clone(),
            error: None,
        };
        let normalize = || -> Result<(PathBuf, bool), String> {
            if !input.is_absolute() {
                return Err("请提供绝对路径".into());
            }
            let filename = input.file_name().ok_or("不能重命名根目录")?;
            let parent = input
                .parent()
                .ok_or("路径缺少父目录")?
                .canonicalize()
                .map_err(|e| format!("父目录无法访问：{e}"))?;
            let full = parent.join(filename);
            let metadata =
                std::fs::symlink_metadata(&full).map_err(|e| format!("原对象无法访问：{e}"))?;
            Ok((full, metadata.is_dir()))
        };
        match normalize() {
            Ok((old, is_dir)) => match rules.apply(&name, is_dir, index) {
                Ok(new_name) => {
                    row.old_path = old.to_string_lossy().into();
                    row.new_path = old.with_file_name(&new_name).to_string_lossy().into();
                    row.new_name = new_name;
                }
                Err(e) => row.error = Some(e),
            },
            Err(e) => row.error = Some(e),
        }
        rows.push(row);
    }
    executor::validate_plan(&mut rows);
    Ok(rows)
}

pub fn execute(
    conn: &mut Connection,
    paths: &[String],
    rules: &RenameRules,
) -> Result<usize, String> {
    let rows = preview(paths, rules)?;
    if rows.is_empty() {
        return Err("请先选择文件或文件夹".into());
    }
    if let Some(row) = rows.iter().find(|r| r.error.is_some()) {
        return Err(format!(
            "{}：{}",
            row.old_name,
            row.error.as_deref().unwrap_or("无效名称")
        ));
    }
    let changed: Vec<_> = rows
        .into_iter()
        .filter(|r| r.old_path != r.new_path)
        .collect();
    if changed.is_empty() {
        return Ok(0);
    }
    // Persist the undo record in the same open transaction as execution. If
    // persistence fails, reverse the filesystem operation before returning.
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    history::save(&tx, &changed)?;
    let count = executor::execute(&changed)?;
    if let Err(e) = tx.commit() {
        let reverse = history::reverse(&changed);
        let result = executor::execute(&reverse);
        return Err(format!("撤销记录保存失败：{e}；回滚结果：{result:?}"));
    }
    Ok(count)
}

pub fn undo(conn: &mut Connection) -> Result<usize, String> {
    history::undo(conn)
}
pub fn history_available(conn: &Connection) -> Result<bool, String> {
    history::available(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_operation_changes_only_the_folder() {
        let d = tempfile::tempdir().unwrap();
        let folder = d.path().join("papers");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("inside.txt"), "data").unwrap();
        let mut conn = crate::database::open(&d.path().join("test.db")).unwrap();
        let rules = RenameRules {
            prefix: "new_".into(),
            ..Default::default()
        };
        execute(&mut conn, &[folder.to_string_lossy().into()], &rules).unwrap();
        assert!(d.path().join("new_papers/inside.txt").exists());
        assert_eq!(undo(&mut conn).unwrap(), 1);
        assert!(folder.join("inside.txt").exists());
    }
    #[test]
    fn occupied_original_name_stops_undo() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.txt");
        std::fs::write(&a, "original").unwrap();
        let mut conn = crate::database::open(&d.path().join("test.db")).unwrap();
        execute(
            &mut conn,
            &[a.to_string_lossy().into()],
            &RenameRules {
                prefix: "new_".into(),
                ..Default::default()
            },
        )
        .unwrap();
        std::fs::write(&a, "foreign").unwrap();
        assert!(undo(&mut conn).is_err());
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "foreign");
        assert!(history_available(&conn).unwrap());
    }
}
