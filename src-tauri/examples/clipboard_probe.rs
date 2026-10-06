//! Read-only clipboard integration probe. Never prints content or private paths.
#![allow(dead_code)]
#[path = "../src/clipboard/native.rs"]
mod native;
#[path = "../src/clipboard/parser.rs"]
mod parser;
#[path = "../src/clipboard/storage.rs"]
mod storage;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: i64,
    pub item_type: String,
    pub text_content: Option<String>,
    pub data_path: Option<String>,
    pub preview: String,
    pub created_at: i64,
    pub is_favorite: bool,
}
fn main() -> Result<(), String> {
    let folder = std::env::args()
        .nth(1)
        .ok_or("Provide an isolated output directory")?;
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let sequence = unsafe { native::GetClipboardSequenceNumber() };
    let owner = native::owner_name();
    let files = native::read_files()?.map(|p| p.len()).unwrap_or(0);
    let has_png = native::read_png()?.is_some();
    let content = parser::capture()?.ok_or("Clipboard has no supported content")?;
    let mut db = xtools_lib::database::open(&folder.join("probe.db"))?;
    let dimensions = match &content {
        parser::CapturedContent::Image { width, height, .. } => Some((*width, *height)),
        _ => None,
    };
    let id = storage::record(&mut db, content, &folder.join("images"), 100)?;
    let stored = storage::get(&db, id)?;
    let reopened = xtools_lib::database::open(&folder.join("probe.db"))?;
    let durable = storage::get(&reopened, id)?;
    let persisted = durable
        .data_path
        .as_ref()
        .is_some_and(|p| image::open(p).is_ok());
    println!(
        "{}",
        serde_json::json!({"owner":owner,"sequenceStable":sequence == unsafe {native::GetClipboardSequenceNumber()},"fileCount":files,"pngFormat":has_png,"capturedType":stored.item_type,"dimensions":dimensions,"persistentImageReadable":persisted})
    );
    Ok(())
}
