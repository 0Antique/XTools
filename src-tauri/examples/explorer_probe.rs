//! STA Shell selection probe. Select fixture files in Explorer before running.
#![allow(dead_code)]
#[path = "../src/windows/explorer_selection.rs"]
mod explorer_selection;
fn main() {
    let source = if let Some(title) = std::env::args().nth(1) {
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                windows::core::w!("CabinetWClass"),
                windows::core::PCWSTR(title.as_ptr()),
            )
            .map(|h| h.0 as isize)
            .unwrap_or(0)
        }
    } else {
        explorer_selection::foreground()
    };
    let context =
        explorer_selection::capture(&explorer_selection::InvocationState::default(), source);
    println!(
        "{}",
        serde_json::json!({"status": context.status,"selectedCount":context.paths.len(),"names":context.paths.iter().map(|p| std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy()).collect::<Vec<_>>(),"message":context.message})
    );
}
