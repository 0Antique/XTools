#[derive(Debug)]
pub struct Shortcut {
    pub target: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    pub icon_location: Option<String>,
    pub icon_index: i32,
    pub aumid: Option<String>,
}

#[cfg(windows)]
pub fn resolve(path: &std::path::Path) -> Result<Shortcut, String> {
    use crate::windows::{from_wide, wide};
    use windows::{
        core::{Interface, PCWSTR},
        Win32::{
            System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ},
            UI::Shell::{IShellLinkW, ShellLink},
        },
    };
    let name = path.to_string_lossy();
    let path_wide = wide(&name);
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| e.to_string())?;
    let file: IPersistFile = link.cast().map_err(|e| e.to_string())?;
    unsafe { file.Load(PCWSTR(path_wide.as_ptr()), STGM_READ) }.map_err(|e| e.to_string())?;
    let mut target = vec![0u16; 32_768];
    let mut arguments = vec![0u16; 32_768];
    let mut directory = vec![0u16; 32_768];
    let mut icon = vec![0u16; 32_768];
    let mut icon_index = 0;
    unsafe {
        let _ = link.GetPath(&mut target, std::ptr::null_mut(), 0);
        let _ = link.GetArguments(&mut arguments);
        let _ = link.GetWorkingDirectory(&mut directory);
        let _ = link.GetIconLocation(&mut icon, &mut icon_index);
    }
    let nonempty = |value: &[u16]| {
        let text = from_wide(value);
        (!text.trim().is_empty()).then_some(text)
    };
    let arguments = nonempty(&arguments);
    let mut aumid = super::shell::shell_item(&name)
        .ok()
        .and_then(|item| super::shell::app_user_model_id(&item));
    if aumid.is_none() {
        if let Some(args) = &arguments {
            if let Some(offset) = args.to_lowercase().find("shell:appsfolder\\") {
                let id = args[offset + "shell:appsfolder\\".len()..]
                    .trim()
                    .trim_matches('"');
                if id.contains('!') {
                    aumid = Some(id.into());
                }
            }
        }
    }
    Ok(Shortcut {
        target: from_wide(&target),
        arguments,
        working_directory: nonempty(&directory),
        icon_location: nonempty(&icon),
        icon_index,
        aumid,
    })
}

#[cfg(not(windows))]
pub fn resolve(_: &std::path::Path) -> Result<Shortcut, String> {
    Err("应用快捷方式仅支持 Windows".into())
}
