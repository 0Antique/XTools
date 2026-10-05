use super::Application;

#[cfg(windows)]
pub fn launch(app: &Application) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::{
            System::Com::{CoAllowSetForegroundWindow, CoCreateInstance, CLSCTX_INPROC_SERVER},
            UI::{
                Shell::{
                    ApplicationActivationManager, IApplicationActivationManager, ShellExecuteExW,
                    AO_NONE, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
                },
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        },
    };
    let _apartment = crate::windows::shell::ComApartment::initialize()?;
    let target = crate::windows::wide(&app.launch_target);
    if app.launch_type == "uwp" {
        let arguments = crate::windows::wide(app.arguments.as_deref().unwrap_or(""));
        let manager: IApplicationActivationManager =
            unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| format!("无法初始化应用激活器：{e}"))?;
        unsafe {
            let _ = CoAllowSetForegroundWindow(&manager, None);
        }
        unsafe {
            manager.ActivateApplication(
                PCWSTR(target.as_ptr()),
                PCWSTR(arguments.as_ptr()),
                AO_NONE,
            )
        }
        .map_err(|e| format!("无法启动 {}：{e}", app.name))?;
        return Ok(());
    }
    if !matches!(app.launch_type.as_str(), "executable" | "shortcut") {
        return Err("未知的应用启动方式".into());
    }
    if !std::path::Path::new(&app.launch_target).is_file() {
        return Err(format!(
            "应用路径已不存在，请重新扫描：{}",
            app.launch_target
        ));
    }
    // Shell follows shortcut parameters and working directory itself. Passing
    // the extracted arguments again would launch some shortcuts incorrectly.
    let arguments = if app.launch_type == "shortcut" {
        None
    } else {
        app.arguments.as_deref().map(crate::windows::wide)
    };
    let directory = if app.launch_type == "shortcut" {
        None
    } else {
        app.working_directory.as_deref().map(crate::windows::wide)
    };
    let verb = crate::windows::wide("open");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(target.as_ptr()),
        lpParameters: arguments
            .as_ref()
            .map(|s| PCWSTR(s.as_ptr()))
            .unwrap_or(PCWSTR::null()),
        lpDirectory: directory
            .as_ref()
            .map(|s| PCWSTR(s.as_ptr()))
            .unwrap_or(PCWSTR::null()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| format!("无法启动 {}：{e}", app.name))
}

#[cfg(not(windows))]
pub fn launch(_: &Application) -> Result<(), String> {
    Err("应用启动仅支持 Windows".into())
}
