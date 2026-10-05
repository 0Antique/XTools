//! The clipboard owns an HGLOBAL after successful SetClipboardData. Keep this
//! small FFI boundary explicit so its ownership cannot escape into storage.
use std::ffi::c_void;
use std::mem::size_of;
use std::thread;
use std::time::Duration;

pub(crate) type Handle = isize;
pub(crate) const CF_HDROP: u32 = 15;
pub(crate) const WM_CLIPBOARDUPDATE: u32 = 0x031D;
pub(crate) const WM_CLOSE: u32 = 0x0010;
pub(crate) const WM_DESTROY: u32 = 0x0002;
pub(crate) const WM_NCCREATE: u32 = 0x0081;
pub(crate) const GWLP_USERDATA: i32 = -21;
pub(crate) const HWND_MESSAGE: Handle = -3;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub(crate) struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Default)]
pub(crate) struct Message {
    pub hwnd: Handle,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub point: Point,
    pub private: u32,
}

#[repr(C)]
pub(crate) struct WindowClass {
    pub size: u32,
    pub style: u32,
    pub window_proc: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
    pub class_extra: i32,
    pub window_extra: i32,
    pub instance: Handle,
    pub icon: Handle,
    pub cursor: Handle,
    pub background: Handle,
    pub menu_name: *const u16,
    pub class_name: *const u16,
    pub small_icon: Handle,
}

#[repr(C)]
struct DropFiles {
    files_offset: u32,
    point: Point,
    non_client: i32,
    wide: i32,
}

#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(owner: Handle) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn GetClipboardData(format: u32) -> Handle;
    fn SetClipboardData(format: u32, data: Handle) -> Handle;
    pub(crate) fn AddClipboardFormatListener(window: Handle) -> i32;
    pub(crate) fn RemoveClipboardFormatListener(window: Handle) -> i32;
    pub(crate) fn RegisterClassExW(class: *const WindowClass) -> u16;
    pub(crate) fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: *const c_void,
    ) -> Handle;
    pub(crate) fn DestroyWindow(window: Handle) -> i32;
    pub(crate) fn DefWindowProcW(
        window: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
    pub(crate) fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    pub(crate) fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    pub(crate) fn GetMessageW(message: *mut Message, window: Handle, min: u32, max: u32) -> i32;
    pub(crate) fn TranslateMessage(message: *const Message) -> i32;
    pub(crate) fn DispatchMessageW(message: *const Message) -> isize;
    pub(crate) fn PostMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
    pub(crate) fn PostQuitMessage(exit_code: i32);
}

#[link(name = "kernel32")]
extern "system" {
    pub(crate) fn GetModuleHandleW(name: *const u16) -> Handle;
    fn GlobalAlloc(flags: u32, bytes: usize) -> Handle;
    fn GlobalLock(handle: Handle) -> *mut c_void;
    fn GlobalUnlock(handle: Handle) -> i32;
    fn GlobalFree(handle: Handle) -> Handle;
}

#[link(name = "shell32")]
extern "system" {
    fn DragQueryFileW(drop: Handle, index: u32, path: *mut u16, length: u32) -> u32;
}

pub(crate) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

struct ClipboardGuard;
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

fn open(owner: Handle) -> Result<ClipboardGuard, String> {
    for attempt in 0..8 {
        if unsafe { OpenClipboard(owner) } != 0 {
            return Ok(ClipboardGuard);
        }
        if attempt < 7 {
            thread::sleep(Duration::from_millis(15));
        }
    }
    Err("剪贴板暂时被其他程序占用".into())
}

pub(crate) fn read_files() -> Result<Option<Vec<String>>, String> {
    if unsafe { IsClipboardFormatAvailable(CF_HDROP) } == 0 {
        return Ok(None);
    }
    let _clipboard = open(0)?;
    let handle = unsafe { GetClipboardData(CF_HDROP) };
    if handle == 0 {
        return Err("无法读取文件剪贴板".into());
    }
    let count = unsafe { DragQueryFileW(handle, u32::MAX, std::ptr::null_mut(), 0) };
    let mut files = Vec::with_capacity(count as usize);
    for index in 0..count {
        let length = unsafe { DragQueryFileW(handle, index, std::ptr::null_mut(), 0) };
        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { DragQueryFileW(handle, index, buffer.as_mut_ptr(), length + 1) };
        if copied > 0 {
            files.push(String::from_utf16_lossy(&buffer[..copied as usize]));
        }
    }
    if files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(files))
    }
}

pub(crate) fn write_files(paths: &[String]) -> Result<(), String> {
    if paths.is_empty()
        || paths
            .iter()
            .any(|path| path.is_empty() || path.contains('\0'))
    {
        return Err("文件路径列表无效".into());
    }
    let mut characters = Vec::<u16>::new();
    for path in paths {
        characters.extend(path.encode_utf16());
        characters.push(0);
    }
    characters.push(0);
    let bytes = size_of::<DropFiles>()
        .checked_add(characters.len().checked_mul(2).ok_or("文件路径列表过大")?)
        .ok_or("文件路径列表过大")?;
    let memory = unsafe { GlobalAlloc(0x0002, bytes) }; // GMEM_MOVEABLE
    if memory == 0 {
        return Err("无法分配文件剪贴板内存".into());
    }
    let target = unsafe { GlobalLock(memory) };
    if target.is_null() {
        unsafe {
            GlobalFree(memory);
        }
        return Err("无法写入文件剪贴板内存".into());
    }
    unsafe {
        std::ptr::write(
            target.cast::<DropFiles>(),
            DropFiles {
                files_offset: size_of::<DropFiles>() as u32,
                point: Point::default(),
                non_client: 0,
                wide: 1,
            },
        );
        std::ptr::copy_nonoverlapping(
            characters.as_ptr(),
            target
                .cast::<u8>()
                .add(size_of::<DropFiles>())
                .cast::<u16>(),
            characters.len(),
        );
        GlobalUnlock(memory);
    }

    // EmptyClipboard must receive a real owner, otherwise SetClipboardData can
    // fail. A hidden built-in STATIC window needs no custom registration.
    let class = wide("STATIC");
    let owner = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            0,
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    if owner == 0 {
        unsafe {
            GlobalFree(memory);
        }
        return Err("无法创建文件剪贴板所有者".into());
    }
    let result = (|| {
        let _clipboard = open(owner)?;
        if unsafe { EmptyClipboard() } == 0 {
            return Err("无法清空系统剪贴板".into());
        }
        if unsafe { SetClipboardData(CF_HDROP, memory) } == 0 {
            return Err("无法写入文件剪贴板".into());
        }
        Ok(())
    })();
    unsafe {
        DestroyWindow(owner);
    }
    // Success transfers ownership to Windows. Failure leaves it with us.
    if result.is_err() {
        unsafe {
            GlobalFree(memory);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_ffi_structs_match_windows_x64_layout() {
        assert_eq!(size_of::<DropFiles>(), 20);
        assert_eq!(size_of::<Message>(), 48);
        assert_eq!(size_of::<WindowClass>(), 80);
    }
}
