use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    pub autostart: bool,
    pub clipboard_limit: usize,
    pub show_recent: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            autostart: true,
            clipboard_limit: 100,
            show_recent: true,
        }
    }
}

pub fn load(data_dir: &Path) -> Result<Settings, String> {
    let path = data_dir.join("config.json");
    if path.exists() {
        let s: Settings = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("配置文件无法读取：{e}"))?;
        validate(&s)?;
        Ok(apply_installer_preference(s))
    } else {
        let mut s = Settings::default();
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\XTools") {
            if let Ok(pref) = key.get_value::<u32, _>("AutostartPreference") {
                s.autostart = pref != 0;
            }
        }
        Ok(apply_installer_preference(s))
    }
}

fn apply_installer_preference(mut s: Settings) -> Settings {
    use winreg::enums::KEY_READ;
    if let Ok(key) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\XTools", KEY_READ)
    {
        if key
            .get_value::<u32, _>("ApplyInstallerPreference")
            .unwrap_or(0)
            != 0
        {
            if let Ok(pref) = key.get_value::<u32, _>("AutostartPreference") {
                s.autostart = pref != 0;
            }
        }
    }
    s
}

pub fn acknowledge_installer_preference() {
    use winreg::enums::KEY_WRITE;
    if let Ok(key) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\XTools", KEY_WRITE)
    {
        let _ = key.delete_value("ApplyInstallerPreference");
    }
}

pub fn validate(s: &Settings) -> Result<(), String> {
    if !(1..=10_000).contains(&s.clipboard_limit) {
        return Err("剪贴板普通历史数量需介于 1～10000".into());
    }
    let shortcut = s
        .hotkey
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map_err(|e| format!("无效的快捷键：{e}"))?;
    if shortcut.mods.is_empty() {
        return Err("快捷键至少需要 Alt、Ctrl、Shift 或 Win 修饰键".into());
    }
    Ok(())
}

pub fn persist(data_dir: &Path, s: &Settings) -> Result<(), String> {
    let path = data_dir.join("config.json");
    let tmp = data_dir.join(format!("config.{}.tmp", uuid::Uuid::new_v4()));
    let contents = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, contents).map_err(|e| e.to_string())?;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
    };
    let a: Vec<_> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
    let b: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        MoveFileExW(
            PCWSTR(a.as_ptr()),
            PCWSTR(b.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| e.to_string());
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    // Only this application's own per-user Run value is modified.
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = root
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        run.set_value("XTools", &format!("\"{}\" --autostart", exe.display()))
            .map_err(|e| e.to_string())?;
    } else {
        match run.delete_value("XTools") {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub fn update(app: &AppHandle, dir: &Path, old: &Settings, new: &Settings) -> Result<(), String> {
    validate(new)?;
    let shortcut_changed = old.hotkey != new.hotkey;
    let old_registered = app.global_shortcut().is_registered(old.hotkey.as_str());
    let register_new = shortcut_changed || !old_registered;
    // Reserve the new shortcut before releasing the old one. Failure leaves the
    // old registration and persisted settings intact.
    if register_new {
        app.global_shortcut()
            .register(new.hotkey.as_str())
            .map_err(|e| format!("无法注册该快捷键，原快捷键保持可用：{e}"))?;
    }
    if let Err(e) = set_autostart(new.autostart) {
        if register_new {
            let _ = app.global_shortcut().unregister(new.hotkey.as_str());
        }
        return Err(format!("开机启动设置失败：{e}"));
    }
    if let Err(e) = persist(dir, new) {
        let rollback = set_autostart(old.autostart);
        if register_new {
            let _ = app.global_shortcut().unregister(new.hotkey.as_str());
        }
        return Err(format!("设置保存失败：{e}；开机启动恢复结果：{rollback:?}"));
    }
    if shortcut_changed && old_registered {
        if let Err(e) = app.global_shortcut().unregister(old.hotkey.as_str()) {
            let _ = persist(dir, old);
            let _ = set_autostart(old.autostart);
            let _ = app.global_shortcut().unregister(new.hotkey.as_str());
            return Err(format!("无法释放原快捷键：{e}，已恢复设置"));
        }
    }
    Ok(())
}
