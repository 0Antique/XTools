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

#[cfg(windows)]
pub fn position_launcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO,
            MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL,
        },
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::{GetCursorPos, GetForegroundWindow},
        },
    };
    let foreground = unsafe { GetForegroundWindow() };
    let mut monitor = unsafe { MonitorFromWindow(foreground, MONITOR_DEFAULTTONULL) };
    if monitor.0.is_null() {
        let mut mouse = POINT::default();
        unsafe { GetCursorPos(&mut mouse) }.map_err(|e| e.to_string())?;
        monitor = unsafe { MonitorFromPoint(mouse, MONITOR_DEFAULTTONEAREST) };
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return Err("无法确定当前显示器位置".into());
    }
    let (mut dpi_x, mut dpi_y) = (96, 96);
    unsafe {
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    }
    let scale = dpi_x as f64 / 96.0;
    let area = info.rcWork;
    let available_width = (area.right - area.left).max(1) as u32;
    let available_height = (area.bottom - area.top).max(1) as u32;
    let width = ((780.0 * scale).round() as u32).min(available_width);
    let height = ((520.0 * scale).round() as u32).min(available_height);
    let x = area.left + ((available_width - width) / 2) as i32;
    let y = (info.rcMonitor.top + (130.0 * dpi_y as f64 / 96.0).round() as i32)
        .max(area.top)
        .min(area.bottom - height as i32);
    // Using physical dimensions avoids the previous monitor's scale factor
    // during a cross-monitor move, including negative desktop coordinates.
    window
        .set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub fn position_launcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.center().map_err(|e| e.to_string())
}
