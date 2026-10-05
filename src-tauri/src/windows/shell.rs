#[cfg(windows)]
mod native {
    use crate::windows::wide;
    use std::path::PathBuf;
    use windows::{
        core::{Interface, GUID, PCWSTR, PWSTR},
        Win32::{
            Foundation::PROPERTYKEY,
            System::Com::{
                CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                BHID_EnumItems, IEnumShellItems, IShellItem, IShellItem2,
                SHCreateItemFromParsingName, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
                SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
            },
        },
    };

    pub struct ComApartment(bool);
    impl ComApartment {
        pub fn initialize() -> Result<Self, String> {
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            // RPC_E_CHANGED_MODE: this thread already owns a different valid
            // apartment. Do not balance somebody else's initialization.
            if result.0 == 0x80010106u32 as i32 {
                return Ok(Self(false));
            }
            result
                .ok()
                .map_err(|e| format!("无法初始化 Windows Shell：{e}"))?;
            Ok(Self(true))
        }
    }
    impl Drop for ComApartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    pub fn take_string(value: PWSTR) -> String {
        if value.is_null() {
            return String::new();
        }
        let result = unsafe { value.to_string().unwrap_or_default() };
        unsafe {
            CoTaskMemFree(Some(value.0.cast()));
        }
        result
    }

    pub fn known_folder(id: &GUID) -> Option<PathBuf> {
        unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }
            .ok()
            .map(take_string)
            .map(PathBuf::from)
    }

    pub fn shell_item(path: &str) -> windows::core::Result<IShellItem> {
        let path = wide(path);
        unsafe { SHCreateItemFromParsingName(PCWSTR(path.as_ptr()), None) }
    }

    pub fn app_user_model_id(item: &IShellItem) -> Option<String> {
        const APP_ID: PROPERTYKEY = PROPERTYKEY {
            fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
            pid: 5,
        };
        let properties: IShellItem2 = item.cast().ok()?;
        let value = unsafe { properties.GetString(&APP_ID) }
            .ok()
            .map(take_string)?;
        (!value.is_empty()).then_some(value)
    }

    pub struct StoreApplication {
        pub name: String,
        pub aumid: String,
    }

    /// Enumerate the Shell's AppsFolder directly; this needs no PowerShell,
    /// temporary scripts, package access permissions, or background process.
    pub fn store_applications() -> Result<Vec<StoreApplication>, String> {
        let folder =
            shell_item("shell:AppsFolder").map_err(|e| format!("无法读取应用文件夹：{e}"))?;
        let enumeration: IEnumShellItems =
            unsafe { folder.BindToHandler(None, &BHID_EnumItems) }
                .map_err(|e| format!("无法枚举 Microsoft Store 应用：{e}"))?;
        let mut result = Vec::new();
        for _ in 0..20_000 {
            let mut items = [None];
            let mut fetched = 0;
            let next = unsafe { enumeration.Next(&mut items, Some(&mut fetched)) };
            if fetched == 0 {
                break;
            }
            next.map_err(|e| format!("读取应用文件夹失败：{e}"))?;
            let Some(item) = items[0].take() else {
                continue;
            };
            let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
                .ok()
                .map(take_string)
                .unwrap_or_default();
            let parsing_name = unsafe { item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) }
                .ok()
                .map(take_string)
                .unwrap_or_default();
            let aumid = app_user_model_id(&item).or_else(|| {
                parsing_name
                    .rsplit('\\')
                    .next()
                    .filter(|v| v.contains('!'))
                    .map(str::to_string)
            });
            // The same folder also contains classic applications. Those are
            // indexed from actual shortcuts/executables to deduplicate by path.
            if let Some(aumid) = aumid.filter(|v| v.contains('!')) {
                if !name.trim().is_empty() {
                    result.push(StoreApplication { name, aumid });
                }
            }
        }
        Ok(result)
    }
}

#[cfg(windows)]
pub(crate) use native::*;
