use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Default)]
struct Behavior {
    pinned: bool,
    protected: usize,
    transition: Option<Instant>,
}
#[derive(Default)]
pub struct WindowBehaviors(Mutex<HashMap<String, Behavior>>);

pub fn pinned(window: &WebviewWindow) -> Result<bool, String> {
    Ok(window
        .state::<WindowBehaviors>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .entry(window.label().to_owned())
        .or_default()
        .pinned)
}
pub fn set_pinned(window: &WebviewWindow, pinned: bool) -> Result<bool, String> {
    let state = window.state::<WindowBehaviors>();
    let mut states = state.0.lock().map_err(|e| e.to_string())?;
    window
        .set_always_on_top(pinned)
        .map_err(|e| e.to_string())?;
    states.entry(window.label().to_owned()).or_default().pinned = pinned;
    Ok(pinned)
}
pub fn transition(window: &WebviewWindow) {
    if let Ok(mut states) = window.state::<WindowBehaviors>().0.lock() {
        states
            .entry(window.label().to_owned())
            .or_default()
            .transition = Some(Instant::now());
    }
}
pub struct DialogGuard {
    app: AppHandle,
    label: String,
}
pub fn protect_dialog(window: &WebviewWindow) -> DialogGuard {
    if let Ok(mut states) = window.state::<WindowBehaviors>().0.lock() {
        states
            .entry(window.label().to_owned())
            .or_default()
            .protected += 1;
    }
    DialogGuard {
        app: window.app_handle().clone(),
        label: window.label().to_owned(),
    }
}
impl Drop for DialogGuard {
    fn drop(&mut self) {
        if let Ok(mut states) = self.app.state::<WindowBehaviors>().0.lock() {
            let state = states.entry(self.label.clone()).or_default();
            state.protected = state.protected.saturating_sub(1);
            state.transition = Some(Instant::now());
        }
    }
}
fn can_hide(window: &WebviewWindow) -> bool {
    let state = window.state::<WindowBehaviors>();
    let Ok(states) = state.0.lock() else {
        return false;
    };
    let allowed = states.get(window.label()).is_none_or(|b| {
        !b.pinned
            && b.protected == 0
            && b.transition
                .is_none_or(|time| time.elapsed() > Duration::from_millis(150))
    });
    allowed && window.is_visible().unwrap_or(false)
}
pub fn focus_lost(window: &tauri::Window) {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        if let Some(window) = app.get_webview_window(&label) {
            // Owned file dialogs and other XTools windows remain one workflow.
            if !foreground_is_ours() && can_hide(&window) {
                if window.label() == "launcher" {
                    crate::runtime::clear_invocation(&app);
                }
                let _ = window.hide();
            }
        }
    });
}
fn foreground_is_ours() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
    }
    pid == std::process::id()
}

// A mouse hook covers desktop/taskbar clicks that do not change foreground
// focus. It forwards every click untouched and does no IO in the hook.
static APP: OnceLock<AppHandle> = OnceLock::new();
pub fn start_external_clicks(app: AppHandle) {
    let _ = APP.set(app);
    std::thread::spawn(|| unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), None, 0) else {
            return;
        };
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = UnhookWindowsHookEx(hook);
    });
}
unsafe extern "system" fn mouse_hook(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    if code >= 0 && [WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN].contains(&(wparam.0 as u32)) {
        let point = (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt;
        let target = GetAncestor(WindowFromPoint(point), GA_ROOT);
        let owner = GetWindow(target, GW_OWNER).unwrap_or_default();
        let target = target.0 as usize;
        let owner = owner.0 as usize;
        if let Some(app) = APP.get() {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                for window in handle.webview_windows().values() {
                    if window
                        .hwnd()
                        .is_ok_and(|hwnd| hwnd.0 as usize == target || hwnd.0 as usize == owner)
                    {
                        continue;
                    }
                    if can_hide(window) {
                        if window.label() == "launcher" {
                            crate::runtime::clear_invocation(&handle);
                        }
                        let _ = window.hide();
                    }
                }
            });
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
