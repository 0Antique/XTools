pub const APP_ID: &str = "com.antique.xtools";

pub fn send_color(hex: &str) -> Result<(), String> {
    // Use the plugin's underlying Tauri WinRT backend directly: the plugin's
    // desktop wrapper discards asynchronous show errors. This preserves native
    // short-duration toasts and lets us retain a meaningful local diagnostic.
    tauri_winrt_notification::Toast::new(APP_ID)
        .title("颜色已复制")
        .text1(hex)
        .duration(tauri_winrt_notification::Duration::Short)
        .show()
        .map_err(|error| error.to_string())
}
pub fn color_copied(app: tauri::AppHandle, hex: String) {
    std::thread::spawn(move || {
        if let Err(error) = send_color(&hex) {
            crate::log_error(&app, &format!("通知发送失败：{error}"));
        }
    });
}
