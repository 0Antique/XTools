use crate::{runtime, AppState};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub struct TrayState {
    pub autostart: CheckMenuItem<tauri::Wry>,
}

pub fn create(app: &AppHandle) -> Result<(), String> {
    let create_menu = || -> tauri::Result<_> {
        let open = MenuItem::with_id(app, "open", "打开 XTools", true, None::<&str>)?;
        let clipboard = MenuItem::with_id(app, "clipboard", "剪贴板", true, None::<&str>)?;
        let rename = MenuItem::with_id(app, "rename", "批量重命名", true, None::<&str>)?;
        let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
        let separator = PredefinedMenuItem::separator(app)?;
        let enabled = app
            .state::<AppState>()
            .settings
            .lock()
            .map(|s| s.autostart)
            .unwrap_or(false);
        let autostart =
            CheckMenuItem::with_id(app, "autostart", "开机启动", true, enabled, None::<&str>)?;
        let separator2 = PredefinedMenuItem::separator(app)?;
        let exit = MenuItem::with_id(app, "exit", "退出 XTools", true, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[
                &open,
                &clipboard,
                &rename,
                &settings,
                &separator,
                &autostart,
                &separator2,
                &exit,
            ],
        )?;
        Ok((menu, autostart))
    };
    let (menu, autostart) = create_menu().map_err(|e| e.to_string())?;
    app.manage(TrayState { autostart });
    TrayIconBuilder::with_id("xtools")
        .icon(app.default_window_icon().ok_or("缺少托盘图标")?.clone())
        .tooltip("XTools · Alt + Space")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let app = app.clone();
            let id = event.id.as_ref().to_owned();
            // WebView creation must not block a Windows menu/event callback.
            std::thread::spawn(move || {
                let app = &app;
                let result = match id.as_str() {
                    "open" => runtime::show_launcher(app),
                    "clipboard" | "rename" | "settings" => runtime::open_tool(app, id.as_str()),
                    "autostart" => {
                        let new = app
                            .state::<AppState>()
                            .settings
                            .lock()
                            .map(|s| {
                                let mut n = s.clone();
                                n.autostart = !n.autostart;
                                n
                            })
                            .map_err(|e| e.to_string());
                        match new {
                            Ok(s) => runtime::update_settings(app, s).map(|_| ()),
                            Err(e) => Err(e),
                        }
                    }
                    "exit" => {
                        runtime::request_exit(app);
                        Ok(())
                    }
                    _ => Ok(()),
                };
                if let Err(e) = result {
                    crate::log_error(app, &e);
                    let _ = runtime::open_tool(app, "settings");
                }
            });
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(e) = runtime::show_launcher(tray.app_handle()) {
                    crate::log_error(tray.app_handle(), &e);
                }
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;
    Ok(())
}
