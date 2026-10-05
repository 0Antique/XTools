use crate::{database, launcher, settings, AppState};
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    sync::{atomic::Ordering, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

struct FileLogger(Mutex<std::fs::File>);
impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Warn
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            if let Ok(mut f) = self.0.lock() {
                let _ = writeln!(f, "{} {} {}", crate::now_ms(), r.level(), r.args());
            }
        }
    }
    fn flush(&self) {
        if let Ok(mut f) = self.0.lock() {
            let _ = f.flush();
        }
    }
}
pub fn initialize_log(dir: &Path) -> std::io::Result<()> {
    let path = dir.join("logs/xtools.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 2_000_000) {
        let _ = std::fs::rename(&path, dir.join("logs/xtools.previous.log"));
    }
    let logger = Box::leak(Box::new(FileLogger(Mutex::new(
        OpenOptions::new().create(true).append(true).open(path)?,
    ))));
    let _ = log::set_logger(logger).map(|()| log::set_max_level(log::LevelFilter::Warn));
    Ok(())
}

pub fn show_launcher(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("launcher")
        .ok_or("Launcher 窗口不可用")?;
    crate::windows::dpi::position_launcher(&window)?;
    window.show().map_err(|e| e.to_string())?;
    window
        .emit("launcher-shown", ())
        .map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
pub fn toggle_launcher(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("launcher")
        .ok_or("Launcher 窗口不可用")?;
    if window.is_visible().map_err(|e| e.to_string())? {
        window.hide().map_err(|e| e.to_string())
    } else {
        show_launcher(app)
    }
}
pub fn request_exit(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        app.exit(0);
        return;
    };
    if state.exiting.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        // Allow an active two-phase rename or rollback to finish before shutdown.
        let _guard = state.rename_busy.lock();
        app.exit(0);
    });
}
pub fn open_tool(app: &AppHandle, id: &str) -> Result<(), String> {
    match id {
        "clipboard" => {
            show_launcher(app)?;
            app.get_webview_window("launcher")
                .ok_or("Launcher 窗口不可用")?
                .emit("tool-opened", serde_json::json!({"id":id}))
                .map_err(|e| e.to_string())
        }
        "color" => {
            if let Some(w) = app.get_webview_window("launcher") {
                w.hide().map_err(|e| e.to_string())?;
            }
            match crate::color_picker::start(app.clone()) {
                Ok(()) => Ok(()),
                Err(e) => {
                    let _ = show_launcher(app);
                    Err(e)
                }
            }
        }
        "rename" | "settings" => {
            if let Some(w) = app.get_webview_window("launcher") {
                let _ = w.hide();
            }
            if let Some(w) = app.get_webview_window(id) {
                w.show().map_err(|e| e.to_string())?;
                w.unminimize().map_err(|e| e.to_string())?;
                return w.set_focus().map_err(|e| e.to_string());
            }
            let (width, height, title) = if id == "rename" {
                (1000., 720., "XTools · 批量重命名")
            } else {
                (720., 680., "XTools · 设置")
            };
            WebviewWindowBuilder::new(
                app,
                id,
                WebviewUrl::App(format!("index.html?view={id}").into()),
            )
            .title(title)
            .inner_size(width, height)
            .min_inner_size(
                if id == "rename" { 900. } else { 660. },
                if id == "rename" { 650. } else { 620. },
            )
            .center()
            .build()
            .map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => Err("未知的内置工具".into()),
    }
}

pub fn scan_applications(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<AppState>();
    if state.scanning.swap(true, Ordering::AcqRel) {
        return Err("应用扫描已在进行，请稍候".into());
    }
    struct Reset<'a>(&'a std::sync::atomic::AtomicBool);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _reset = Reset(&state.scanning);
    let apps = launcher::scanner::scan(&state.data_dir.join("cache/icons"))?;
    let count = apps.len();
    let loaded = {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        database::replace_applications(&mut db, &apps)?;
        database::load_applications(&db)?
    };
    *state.apps.write().map_err(|e| e.to_string())? = loaded;
    app.emit("applications-updated", count)
        .map_err(|e| e.to_string())?;
    Ok(count)
}

pub fn update_settings(
    app: &AppHandle,
    new: settings::Settings,
) -> Result<settings::Settings, String> {
    let state = app.state::<AppState>();
    let mut old = state.settings.lock().map_err(|e| e.to_string())?;
    settings::update(app, &state.data_dir, &old, &new)?;
    *old = new.clone();
    drop(old);
    if let Ok(mut db) = state.db.lock() {
        if let Err(e) = crate::clipboard::enforce_limit(
            &mut db,
            new.clipboard_limit,
            &state.data_dir.join("clipboard/images"),
        ) {
            crate::log_error(app, &e);
        }
    }
    if let Some(tray) = app.try_state::<crate::tray::TrayState>() {
        let _ = tray.autostart.set_checked(new.autostart);
    }
    let _ = app.emit("settings-changed", &new);
    let _ = app.emit("clipboard-changed", ());
    Ok(new)
}
