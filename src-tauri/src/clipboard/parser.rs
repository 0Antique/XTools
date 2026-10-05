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

const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn decode_image(bytes: &[u8]) -> Result<CapturedContent, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| format!("无法解码剪贴板图片：{e}"))?;
    if decoded.width() as u64 * decoded.height() as u64 * 4 > 128 * 1024 * 1024 {
        return Err("剪贴板图片解码内存过大".into());
    }
    let rgba = decoded.into_rgba8();
    Ok(CapturedContent::Image {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}
pub(crate) fn wechat_temp_path(path: &str) -> bool {
    let path = path.replace('/', "\\").to_lowercase();
    (path.contains("\\xwechat_files\\") && path.contains("\\temp\\rwtemp\\"))
        || (path.contains("\\wechat files\\")
            && (path.contains("\\temp\\") || path.contains("\\image\\temp\\")))
}
pub(crate) fn decode_file(path: &str) -> Result<CapturedContent, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|_| "原图片已失效，请重新复制")?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if size == 0 || size > MAX_IMAGE_BYTES {
        return Err("图片文件过大或为空".into());
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err("图片文件过大".into());
    }
    decode_image(&bytes)
}

pub(crate) fn capture() -> Result<Option<CapturedContent>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    for attempt in 0..6 {
        let before = unsafe { super::native::GetClipboardSequenceNumber() };
        let result = capture_snapshot(&mut clipboard);
        let after = unsafe { super::native::GetClipboardSequenceNumber() };
        if before == after {
            match result {
                Ok(Some(content)) => return Ok(Some(content)),
                Ok(None) if attempt == 5 => return Ok(None),
                Err(error) if attempt == 5 => return Err(error),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("剪贴板更新过快，请重新复制".into())
}
fn capture_snapshot(clipboard: &mut arboard::Clipboard) -> Result<Option<CapturedContent>, String> {
    let owner = super::native::owner_name();
    let files = super::native::read_files()?;
    if let Some(paths) = files.as_ref() {
        if paths.len() > 1 || paths.iter().any(|p| std::path::Path::new(p).is_dir()) {
            return Ok(Some(CapturedContent::Files(paths.clone())));
        }
    }
    // Explorer's file-copy semantics take precedence over auxiliary image formats.
    if owner == "explorer.exe" {
        if let Some(paths) = files.as_ref() {
            return Ok(Some(CapturedContent::Files(paths.clone())));
        }
    }
    if let Some(bytes) = super::native::read_png()? {
        return decode_image(&bytes).map(Some);
    }
    super::native::validate_bitmap_size()?;
    match clipboard.get_image() {
        Ok(image) => {
            let width = u32::try_from(image.width).map_err(|_| "图片宽度无效")?;
            let height = u32::try_from(image.height).map_err(|_| "图片高度无效")?;
            if width == 0
                || height == 0
                || width > 16384
                || height > 16384
                || image.bytes.len() > 128 * 1024 * 1024
            {
                return Err("图片尺寸过大或无效".into());
            }
            return Ok(Some(CapturedContent::Image {
                width,
                height,
                rgba: image.bytes.into_owned(),
            }));
        }
        Err(arboard::Error::ClipboardOccupied) => return Err("剪贴板暂时被占用".into()),
        Err(arboard::Error::ContentNotAvailable) => {}
        Err(error) => log::warn!("剪贴板位图解码失败：{error}"),
    }
    if let Some(paths) = files {
        if paths.len() == 1
            && matches!(owner.as_str(), "wechat.exe" | "weixin.exe")
            && wechat_temp_path(&paths[0])
        {
            // Only a validated producer + temp location + content decoder can
            // turn a file payload into a WeChat image. Never infer by suffix.
            match decode_file(&paths[0]) {
                Ok(image) => return Ok(Some(image)),
                Err(error) => log::warn!("微信临时图片读取失败：{error}"),
            }
        }
        return Ok(Some(CapturedContent::Files(paths)));
    }
    match clipboard.get_text() {
        Ok(text) if !text.is_empty() => Ok(Some(CapturedContent::Text(text))),
        Ok(_) | Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_distinctive_wechat_temp_paths_qualify() {
        assert!(wechat_temp_path(
            r"D:\data\xwechat_files\wxid_test\temp\RWTemp\image.dat"
        ));
        assert!(!wechat_temp_path(r"D:\Pictures\wechat-photo.png"));
        assert!(!wechat_temp_path(r"C:\Temp\photo.png"));
        assert!(!wechat_temp_path(
            r"D:\data\xwechat_files\wxid_test\msg\file\photo.png"
        ));
    }
    #[test]
    fn file_decoder_uses_content_and_rejects_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("no-extension");
        let original = image::RgbaImage::from_pixel(2, 3, image::Rgba([11, 22, 33, 128]));
        original
            .save_with_format(&file, image::ImageFormat::Png)
            .unwrap();
        match decode_file(file.to_str().unwrap()).unwrap() {
            CapturedContent::Image {
                width,
                height,
                rgba,
            } => {
                assert_eq!((width, height), (2, 3));
                assert_eq!(rgba, original.into_raw());
            }
            _ => panic!("expected image"),
        }
        assert!(decode_image(b"not an image").is_err());
        assert!(decode_file(temp.path().join("missing").to_str().unwrap()).is_err());
    }
}
