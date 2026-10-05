use super::Application;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub fn normalized_executable(path: &str) -> String {
    let path = Path::new(path);
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.to_string_lossy();
    let path = if let Some(share) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{share}")
    } else {
        text.trim_start_matches("\\\\?\\").to_owned()
    };
    path.replace('/', "\\").to_lowercase()
}

fn source_priority(source: &str) -> u8 {
    match source {
        "startMenu" => 0,
        "desktop" => 1,
        "uwp" => 2,
        _ => 3,
    }
}

fn deduplicate(apps: Vec<Application>) -> Vec<Application> {
    let mut indexed = BTreeMap::<String, Application>::new();
    for app in apps {
        let replace = indexed.get(&app.unique_key).is_none_or(|existing| {
            source_priority(&app.source) < source_priority(&existing.source)
                || (source_priority(&app.source) == source_priority(&existing.source)
                    && (&app.normalized_name, &app.launch_target)
                        < (&existing.normalized_name, &existing.launch_target))
        });
        if replace {
            indexed.insert(app.unique_key.clone(), app);
        }
    }
    let mut results: Vec<_> = indexed.into_values().collect();
    results.sort_by(|a, b| {
        a.normalized_name
            .cmp(&b.normalized_name)
            .then_with(|| a.id.cmp(&b.id))
    });
    results
}

#[cfg(windows)]
pub fn scan(icon_dir: &Path) -> Result<Vec<Application>, String> {
    use windows::Win32::UI::Shell::{
        FOLDERID_CommonPrograms, FOLDERID_Desktop, FOLDERID_Programs, FOLDERID_PublicDesktop,
    };
    let _apartment = crate::windows::shell::ComApartment::initialize()?;
    let mut result = Vec::new();
    let roots = [
        (FOLDERID_Programs, "startMenu", true),
        (FOLDERID_CommonPrograms, "startMenu", true),
        (FOLDERID_Desktop, "desktop", false),
        (FOLDERID_PublicDesktop, "desktop", false),
    ];
    let mut readable_sources = 0;
    for (id, source, recursive) in roots {
        if let Some(root) = crate::windows::shell::known_folder(&id) {
            if !root.is_dir() {
                continue;
            }
            match collect_paths(&root, recursive) {
                Ok(paths) => {
                    readable_sources += 1;
                    for path in paths {
                        if let Some(app) = from_path(&path, source, icon_dir) {
                            result.push(app);
                        }
                    }
                }
                Err(error) => {
                    return Err(format!(
                        "读取应用来源 {} 失败，保留之前的索引：{}",
                        root.display(),
                        error
                    ))
                }
            }
        }
    }
    for app in crate::windows::registry::installed_applications() {
        let target = app.executable.to_string_lossy().into_owned();
        let key = format!("exe:{}", normalized_executable(&target));
        let mut application =
            Application::new(app.name, "registry", "executable", target.clone(), key);
        application.working_directory = app
            .executable
            .parent()
            .map(|path| path.to_string_lossy().into_owned());
        application.icon_path = super::icon::cache_icon(
            icon_dir,
            &application.unique_key,
            &target,
            app.icon_location.as_deref(),
            app.icon_index,
        );
        result.push(application);
    }
    match crate::windows::shell::store_applications() {
        Ok(apps) => {
            readable_sources += 1;
            for app in apps {
                let mut application = Application::new(
                    app.name,
                    "uwp",
                    "uwp",
                    app.aumid.clone(),
                    format!("aumid:{}", app.aumid.to_lowercase()),
                );
                // AppsFolder can return a bare AUMID as its absolute parsing
                // name. Resolve it within the folder for icon extraction.
                let parsing_name = format!("shell:AppsFolder\\{}", app.aumid);
                application.icon_path = super::icon::cache_icon(
                    icon_dir,
                    &application.unique_key,
                    &parsing_name,
                    None,
                    0,
                );
                result.push(application);
            }
        }
        Err(error) => return Err(format!("{error}，保留之前的索引")),
    }
    if readable_sources == 0 || result.is_empty() {
        return Err("应用扫描未能读取有效应用来源，保留之前的索引".into());
    }
    Ok(deduplicate(result))
}

#[cfg(windows)]
fn from_path(path: &Path, source: &str, icon_dir: &Path) -> Option<Application> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    if !crate::windows::registry::is_user_application(&name) {
        return None;
    }
    let path_string = path.to_string_lossy().into_owned();
    if path.extension()?.eq_ignore_ascii_case("exe") {
        let key = format!("exe:{}", normalized_executable(&path_string));
        let mut app = Application::new(name, source, "executable", path_string.clone(), key);
        app.working_directory = path
            .parent()
            .map(|path| path.to_string_lossy().into_owned());
        app.icon_path = super::icon::cache_icon(icon_dir, &app.unique_key, &path_string, None, 0);
        return Some(app);
    }
    let shortcut = crate::windows::shortcuts::resolve(path).ok()?;
    let aumid = shortcut.aumid.filter(|id| id.contains('!'));
    let mut app = if let Some(id) = aumid {
        Application::new(
            name,
            source,
            "uwp",
            id.clone(),
            format!("aumid:{}", id.to_lowercase()),
        )
    } else {
        let target = Path::new(&shortcut.target);
        if !target.is_file()
            || !target
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return None;
        }
        let key = format!("exe:{}", normalized_executable(&shortcut.target));
        Application::new(name, source, "shortcut", path_string.clone(), key)
    };
    // Store shortcuts can contain explorer.exe's AppsFolder arguments; these
    // should not be forwarded to the UWP application's activation manager.
    if app.launch_type != "uwp" {
        app.arguments = shortcut.arguments;
    }
    app.working_directory = shortcut.working_directory;
    app.icon_path = super::icon::cache_icon(
        icon_dir,
        &app.unique_key,
        &path_string,
        shortcut.icon_location.as_deref(),
        shortcut.icon_index,
    );
    Some(app)
}

/// Only Start Menu folders recurse. Reparse points are never followed, so
/// redirected junctions cannot accidentally turn this into a whole-disk scan.
fn collect_paths(root: &Path, recursive: bool) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![(root.to_path_buf(), 0u8)];
    let mut paths = Vec::new();
    let mut visited = 0;
    while let Some((directory, depth)) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if directory == root => return Err(error.to_string()),
            Err(_) => continue,
        };
        for entry in entries.filter_map(Result::ok) {
            visited += 1;
            if visited > 50_000 {
                return Err("开始菜单内容超过安全扫描上限".into());
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    continue;
                }
            }
            if metadata.is_symlink() {
                continue;
            }
            let path = entry.path();
            if metadata.is_dir() {
                if recursive && depth < 32 {
                    pending.push((path, depth + 1));
                }
            } else if metadata.is_file()
                && path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("exe")
                })
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(not(windows))]
pub fn scan(_: &Path) -> Result<Vec<Application>, String> {
    Err("应用扫描仅支持 Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_precedence_preserves_start_menu_and_stable_identity() {
        let start = Application::new(
            "微信".into(),
            "startMenu",
            "shortcut",
            "Start.lnk".into(),
            "exe:c:\\wechat.exe".into(),
        );
        let desktop = Application::new(
            "WeChat".into(),
            "desktop",
            "shortcut",
            "Desktop.lnk".into(),
            start.unique_key.clone(),
        );
        let registry = Application::new(
            "WeChat Windows".into(),
            "registry",
            "executable",
            "c:\\wechat.exe".into(),
            start.unique_key.clone(),
        );
        assert_eq!(start.id, desktop.id);
        let found = deduplicate(vec![registry, desktop, start]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, "startMenu");
        assert_eq!(found[0].name, "微信");
    }
    #[test]
    fn only_start_menu_traversal_is_recursive_and_never_indexes_documents() {
        let root = std::env::temp_dir().join(format!(
            "xtools-scanner-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("Child")).unwrap();
        std::fs::write(root.join("App.LNK"), []).unwrap();
        std::fs::write(root.join("Document.pdf"), []).unwrap();
        std::fs::write(root.join("Child").join("ChildApp.exe"), []).unwrap();
        assert_eq!(collect_paths(&root, false).unwrap().len(), 1);
        assert_eq!(collect_paths(&root, true).unwrap().len(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
