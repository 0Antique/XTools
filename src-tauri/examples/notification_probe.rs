//! Tests the exact production WinRT sender against an installed XTools AUMID.
#![allow(dead_code)]
pub fn log_error(_: &tauri::AppHandle, text: &str) {
    eprintln!("{text}");
}
#[path = "../src/notifications.rs"]
mod notifications;
fn main() -> Result<(), String> {
    notifications::send_color("#FFFFFF")?;
    println!("Native XTools short toast accepted by Windows");
    Ok(())
}
