use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Mutex, OnceLock,
};
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionContext {
    pub request_id: u64,
    pub paths: Vec<String>,
    pub status: String,
    pub message: Option<String>,
}
#[derive(Default)]
pub struct InvocationState {
    sequence: AtomicU64,
    pub current: Mutex<Option<SelectionContext>>,
    pub rename: Mutex<Option<SelectionContext>>,
}
type Job = (isize, mpsc::SyncSender<Result<Vec<String>, String>>);
impl InvocationState {
    pub fn invalidate(&self) {
        self.sequence.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut current) = self.current.lock() {
            *current = None;
        }
    }

    fn publish(&self, context: &SelectionContext) -> bool {
        let Ok(mut current) = self.current.lock() else {
            return false;
        };
        if self.sequence.load(Ordering::Acquire) != context.request_id {
            return false;
        }
        *current = Some(context.clone());
        true
    }
}
static WORKER: OnceLock<mpsc::SyncSender<Job>> = OnceLock::new();

fn worker() -> &'static mpsc::SyncSender<Job> {
    WORKER.get_or_init(|| {
        let (send, receive) = mpsc::sync_channel::<Job>(1);
        std::thread::Builder::new()
            .name("xtools-explorer-sta".into())
            .spawn(move || {
                use windows::Win32::System::Com::*;
                if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
                    return;
                }
                while let Ok((hwnd, reply)) = receive.recv() {
                    let result = unsafe { selection(hwnd) }
                        .map_err(|e| format!("无法读取资源管理器选择：{e}"));
                    let _ = reply.send(result);
                    // Pump queued STA messages between requests; COM marshalled calls
                    // also dispatch their own modal RPC message loop.
                    unsafe {
                        use windows::Win32::UI::WindowsAndMessaging::*;
                        let mut msg = MSG::default();
                        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                }
                unsafe {
                    CoUninitialize();
                }
            })
            .expect("Explorer selection thread");
        send
    })
}
pub fn foreground() -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize }
}
pub fn capture(state: &InvocationState, hwnd: isize) -> SelectionContext {
    let id = state.sequence.fetch_add(1, Ordering::AcqRel) + 1;
    let mut context = SelectionContext {
        request_id: id,
        paths: vec![],
        status: "unavailable".into(),
        message: None,
    };
    if is_explorer(hwnd) {
        let (send, receive) = mpsc::sync_channel(1);
        let result = worker()
            .try_send((hwnd, send))
            .ok()
            .and_then(|_| receive.recv_timeout(Duration::from_millis(300)).ok());
        match result {
            Some(Ok(paths)) => {
                context.status = if paths.is_empty() { "empty" } else { "ready" }.into();
                context.paths = paths;
            }
            Some(Err(error)) => context.message = Some(error),
            None => {
                context.status = "timeout".into();
                context.message = Some("读取资源管理器选中项超时，请手动添加文件".into());
            }
        }
    }
    // A timed-out worker has no reference to this state, so its late result
    // cannot overwrite this or a later invocation.
    state.publish(&context);
    context
}
fn is_explorer(hwnd: isize) -> bool {
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GetClassNameW};
    let mut name = [0u16; 64];
    let length = unsafe { GetClassNameW(HWND(hwnd as *mut _), &mut name) };
    matches!(
        String::from_utf16_lossy(&name[..length.max(0) as usize]).as_str(),
        "CabinetWClass" | "ExploreWClass"
    )
}
unsafe fn selection(hwnd: isize) -> windows::core::Result<Vec<String>> {
    use windows::{
        core::Interface,
        Win32::{
            Foundation::HWND,
            System::{Com::*, Variant::VARIANT},
            UI::{Shell::*, WindowsAndMessaging::*},
        },
    };
    let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
    let mut matches = Vec::new();
    for index in 0..shell.Count()? {
        let dispatch = match shell.Item(&VARIANT::from(index)) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Ok(browser) = dispatch.cast::<IWebBrowserApp>() else {
            continue;
        };
        if browser.HWND()?.0 != hwnd {
            continue;
        }
        let provider: IServiceProvider = dispatch.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;
        let view_hwnd = view.GetWindow()?;
        // In tabbed Explorer reject hidden views; do not pick the first tab.
        if !IsWindowVisible(view_hwnd).as_bool()
            || GetAncestor(view_hwnd, GA_ROOT) != HWND(hwnd as *mut _)
        {
            continue;
        }
        let folder: IFolderView2 = view.cast()?;
        let items = match folder.GetSelection(false) {
            Ok(items) => items,
            Err(e) if e.code().is_ok() || e.code().0 == 0x80070490u32 as i32 => {
                matches.push(vec![]);
                continue;
            }
            Err(e) => return Err(e),
        };
        let mut paths = Vec::new();
        for index in 0..items.GetCount()? {
            let item = items.GetItemAt(index)?;
            if let Ok(value) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                let path = value.to_string();
                CoTaskMemFree(Some(value.0.cast()));
                if let Ok(path) = path {
                    if std::path::Path::new(&path).is_absolute() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
        }
        paths.sort_by(|a, b| {
            natural_cmp(
                a.rsplit('\\').next().unwrap_or(a),
                b.rsplit('\\').next().unwrap_or(b),
            )
        });
        let mut seen = std::collections::HashSet::new();
        paths.retain(|p| seen.insert(p.to_lowercase()));
        matches.push(paths);
    }
    // Ambiguous active views must never import a background tab's selection.
    if matches.len() == 1 {
        Ok(matches.remove(0))
    } else if matches.is_empty() {
        Ok(vec![])
    } else {
        Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004005u32 as i32,
        )))
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek(), b.peek()) {
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let x: String = std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                let y: String = std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                let (xn, yn) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let order = xn
                    .len()
                    .cmp(&yn.len())
                    .then(xn.cmp(yn))
                    .then(x.len().cmp(&y.len()));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(_), Some(_)) => {
                let order = a.next().cmp(&b.next());
                if order != Ordering::Equal {
                    return order;
                }
            }
            _ => return a.next().cmp(&b.next()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_request_cannot_replace_newer_or_invalidated_context() {
        let state = InvocationState::default();
        let old = capture(&state, 0);
        let new = capture(&state, 0);
        assert!(!state.publish(&old));
        assert_eq!(
            state.current.lock().unwrap().as_ref().unwrap().request_id,
            new.request_id
        );
        state.invalidate();
        assert!(!state.publish(&new));
        assert!(state.current.lock().unwrap().is_none());
    }
    #[test]
    fn natural_file_order_handles_large_numbers() {
        assert!(natural_cmp("文件2.txt", "文件10.txt").is_lt());
        assert!(natural_cmp("a999999999999999999999", "a1000000000000000000000").is_lt());
    }
    #[test]
    fn non_explorer_invocation_clears_old_selection() {
        let state = InvocationState::default();
        *state.current.lock().unwrap() = Some(SelectionContext {
            request_id: 0,
            paths: vec!["old".into()],
            status: "ready".into(),
            message: None,
        });
        let context = capture(&state, 0);
        assert_eq!(context.request_id, 1);
        assert!(context.paths.is_empty());
        assert!(state
            .current
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .paths
            .is_empty());
    }
}
