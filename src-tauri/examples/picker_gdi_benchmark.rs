//! Read-only GDI sampling comparison; no input injection or screenshots saved.
use std::time::Instant;
use windows::Win32::{Foundation::POINT, Graphics::Gdi::*, UI::WindowsAndMessaging::GetCursorPos};
fn main() -> windows::core::Result<()> {
    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point)?;
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 9,
                biHeight: -9,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let original = SelectObject(dc, bitmap.into());
        let mut old = Vec::new();
        let mut new = Vec::new();
        let mut checksum = 0u32;
        for round in 0..1100 {
            let start = Instant::now();
            BitBlt(
                dc,
                0,
                0,
                9,
                9,
                Some(screen),
                point.x - 4,
                point.y - 4,
                SRCCOPY,
            )?;
            for y in 0..9 {
                for x in 0..9 {
                    checksum ^= GetPixel(dc, x, y).0;
                }
            }
            if round >= 100 {
                old.push(start.elapsed().as_secs_f64() * 1000.);
            }
            let start = Instant::now();
            BitBlt(
                dc,
                0,
                0,
                9,
                9,
                Some(screen),
                point.x - 4,
                point.y - 4,
                SRCCOPY,
            )?;
            let _ = GdiFlush();
            for pixel in std::slice::from_raw_parts(bits.cast::<u8>(), 9 * 9 * 4)
                .as_chunks::<4>()
                .0
            {
                checksum ^= pixel[2] as u32 | ((pixel[1] as u32) << 8) | ((pixel[0] as u32) << 16);
            }
            if round >= 100 {
                new.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        SelectObject(dc, original);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        std::hint::black_box(checksum);
        fn stats(mut values: Vec<f64>) -> serde_json::Value {
            let average = values.iter().sum::<f64>() / values.len() as f64;
            values.sort_by(f64::total_cmp);
            serde_json::json!({"meanMs":average,"p95Ms":values[(values.len()*95/100).min(values.len()-1)]})
        }
        println!(
            "{}",
            serde_json::json!({"samplesPerPath":1000,"v1GetPixel81":stats(old),"v2DibMemory":stats(new)})
        );
    }
    Ok(())
}
