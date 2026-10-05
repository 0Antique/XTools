#[cfg(not(all(windows, target_arch = "x86_64")))]
compile_error!("XTools V1 supports Windows x64 only");

mod clipboard;
mod color_picker;
mod commands;
pub mod database;
pub mod launcher;
mod rename;
mod runtime;
pub mod settings;
mod tray;
pub mod windows;

use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex, RwLock},
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub settings: Mutex<settings::Settings>,
    pub apps: RwLock<Vec<launcher::Application>>,
    pub scanning: AtomicBool,
    pub rename_busy: Mutex<()>,
    pub exiting: AtomicBool,
    pub warnings: Mutex<Vec<String>>,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn log_error(_app: &tauri::AppHandle, message: &str) {
    log::error!("{message}");
}

pub fn run() {
    windows::dpi::initialize();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|arg| arg == "--quit") {
                runtime::request_exit(app);
            } else if let Err(e) = runtime::show_launcher(app) {
                log_error(app, &e);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Err(e) = runtime::toggle_launcher(app) {
                            log_error(app, &e);
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if std::env::args().any(|a| a == "--quit") {
                app.handle().exit(0);
                return Ok(());
            }
            let data_dir =
                PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA is unavailable")?)
                    .join("XTools");
            for sub in ["data", "clipboard/images", "cache/icons", "logs"] {
                std::fs::create_dir_all(data_dir.join(sub))?;
            }
            app.asset_protocol_scope()
                .allow_directory(data_dir.join("cache/icons"), true)?;
            app.asset_protocol_scope()
                .allow_directory(data_dir.join("clipboard/images"), true)?;
            runtime::initialize_log(&data_dir)?;
            let configuration = match settings::load(&data_dir) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("{e}; preserving invalid config as config.invalid.json");
                    let bad = data_dir.join("config.json");
                    if bad.exists() {
                        let _ = std::fs::copy(
                            &bad,
                            data_dir.join(format!("config.invalid.{}.json", now_ms())),
                        );
                    }
                    settings::Settings::default()
                }
            };
            let mut db = database::open(&data_dir.join("data/xtools.db"))?;
            clipboard::enforce_limit(
                &mut db,
                configuration.clipboard_limit,
                &data_dir.join("clipboard/images"),
            )?;
            let apps = database::load_applications(&db)?;
            app.manage(AppState {
                db: Mutex::new(db),
                data_dir: data_dir.clone(),
                settings: Mutex::new(configuration.clone()),
                apps: RwLock::new(apps),
                scanning: AtomicBool::new(false),
                rename_busy: Mutex::new(()),
                exiting: AtomicBool::new(false),
                warnings: Mutex::new(Vec::new()),
            });
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Err(e) = app
                .global_shortcut()
                .register(configuration.hotkey.as_str())
            {
                log::error!("快捷键 {} 注册失败：{e}", configuration.hotkey);
                // Keep the tray reachable when another application owns the configured hotkey.
                runtime::open_tool(app.handle(), "settings")?;
            }
            if !cfg!(debug_assertions) {
                if let Err(e) = settings::set_autostart(configuration.autostart) {
                    log::error!("开机启动初始化失败：{e}");
                }
            }
            settings::persist(&data_dir, &configuration)?;
            settings::acknowledge_installer_preference();
            tray::create(app.handle())?;
            if let Err(e) = clipboard::start_listener(app.handle().clone()) {
                log_error(app.handle(), &e);
                if let Ok(mut warnings) = app.state::<AppState>().warnings.lock() {
                    warnings.push(format!("剪贴板监听启动失败：{e}"));
                }
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(e) = runtime::scan_applications(&handle) {
                    log_error(&handle, &e);
                    let _ = handle.emit("scan-error", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::launch_application,
            commands::open_tool,
            commands::hide_launcher,
            commands::get_settings,
            commands::get_runtime_warnings,
            commands::save_settings,
            commands::rescan_applications,
            commands::list_clipboard,
            commands::favorite_clipboard,
            commands::delete_clipboard,
            commands::clear_clipboard,
            commands::copy_clipboard,
            commands::preview_rename,
            commands::execute_rename,
            commands::undo_rename,
            commands::rename_history_available
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("Unable to initialize XTools; see %APPDATA%\\XTools\\logs");
    app.run(|_app, event| {
        if let tauri::RunEvent::Exit = event {
            clipboard::stop_listener();
            color_picker::cancel();
        }
    });
}
