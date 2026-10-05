use super::ClipboardItem;
use std::borrow::Cow;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub(crate) enum CapturedContent {
    Text(String),
    Image {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Files(Vec<String>),
}

/// Prefer CF_HDROP to the text fallback some file managers also publish.
pub(crate) fn capture() -> Result<Option<CapturedContent>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    for attempt in 0..6 {
        #[cfg(windows)]
        if let Some(paths) = super::native::read_files()? {
            return Ok(Some(CapturedContent::Files(paths)));
        }
        let mut image_error = None;
        match clipboard.get_image() {
            Ok(image) => {
                let width = u32::try_from(image.width).map_err(|_| "剪贴板图片宽度过大")?;
                let height = u32::try_from(image.height).map_err(|_| "剪贴板图片高度过大")?;
                if width == 0 || height == 0 {
                    return Ok(None);
                }
                return Ok(Some(CapturedContent::Image {
                    width,
                    height,
                    rgba: image.bytes.into_owned(),
                }));
            }
            Err(arboard::Error::ClipboardOccupied) if attempt < 5 => {
                thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(arboard::Error::ContentNotAvailable) => {}
            Err(arboard::Error::ClipboardOccupied) => return Err("剪贴板暂时被其他程序占用".into()),
            // Some applications also publish text alongside an image. Preserve
            // the image error for logging only when no text can be recovered.
            Err(error) => image_error = Some(error.to_string()),
        }
        match clipboard.get_text() {
            Ok(text) if !text.is_empty() => return Ok(Some(CapturedContent::Text(text))),
            Ok(_) | Err(arboard::Error::ContentNotAvailable) if attempt < 5 => {
                // Windows can expose a short empty interval while the producing
                // process publishes clipboard formats. Retry only in response
                // to this update event; the listener never polls while idle.
                thread::sleep(Duration::from_millis(20));
            }
            Ok(_) | Err(arboard::Error::ContentNotAvailable) => {
                return match image_error {
                    Some(error) => Err(format!("无法读取剪贴板图片：{error}")),
                    None => Ok(None),
                };
            }
            Err(arboard::Error::ClipboardOccupied) if attempt < 5 => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(None)
}

pub(crate) fn write_item(item: &ClipboardItem) -> Result<(), String> {
    match item.item_type.as_str() {
        "text" => set_text(item.text_content.as_deref().ok_or("文本记录内容缺失")?),
        "files" => {
            let paths: Vec<String> =
                serde_json::from_str(item.text_content.as_deref().ok_or("文件路径记录内容缺失")?)
                    .map_err(|error| format!("文件路径记录无效：{error}"))?;
            #[cfg(windows)]
            {
                super::native::write_files(&paths)
            }
            #[cfg(not(windows))]
            {
                let _ = paths;
                Err("文件剪贴板仅支持 Windows".into())
            }
        }
        "image" => {
            let path = item.data_path.as_deref().ok_or("图片记录路径缺失")?;
            let rgba = image::open(path)
                .map_err(|error| format!("无法读取剪贴板图片：{error}"))?
                .into_rgba8();
            let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
            for attempt in 0..6 {
                match clipboard.set_image(arboard::ImageData {
                    width: rgba.width() as usize,
                    height: rgba.height() as usize,
                    bytes: Cow::Borrowed(rgba.as_raw()),
                }) {
                    Ok(()) => return Ok(()),
                    Err(arboard::Error::ClipboardOccupied) if attempt < 5 => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
            Err("无法写入剪贴板图片".into())
        }
        other => Err(format!("不支持的剪贴板记录类型：{other}")),
    }
}

pub(crate) fn set_text(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    for attempt in 0..6 {
        match clipboard.set_text(text) {
            Ok(()) => return Ok(()),
            Err(arboard::Error::ClipboardOccupied) if attempt < 5 => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("无法写入剪贴板".into())
}
