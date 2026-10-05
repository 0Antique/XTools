use super::{rules::validate_name, RenamePreview};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub fn key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

pub fn validate_plan(rows: &mut [RenamePreview]) {
    let mut sources = HashMap::<String, usize>::new();
    let mut targets = HashMap::<String, usize>::new();
    for row in rows.iter() {
        *sources.entry(key(Path::new(&row.old_path))).or_default() += 1;
        *targets.entry(key(Path::new(&row.new_path))).or_default() += 1;
    }
    let mut nested = HashSet::new();
    for row in rows.iter() {
        let old = Path::new(&row.old_path);
        let old_key = key(old);
        let mut parent = old.parent();
        while let Some(path) = parent {
            let parent_key = key(path);
            if sources.contains_key(&parent_key) {
                nested.insert(old_key.clone());
                nested.insert(parent_key);
            }
            parent = path.parent();
        }
    }
    for row in rows.iter_mut() {
        if row.error.is_some() {
            continue;
        }
        if sources
            .get(&key(Path::new(&row.old_path)))
            .copied()
            .unwrap_or(0)
            > 1
        {
            row.error = Some("同一个对象重复加入".into());
            continue;
        }
        if targets
            .get(&key(Path::new(&row.new_path)))
            .copied()
            .unwrap_or(0)
            > 1
        {
            row.error = Some("多个对象将产生相同的新名称".into());
            continue;
        }
        if let Err(e) = validate_name(&row.new_name) {
            row.error = Some(e);
            continue;
        }
        let old = Path::new(&row.old_path);
        let new = Path::new(&row.new_path);
        if std::fs::symlink_metadata(old).is_err() {
            row.error = Some("原文件或文件夹已不存在 / 无法访问".into());
            continue;
        }
        if nested.contains(&key(old)) {
            row.error = Some("不能同时重命名文件夹及其内部对象；请分开操作".into());
            continue;
        }
        if std::fs::symlink_metadata(new).is_ok() && !sources.contains_key(&key(new)) {
            row.error = Some("目标路径已存在".into());
        }
    }
}

fn move_without_replace(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{MoveFileExW, MOVE_FILE_FLAGS},
        };
        let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe { MoveFileExW(PCWSTR(a.as_ptr()), PCWSTR(b.as_ptr()), MOVE_FILE_FLAGS(0)) }
            .map_err(|e| format!("{} → {}：{e}", from.display(), to.display()))
    }
    #[cfg(not(windows))]
    {
        if std::fs::symlink_metadata(to).is_ok() {
            return Err("目标路径已存在".into());
        }
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }
}

pub fn execute(rows: &[RenamePreview]) -> Result<usize, String> {
    let changed: Vec<&RenamePreview> = rows.iter().filter(|r| r.old_path != r.new_path).collect();
    let mut staged: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::new();
    for row in &changed {
        let old = PathBuf::from(&row.old_path);
        let parent = old.parent().ok_or("不能重命名磁盘根目录")?;
        let temp = parent.join(format!(".xtools_tmp_{}", uuid::Uuid::new_v4()));
        if let Err(e) = move_without_replace(&old, &temp) {
            let rollback = rollback(&staged, 0);
            return Err(format!("临时重命名失败：{e}{rollback}"));
        }
        staged.push((old, temp, PathBuf::from(&row.new_path)));
    }
    for (i, (_, temp, target)) in staged.iter().enumerate() {
        if let Err(e) = move_without_replace(temp, target) {
            let rollback = rollback(&staged, i);
            return Err(format!("重命名失败：{e}{rollback}"));
        }
    }
    Ok(changed.len())
}

fn rollback(staged: &[(PathBuf, PathBuf, PathBuf)], completed: usize) -> String {
    let mut errors = Vec::new();
    // Move completed final names back to their own unique temporary paths first,
    // so cyclic/swap plans can be restored without clobbering another source.
    for (_, temp, target) in staged.iter().take(completed).rev() {
        if let Err(e) = move_without_replace(target, temp) {
            errors.push(e);
        }
    }
    for (old, temp, _) in staged.iter().rev() {
        if std::fs::symlink_metadata(temp).is_ok() {
            if let Err(e) = move_without_replace(temp, old) {
                errors.push(e);
            }
        }
    }
    if errors.is_empty() {
        "；已回滚原名称".into()
    } else {
        format!(
            "；部分对象未能回滚，请检查 .xtools_tmp_ 文件：{}",
            errors.join("；")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(old: &Path, new: &Path) -> RenamePreview {
        RenamePreview {
            old_path: old.to_string_lossy().into(),
            new_path: new.to_string_lossy().into(),
            old_name: old.file_name().unwrap().to_string_lossy().into(),
            new_name: new.file_name().unwrap().to_string_lossy().into(),
            error: None,
        }
    }
    #[test]
    fn two_phase_swap_preserves_both_contents() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.txt");
        let b = d.path().join("b.txt");
        std::fs::write(&a, "A").unwrap();
        std::fs::write(&b, "B").unwrap();
        let mut rows = vec![row(&a, &b), row(&b, &a)];
        validate_plan(&mut rows);
        assert!(rows.iter().all(|r| r.error.is_none()));
        assert_eq!(execute(&rows).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(a).unwrap(), "B");
        assert_eq!(std::fs::read_to_string(b).unwrap(), "A");
    }
    #[test]
    fn late_target_conflict_rolls_back_without_overwrite() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        let b = d.path().join("b");
        let c = d.path().join("c");
        let x = d.path().join("occupied");
        std::fs::write(&a, "A").unwrap();
        std::fs::write(&b, "B").unwrap();
        std::fs::write(&x, "KEEP").unwrap();
        assert!(execute(&[row(&a, &c), row(&b, &x)]).is_err());
        assert_eq!(std::fs::read_to_string(a).unwrap(), "A");
        assert_eq!(std::fs::read_to_string(b).unwrap(), "B");
        assert_eq!(std::fs::read_to_string(x).unwrap(), "KEEP");
        assert!(!c.exists());
    }
}
