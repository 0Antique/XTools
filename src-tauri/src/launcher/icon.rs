use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn cache_icon(
    icon_dir: &Path,
    unique_key: &str,
    parsing_name: &str,
    icon_location: Option<&str>,
    icon_index: i32,
) -> Option<String> {
    let path = icon_dir.join(format!("{:x}.png", Sha256::digest(unique_key.as_bytes())));
    if path.is_file() && path.metadata().ok()?.len() > 8 {
        return Some(path.to_string_lossy().into_owned());
    }
    std::fs::create_dir_all(icon_dir).ok()?;
    #[cfg(windows)]
    {
        let pixels = icon_location
            .and_then(|location| native::resource_icon(location, icon_index))
            .or_else(|| native::shell_bitmap(parsing_name))?;
        save(&path, pixels.0, pixels.1, &pixels.2)?;
        Some(path.to_string_lossy().into_owned())
    }
    #[cfg(not(windows))]
    {
        let _ = (parsing_name, icon_location, icon_index, path);
        None
    }
}

#[cfg(windows)]
fn save(path: &Path, width: u32, height: u32, bytes: &[u8]) -> Option<()> {
    let temporary: PathBuf = path.with_extension("png.tmp");
    if image::save_buffer_with_format(
        &temporary,
        bytes,
        width,
        height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .is_err()
    {
        let _ = std::fs::remove_file(&temporary);
        return None;
    }
    match std::fs::rename(&temporary, path) {
        Ok(()) => Some(()),
        Err(_) => {
            let _ = std::fs::remove_file(&temporary);
            None
        }
    }
}

#[cfg(windows)]
mod native {
    use std::mem::size_of;
    use windows::{
        core::{Interface, PCWSTR},
        Win32::{
            Foundation::SIZE,
            Graphics::Gdi::{
                DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
                BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
            },
            UI::{
                Shell::{
                    ExtractIconExW, IShellItemImageFactory, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
                },
                WindowsAndMessaging::{
                    DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO,
                },
            },
        },
    };
    type Pixels = (u32, u32, Vec<u8>);
    struct Bitmap(HBITMAP);
    impl Drop for Bitmap {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0 .0));
            }
        }
    }
    struct Icon(HICON);
    impl Drop for Icon {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }

    fn bitmap_pixels(bitmap: HBITMAP) -> Option<Pixels> {
        let mut details = BITMAP::default();
        if unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                size_of::<BITMAP>() as i32,
                Some((&mut details as *mut BITMAP).cast()),
            )
        } == 0
        {
            return None;
        }
        let width = details.bmWidth.unsigned_abs();
        let height = details.bmHeight.unsigned_abs();
        if width == 0 || height == 0 || width > 1024 || height > 2048 {
            return None;
        }
        let mut bytes = vec![0u8; width as usize * height as usize * 4];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = unsafe { GetDC(None) };
        if dc.0.is_null() {
            return None;
        }
        let copied = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                height,
                Some(bytes.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        unsafe {
            ReleaseDC(None, dc);
        }
        if copied != height as i32 {
            return None;
        }
        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        Some((width, height, bytes))
    }

    fn straight_alpha(bytes: &mut [u8]) {
        let has_alpha = bytes.chunks_exact(4).any(|pixel| pixel[3] != 0);
        for pixel in bytes.chunks_exact_mut(4) {
            if !has_alpha {
                pixel[3] = 255;
            } else if pixel[3] > 0 && pixel[3] < 255 {
                // Shell bitmaps use premultiplied BGRA. PNG expects straight RGBA.
                let alpha = pixel[3] as u16;
                for channel in &mut pixel[..3] {
                    *channel = ((*channel as u16 * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }

    pub fn shell_bitmap(parsing_name: &str) -> Option<Pixels> {
        let item = crate::windows::shell::shell_item(parsing_name).ok()?;
        let factory: IShellItemImageFactory = item.cast().ok()?;
        let bitmap = Bitmap(
            unsafe {
                factory.GetImage(
                    SIZE { cx: 64, cy: 64 },
                    SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
                )
            }
            .ok()?,
        );
        let mut pixels = bitmap_pixels(bitmap.0)?;
        straight_alpha(&mut pixels.2);
        Some(pixels)
    }

    pub fn resource_icon(location: &str, index: i32) -> Option<Pixels> {
        let location = crate::windows::wide(location);
        let mut handle = HICON::default();
        if location.len() <= 260 {
            let mut filename = [0u16; 260];
            filename[..location.len()].copy_from_slice(&location);
            let mut icons = [HICON::default()];
            unsafe {
                PrivateExtractIconsW(&filename, index, 64, 64, Some(&mut icons), None, 0);
            }
            handle = icons[0];
        }
        if handle.0.is_null() {
            unsafe {
                ExtractIconExW(PCWSTR(location.as_ptr()), index, Some(&mut handle), None, 1);
            }
        }
        if handle.0.is_null() {
            return None;
        }
        let icon = Icon(handle);
        let mut info = ICONINFO::default();
        unsafe { GetIconInfo(icon.0, &mut info) }.ok()?;
        let color = Bitmap(info.hbmColor);
        let mask = Bitmap(info.hbmMask);
        let mut pixels = bitmap_pixels(color.0)?;
        if !pixels.2.chunks_exact(4).any(|pixel| pixel[3] > 0) {
            if let Some((mask_width, mask_height, mask_pixels)) = bitmap_pixels(mask.0) {
                if mask_width == pixels.0 && mask_height == pixels.1 {
                    for (pixel, mask) in pixels
                        .2
                        .chunks_exact_mut(4)
                        .zip(mask_pixels.chunks_exact(4))
                    {
                        pixel[3] = if mask[0] == 0 { 255 } else { 0 };
                    }
                    return Some(pixels);
                }
            }
        }
        straight_alpha(&mut pixels.2);
        Some(pixels)
    }
}
