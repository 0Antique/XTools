use crate::{launcher, settings::Settings, AppState};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command(async)]
pub fn search(
    state: State<AppState>,
    query: String,
) -> Result<Vec<launcher::SearchResult>, String> {
    let show_recent = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .show_recent;
    let apps = state.apps.read().map_err(|e| e.to_string())?;
    Ok(launcher::search::search(&apps, &query, show_recent))
}
#[tauri::command(async)]
pub async fn launch_application(app: AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let selected = state
            .apps
            .read()
            .map_err(|e| e.to_string())?
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or("应用索引已变化，请重新搜索")?;
        launcher::launch::launch(&selected)?;
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            crate::database::record_launch(&db, &id)?;
        }
        if let Some(a) = state
            .apps
            .write()
            .map_err(|e| e.to_string())?
            .iter_mut()
            .find(|a| a.id == id)
        {
            a.launch_count += 1;
            a.last_launched_at = Some(crate::now_ms());
        }
        if let Some(w) = app.get_webview_window("launcher") {
            w.hide().map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub async fn open_tool(app: AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::runtime::open_tool(&app, &id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub fn hide_launcher(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("launcher")
        .ok_or("Launcher 窗口不可用")?
        .hide()
        .map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command(async)]
pub fn get_runtime_warnings(app: AppHandle) -> Result<Vec<String>, String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let state = app.state::<AppState>();
    let mut warnings = state.warnings.lock().map_err(|e| e.to_string())?.clone();
    let hotkey = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .hotkey
        .clone();
    if !app.global_shortcut().is_registered(hotkey.as_str()) {
        warnings.insert(0,format!("无法注册快捷键 {hotkey}，可能已被其他程序占用。请录制其他快捷键；仍可通过托盘打开 XTools。"));
    }
    Ok(warnings)
}
#[tauri::command(async)]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    crate::runtime::update_settings(&app, settings)
}
#[tauri::command(async)]
pub async fn rescan_applications(app: AppHandle) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || crate::runtime::scan_applications(&app))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub fn list_clipboard(
    state: State<AppState>,
    query: String,
) -> Result<Vec<crate::clipboard::ClipboardItem>, String> {
    crate::clipboard::list(&*state.db.lock().map_err(|e| e.to_string())?, &query)
}
#[tauri::command(async)]
pub fn favorite_clipboard(app: AppHandle, id: i64, favorite: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let limit = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .clipboard_limit;
    crate::clipboard::favorite(
        &mut *state.db.lock().map_err(|e| e.to_string())?,
        id,
        favorite,
        limit,
        &state.data_dir.join("clipboard/images"),
    )?;
    app.emit("clipboard-changed", ()).map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub fn delete_clipboard(app: AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    crate::clipboard::delete(
        &mut *state.db.lock().map_err(|e| e.to_string())?,
        id,
        &state.data_dir.join("clipboard/images"),
    )?;
    app.emit("clipboard-changed", ()).map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub fn clear_clipboard(app: AppHandle, include_favorites: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    crate::clipboard::clear(
        &mut *state.db.lock().map_err(|e| e.to_string())?,
        include_favorites,
        &state.data_dir.join("clipboard/images"),
    )?;
    app.emit("clipboard-changed", ()).map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub async fn copy_clipboard(app: AppHandle, id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        {
            let state = app.state::<AppState>();
            crate::clipboard::copy(&*state.db.lock().map_err(|e| e.to_string())?, id)?;
        }
        if let Some(w) = app.get_webview_window("launcher") {
            w.hide().map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub async fn preview_rename(
    paths: Vec<String>,
    rules: crate::rename::RenameRules,
) -> Result<Vec<crate::rename::RenamePreview>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::rename::preview(&paths, &rules))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub async fn execute_rename(
    app: AppHandle,
    paths: Vec<String>,
    rules: crate::rename::RenameRules,
) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _guard = state.rename_busy.lock().map_err(|e| e.to_string())?;
        if state.exiting.load(std::sync::atomic::Ordering::Acquire) {
            return Err("XTools 正在退出，请重新启动后操作".into());
        }
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        crate::rename::execute(&mut db, &paths, &rules)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub async fn undo_rename(app: AppHandle) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _guard = state.rename_busy.lock().map_err(|e| e.to_string())?;
        if state.exiting.load(std::sync::atomic::Ordering::Acquire) {
            return Err("XTools 正在退出，请重新启动后操作".into());
        }
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        crate::rename::undo(&mut db)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command(async)]
pub fn rename_history_available(state: State<AppState>) -> Result<bool, String> {
    crate::rename::history_available(&*state.db.lock().map_err(|e| e.to_string())?)
}
