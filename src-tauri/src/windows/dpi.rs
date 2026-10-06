/// Must run before Tauri creates a window. Already configured DPI awareness
/// returns ACCESS_DENIED and is intentionally left intact.
pub fn initialize() {
    #[cfg(windows)]
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
}

pub fn position_launcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    position(window, 780., 520., true)
}
pub fn position_tool(window: &tauri::WebviewWindow, id: &str) -> Result<(), String> {
    let (width, height) = if id == "rename" {
        (1000., 720.)
    } else {
        (720., 680.)
    };
    position(window, width, height, false)
}
fn position(
    window: &tauri::WebviewWindow,
    logical_width: f64,
    logical_height: f64,
    launcher: bool,
) -> Result<(), String> {
    use windows::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY},
        UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    };
    // The Windows primary display always contains the desktop origin.
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        log::warn!("主显示器查询失败，使用 Tauri 屏幕兜底");
        return window.center().map_err(|e| e.to_string());
    }
    let (mut dx, mut dy) = (96, 96);
    unsafe {
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    }
    let scale = dx as f64 / 96.;
    let area = info.rcWork;
    let available_width = (area.right - area.left).max(1) as u32;
    let available_height = (area.bottom - area.top).max(1) as u32;
    let width = ((logical_width * scale).round() as u32).min(available_width);
    let height = ((logical_height * scale).round() as u32).min(available_height);
    let x = area.left + ((available_width - width) / 2) as i32;
    let y = if launcher {
        (info.rcMonitor.top + (130. * scale).round() as i32)
            .max(area.top)
            .min(area.bottom - height as i32)
    } else {
        area.top + ((available_height - height) / 2) as i32
    };
    window
        .set_min_size(Some(tauri::PhysicalSize::new(
            width.min((if launcher { 680. } else { 600. } * scale) as u32),
            height.min((440. * scale) as u32),
        )))
        .map_err(|e| e.to_string())?;
    // Move before resizing to let native DPI awareness adopt the destination.
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())
}
