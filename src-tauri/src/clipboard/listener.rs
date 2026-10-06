#[cfg(windows)]
mod implementation {
    use super::super::{native::*, parser, storage};
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::{Emitter, Manager};

    static RUNNING: AtomicBool = AtomicBool::new(false);
    static WINDOW: AtomicIsize = AtomicIsize::new(0);

    struct RunningGuard;
    impl Drop for RunningGuard {
        fn drop(&mut self) {
            WINDOW.store(0, Ordering::Release);
            RUNNING.store(false, Ordering::Release);
        }
    }

    fn changed(app: &tauri::AppHandle) {
        let result = (|| {
            let Some(content) = parser::capture()? else {
                return Ok::<bool, String>(false);
            };
            let state = app.state::<crate::AppState>();
            let limit = state
                .settings
                .lock()
                .map_err(|_| "设置锁已损坏")?
                .clipboard_limit;
            let image_dir = state.data_dir.join("clipboard").join("images");
            let prepared = storage::prepare(content, &image_dir)?;
            let mut connection = state.db.lock().map_err(|_| "数据库锁已损坏")?;
            storage::record_prepared(&mut connection, prepared, &image_dir, limit)?;
            Ok(true)
        })();
        match result {
            Ok(true) => {
                let _ = app.emit("clipboard-changed", ());
            }
            Ok(false) => {}
            Err(error) => crate::log_error(app, &format!("Clipboard: {error}")),
        }
    }

    unsafe extern "system" fn window_proc(
        window: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        match message {
            WM_NCCREATE => {
                // CREATESTRUCTW begins with lpCreateParams.
                let parameter = *(lparam as *const *const std::ffi::c_void);
                SetWindowLongPtrW(window, GWLP_USERDATA, parameter as isize);
                1
            }
            WM_CLIPBOARDUPDATE => {
                let app = GetWindowLongPtrW(window, GWLP_USERDATA) as *const tauri::AppHandle;
                if !app.is_null() {
                    changed(&*app);
                }
                0
            }
            WM_CLOSE => {
                RemoveClipboardFormatListener(window);
                DestroyWindow(window);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    pub fn start_listener(app: tauri::AppHandle) -> Result<(), String> {
        if RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }
        let (ready, response) = mpsc::sync_channel(1);
        if let Err(error) = std::thread::Builder::new()
            .name("xtools-clipboard".into())
            .spawn(move || {
                let _running = RunningGuard;
                let class_name = wide("XToolsClipboardListener");
                let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
                let class = WindowClass {
                    size: std::mem::size_of::<WindowClass>() as u32,
                    style: 0,
                    window_proc: Some(window_proc),
                    class_extra: 0,
                    window_extra: 0,
                    instance,
                    icon: 0,
                    cursor: 0,
                    background: 0,
                    menu_name: std::ptr::null(),
                    class_name: class_name.as_ptr(),
                    small_icon: 0,
                };
                unsafe {
                    RegisterClassExW(&class);
                }
                let app = Box::new(app);
                let window = unsafe {
                    CreateWindowExW(
                        0,
                        class_name.as_ptr(),
                        class_name.as_ptr(),
                        0,
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE,
                        0,
                        instance,
                        (&*app as *const tauri::AppHandle).cast(),
                    )
                };
                if window == 0 {
                    let _ = ready.send(Err("无法创建剪贴板监听窗口".into()));
                    return;
                }
                if unsafe { AddClipboardFormatListener(window) } == 0 {
                    unsafe {
                        DestroyWindow(window);
                    }
                    let _ = ready.send(Err("无法注册剪贴板更新监听".into()));
                    return;
                }
                WINDOW.store(window, Ordering::Release);
                let _ = ready.send(Ok(()));
                changed(&app);
                let mut message = Message::default();
                loop {
                    let status = unsafe { GetMessageW(&mut message, 0, 0, 0) };
                    if status <= 0 {
                        break;
                    }
                    unsafe {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                unsafe {
                    RemoveClipboardFormatListener(window);
                    DestroyWindow(window);
                }
            })
        {
            RUNNING.store(false, Ordering::Release);
            return Err(error.to_string());
        }
        response
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "剪贴板监听启动超时")?
    }

    pub fn stop_listener() {
        let window = WINDOW.load(Ordering::Acquire);
        if window != 0 {
            unsafe {
                PostMessageW(window, WM_CLOSE, 0, 0);
            }
        }
    }
}

#[cfg(windows)]
pub use implementation::{start_listener, stop_listener};

#[cfg(not(windows))]
pub fn start_listener(_: tauri::AppHandle) -> Result<(), String> {
    Err("剪贴板监听仅支持 Windows".into())
}
#[cfg(not(windows))]
pub fn stop_listener() {}
