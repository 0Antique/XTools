use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct InstalledApplication {
    pub name: String,
    pub executable: PathBuf,
    pub icon_location: Option<String>,
    pub icon_index: i32,
}

/// Registry records are suggestions, not launch commands. Never run an
/// UninstallString or treat a DLL/installer's DisplayIcon as an application.
pub fn is_user_application(name: &str) -> bool {
    let lower = name.to_lowercase();
    let rejected = [
        "runtime",
        "redistributable",
        "visual c++",
        "driver",
        " sdk",
        "sdk ",
        "update",
        "hotfix",
        "security patch",
        "uninstall",
        "卸载",
        "驱动",
        "运行库",
        "更新",
        "补丁",
    ];
    !name.trim().is_empty() && lower != "sdk" && !rejected.iter().any(|word| lower.contains(word))
}

pub(crate) fn icon_spec(value: &str) -> (String, i32) {
    let value = value.trim();
    if let Some((path, index)) = value.rsplit_once(',') {
        if let Ok(index) = index.trim().parse() {
            return (path.trim().trim_matches('"').into(), index);
        }
    }
    (value.trim_matches('"').into(), 0)
}

fn usable_executable(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        && path.file_stem().is_some_and(|stem| {
            let stem = stem.to_string_lossy().to_lowercase();
            ![
                "unins", "uninst", "setup", "install", "update", "crash", "helper",
            ]
            .iter()
            .any(|s| stem.contains(s))
        })
}

#[cfg(windows)]
mod native {
    use super::*;
    use crate::windows::{from_wide, wide};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
            System::{
                Environment::ExpandEnvironmentStringsW,
                Registry::{
                    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY,
                    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_SZ,
                },
            },
        },
    };
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
    fn open(parent: HKEY, name: &str) -> Option<Key> {
        let name = wide(name);
        let mut key = HKEY::default();
        (unsafe { RegOpenKeyExW(parent, PCWSTR(name.as_ptr()), None, KEY_READ, &mut key) }
            == ERROR_SUCCESS)
            .then_some(Key(key))
    }
    fn text(key: HKEY, name: &str) -> Option<String> {
        let name = wide(name);
        let mut length = 0;
        let mut kind = REG_SZ;
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                None,
                Some(&mut length),
            )
        };
        if status != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) || length > 131_072
        {
            return None;
        }
        let mut buffer = vec![0u16; length as usize / 2 + 1];
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut length),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let raw = from_wide(&buffer);
        if kind == REG_EXPAND_SZ || raw.contains('%') {
            let raw_wide = wide(&raw);
            let length = unsafe { ExpandEnvironmentStringsW(PCWSTR(raw_wide.as_ptr()), None) };
            if length > 0 && length < 65_536 {
                let mut expanded = vec![0u16; length as usize];
                unsafe {
                    ExpandEnvironmentStringsW(PCWSTR(raw_wide.as_ptr()), Some(&mut expanded));
                }
                return Some(from_wide(&expanded));
            }
        }
        Some(raw)
    }
    fn flag(key: HKEY, name: &str) -> bool {
        let name = wide(name);
        let mut value = 0u32;
        let mut size = 4;
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            )
        };
        status == ERROR_SUCCESS && value != 0
    }
    fn candidate(key: HKEY) -> Option<InstalledApplication> {
        let name = text(key, "DisplayName")?;
        if !is_user_application(&name)
            || flag(key, "SystemComponent")
            || text(key, "ParentKeyName").is_some()
        {
            return None;
        }
        let display_icon = text(key, "DisplayIcon").map(|value| icon_spec(&value));
        let direct = display_icon
            .as_ref()
            .map(|(path, _)| PathBuf::from(path))
            .filter(|path| usable_executable(path));
        let executable = direct.or_else(|| {
            let directory = PathBuf::from(text(key, "InstallLocation")?.trim().trim_matches('"'));
            if !directory.is_dir() {
                return None;
            }
            let compact_name: String = name
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            let mut choices: Vec<_> = std::fs::read_dir(directory)
                .ok()?
                .take(500)
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| usable_executable(path))
                .collect();
            choices.sort();
            // A lone application executable is unambiguous. For directories
            // containing several executables, require a name resemblance.
            if choices.len() == 1 {
                return choices.pop();
            }
            choices.into_iter().find(|path| {
                let stem: String = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect();
                stem.chars().count() >= 3
                    && (compact_name.contains(&stem) || stem.contains(&compact_name))
            })
        })?;
        Some(InstalledApplication {
            name,
            executable,
            icon_location: display_icon.as_ref().map(|(path, _)| path.clone()),
            icon_index: display_icon.map(|(_, index)| index).unwrap_or(0),
        })
    }
    pub fn installed_applications() -> Vec<InstalledApplication> {
        let locations = [
            (
                HKEY_CURRENT_USER,
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
            (
                HKEY_LOCAL_MACHINE,
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
            (
                HKEY_LOCAL_MACHINE,
                "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
        ];
        let mut result = Vec::new();
        for (parent, location) in locations {
            let Some(root) = open(parent, location) else {
                continue;
            };
            for index in 0..20_000 {
                let mut name = vec![0u16; 512];
                let mut length = name.len() as u32;
                let status = unsafe {
                    RegEnumKeyExW(
                        root.0,
                        index,
                        Some(PWSTR(name.as_mut_ptr())),
                        &mut length,
                        None,
                        None,
                        None,
                        None,
                    )
                };
                if status == ERROR_NO_MORE_ITEMS {
                    break;
                }
                if status != ERROR_SUCCESS {
                    continue;
                }
                if let Some(key) = open(root.0, &from_wide(&name)) {
                    if let Some(app) = candidate(key.0) {
                        result.push(app);
                    }
                }
            }
        }
        result
    }
}

#[cfg(windows)]
pub use native::installed_applications;
#[cfg(not(windows))]
pub fn installed_applications() -> Vec<InstalledApplication> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_driver_and_maintenance_entries_are_filtered() {
        for name in [
            "Microsoft Visual C++ 2022 Redistributable",
            ".NET Runtime",
            "NVIDIA Driver",
            "Windows SDK",
            "Security Update",
            "卸载 微信",
        ] {
            assert!(!is_user_application(name), "{name}");
        }
        for name in [
            "微信",
            "Visual Studio Code",
            "Windows Terminal",
            "Calculator",
            "Microsoft Office",
        ] {
            assert!(is_user_application(name));
        }
    }
    #[test]
    fn display_icon_paths_can_contain_spaces_and_commas() {
        assert_eq!(
            icon_spec("\"C:\\Program Files\\App, Inc\\app.exe\",-2"),
            ("C:\\Program Files\\App, Inc\\app.exe".into(), -2)
        );
        assert_eq!(
            icon_spec("C:\\Apps\\app.exe"),
            ("C:\\Apps\\app.exe".into(), 0)
        );
    }
}
