#[cfg(windows)]
mod implementation {
    use super::super::Color;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::Emitter;

    type Handle = isize;
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    const WM_PICK: u32 = 0x8001;
    const WM_CLOSE: u32 = 0x0010;
    const WM_PAINT: u32 = 0x000F;
    const WM_TIMER: u32 = 0x0113;
    const WM_DESTROY: u32 = 0x0002;
    const WM_NCCREATE: u32 = 0x0081;
    const GWLP_USERDATA: i32 = -21;
    const GRID: i32 = 9;
    const RADIUS: i32 = GRID / 2;

    #[repr(C)]
    #[derive(Default, Copy, Clone)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    #[derive(Default, Copy, Clone)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Message {
        hwnd: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        point: Point,
        private: u32,
    }
    #[repr(C)]
    struct WindowClass {
        size: u32,
        style: u32,
        window_proc: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
        class_extra: i32,
        window_extra: i32,
        instance: Handle,
        icon: Handle,
        cursor: Handle,
        background: Handle,
        menu_name: *const u16,
        class_name: *const u16,
        small_icon: Handle,
    }
    #[repr(C)]
    #[derive(Default)]
    struct PaintStruct {
        dc: Handle,
        erase: i32,
        rect: Rect,
        restore: i32,
        update: i32,
        reserved: [u8; 32],
    }
    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }
    #[repr(C)]
    struct LowLevelMouse {
        point: Point,
        mouse_data: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }
    #[repr(C)]
    struct LowLevelKeyboard {
        virtual_key: u32,
        scan_code: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SetThreadDpiAwarenessContext(context: Handle) -> Handle;
        fn RegisterClassExW(class: *const WindowClass) -> u16;
        fn CreateWindowExW(
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
        fn DestroyWindow(window: Handle) -> i32;
        fn DefWindowProcW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
        fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
        fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
        fn GetMessageW(message: *mut Message, window: Handle, min: u32, max: u32) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
        fn PostMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
        fn PostQuitMessage(exit_code: i32);
        fn SetTimer(window: Handle, event: usize, elapsed: u32, callback: *const c_void) -> usize;
        fn KillTimer(window: Handle, event: usize) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn GetWindowRect(window: Handle, rect: *mut Rect) -> i32;
        fn ShowWindow(window: Handle, command: i32) -> i32;
        fn GetDC(window: Handle) -> Handle;
        fn ReleaseDC(window: Handle, dc: Handle) -> i32;
        fn MonitorFromPoint(point: Point, flags: u32) -> Handle;
        fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
        fn GetDpiForWindow(window: Handle) -> u32;
        fn SetWindowPos(
            window: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn InvalidateRect(window: Handle, rect: *const Rect, erase: i32) -> i32;
        fn BeginPaint(window: Handle, paint: *mut PaintStruct) -> Handle;
        fn EndPaint(window: Handle, paint: *const PaintStruct) -> i32;
        fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
        fn FrameRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
        fn DrawTextW(
            dc: Handle,
            text: *const u16,
            length: i32,
            rect: *mut Rect,
            format: u32,
        ) -> i32;
        fn SetWindowsHookExW(
            id: i32,
            callback: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>,
            module: Handle,
            thread: u32,
        ) -> Handle;
        fn UnhookWindowsHookEx(hook: Handle) -> i32;
        fn CallNextHookEx(hook: Handle, code: i32, wparam: usize, lparam: isize) -> isize;
        fn SetWindowDisplayAffinity(window: Handle, affinity: u32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn GetPixel(dc: Handle, x: i32, y: i32) -> u32;
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateCompatibleBitmap(dc: Handle, width: i32, height: i32) -> Handle;
        fn DeleteDC(dc: Handle) -> i32;
        fn BitBlt(
            destination: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            source: Handle,
            source_x: i32,
            source_y: i32,
            operation: u32,
        ) -> i32;
        fn CreateSolidBrush(color: u32) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strikeout: u32,
            charset: u32,
            output_precision: u32,
            clip_precision: u32,
            quality: u32,
            pitch: u32,
            face: *const u16,
        ) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn SetBkMode(dc: Handle, mode: i32) -> i32;
        fn SetTextColor(dc: Handle, color: u32) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Handle;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmFlush() -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
    fn rgb(color: u32) -> Color {
        Color {
            red: color as u8,
            green: (color >> 8) as u8,
            blue: (color >> 16) as u8,
        }
    }

    struct Picker {
        app: tauri::AppHandle,
        tiles: [u32; (GRID * GRID) as usize],
        color: Color,
        scale: f64,
        font: Handle,
        font_scale: f64,
        valid: bool,
        sample_dc: Handle,
        sample_bitmap: Handle,
        original_bitmap: Handle,
        copy_error: bool,
    }

    impl Drop for Picker {
        fn drop(&mut self) {
            unsafe {
                if self.font != 0 {
                    DeleteObject(self.font);
                }
                if self.sample_dc != 0 {
                    if self.original_bitmap != 0 {
                        SelectObject(self.sample_dc, self.original_bitmap);
                    }
                    if self.sample_bitmap != 0 {
                        DeleteObject(self.sample_bitmap);
                    }
                    DeleteDC(self.sample_dc);
                }
            }
        }
    }

    struct ActiveGuard;
    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            WINDOW.store(0, Ordering::Release);
            ACTIVE.store(false, Ordering::Release);
        }
    }

    struct Hooks {
        mouse: Handle,
        keyboard: Handle,
    }
    impl Drop for Hooks {
        fn drop(&mut self) {
            unsafe {
                if self.mouse != 0 {
                    UnhookWindowsHookEx(self.mouse);
                }
                if self.keyboard != 0 {
                    UnhookWindowsHookEx(self.keyboard);
                }
            }
        }
    }

    unsafe extern "system" fn mouse_hook(code: i32, wparam: usize, lparam: isize) -> isize {
        let window = WINDOW.load(Ordering::Acquire);
        if code >= 0 && window != 0 {
            match wparam as u32 {
                0x0201 => {
                    // WM_LBUTTONDOWN
                    let point = (*(lparam as *const LowLevelMouse)).point;
                    let packed = (point.x as u32 as u64) | ((point.y as u32 as u64) << 32);
                    PostMessageW(window, WM_PICK, 0, packed as isize);
                    return 1;
                }
                0x0204 => {
                    PostMessageW(window, WM_CLOSE, 0, 0);
                    return 1;
                } // right down
                0x0202 | 0x0205 => return 1, // consume corresponding releases
                _ => {}
            }
        }
        CallNextHookEx(0, code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_hook(code: i32, wparam: usize, lparam: isize) -> isize {
        let window = WINDOW.load(Ordering::Acquire);
        if code >= 0 && window != 0 && (*(lparam as *const LowLevelKeyboard)).virtual_key == 0x1B {
            if wparam as u32 == 0x0100 || wparam as u32 == 0x0104 {
                PostMessageW(window, WM_CLOSE, 0, 0);
            }
            return 1;
        }
        CallNextHookEx(0, code, wparam, lparam)
    }

    fn scaled(value: i32, scale: f64) -> i32 {
        (value as f64 * scale).round() as i32
    }

    unsafe fn clear_overlay_from_sample(window: Handle, point: Point) {
        let mut rectangle = Rect::default();
        if GetWindowRect(window, &mut rectangle) != 0
            && point.x + RADIUS >= rectangle.left
            && point.x - RADIUS < rectangle.right
            && point.y + RADIUS >= rectangle.top
            && point.y - RADIUS < rectangle.bottom
        {
            // The cursor can jump into the previous floating-window position.
            // Hide it only in this case and finish composition before sampling;
            // this also works before WDA_EXCLUDEFROMCAPTURE was introduced.
            ShowWindow(window, 0);
            DwmFlush();
        }
    }

    unsafe fn refresh(window: Handle, picker: &mut Picker) {
        let mut point = Point::default();
        if GetCursorPos(&mut point) == 0 {
            return;
        }
        clear_overlay_from_sample(window, point);
        let screen = GetDC(0);
        if screen == 0 {
            return;
        }
        if picker.sample_dc == 0 {
            picker.sample_dc = CreateCompatibleDC(screen);
            if picker.sample_dc != 0 {
                picker.sample_bitmap = CreateCompatibleBitmap(screen, GRID, GRID);
                if picker.sample_bitmap != 0 {
                    picker.original_bitmap = SelectObject(picker.sample_dc, picker.sample_bitmap);
                }
            }
        }
        let copied = picker.sample_dc != 0
            && picker.sample_bitmap != 0
            && BitBlt(
                picker.sample_dc,
                0,
                0,
                GRID,
                GRID,
                screen,
                point.x - RADIUS,
                point.y - RADIUS,
                0x00CC0020,
            ) != 0;
        for y in 0..GRID {
            for x in 0..GRID {
                let color = if copied {
                    GetPixel(picker.sample_dc, x, y)
                } else {
                    u32::MAX
                };
                picker.tiles[(y * GRID + x) as usize] =
                    if color == u32::MAX { 0x00FFFFFF } else { color };
            }
        }
        let center = if copied {
            GetPixel(picker.sample_dc, RADIUS, RADIUS)
        } else {
            GetPixel(screen, point.x, point.y)
        };
        ReleaseDC(0, screen);
        picker.valid = center != u32::MAX;
        if picker.valid {
            picker.color = rgb(center);
        }

        let mut monitor = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
        };
        if GetMonitorInfoW(MonitorFromPoint(point, 2), &mut monitor) == 0 {
            return;
        }
        // Physical coordinates throughout; the thread is Per-Monitor V2.
        // Moving across a DPI boundary updates GetDpiForWindow on this tick.
        let dpi = GetDpiForWindow(window).max(96);
        picker.scale = dpi as f64 / 96.0;
        let width = scaled(220, picker.scale);
        let height = scaled(326, picker.scale);
        let gap = scaled(24, picker.scale);
        let mut x = point.x + gap;
        if x + width > monitor.work.right {
            x = point.x - width - gap;
        }
        let mut y = point.y + gap;
        if y + height > monitor.work.bottom {
            y = point.y - height - gap;
        }
        x = x
            .max(monitor.work.left)
            .min((monitor.work.right - width).max(monitor.work.left));
        y = y
            .max(monitor.work.top)
            .min((monitor.work.bottom - height).max(monitor.work.top));
        SetWindowPos(window, -1, x, y, width, height, 0x0010 | 0x0040); // TOPMOST, NOACTIVATE, SHOWWINDOW
        InvalidateRect(window, std::ptr::null(), 0);
    }

    unsafe fn fill(dc: Handle, rect: Rect, color: u32) {
        let brush = CreateSolidBrush(color);
        if brush != 0 {
            FillRect(dc, &rect, brush);
            DeleteObject(brush);
        }
    }

    unsafe fn text(
        dc: Handle,
        value: &str,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        scale: f64,
    ) {
        let characters = wide(value);
        let mut rect = Rect {
            left: scaled(left, scale),
            top: scaled(top, scale),
            right: scaled(left + width, scale),
            bottom: scaled(top + height, scale),
        };
        DrawTextW(
            dc,
            characters.as_ptr(),
            (characters.len() - 1) as i32,
            &mut rect,
            0x0020 | 0x0800,
        ); // SINGLELINE, NOPREFIX
    }

    unsafe fn paint(window: Handle, picker: &mut Picker) {
        let mut paint = PaintStruct::default();
        let dc = BeginPaint(window, &mut paint);
        if dc == 0 {
            return;
        }
        let scale = picker.scale;
        fill(
            dc,
            Rect {
                left: 0,
                top: 0,
                right: scaled(220, scale),
                bottom: scaled(326, scale),
            },
            0x00FAFAFA,
        );
        for y in 0..GRID {
            for x in 0..GRID {
                fill(
                    dc,
                    Rect {
                        left: scaled(20 + x * 20, scale),
                        top: scaled(16 + y * 20, scale),
                        right: scaled(20 + (x + 1) * 20, scale),
                        bottom: scaled(16 + (y + 1) * 20, scale),
                    },
                    picker.tiles[(y * GRID + x) as usize],
                );
            }
        }
        let black = CreateSolidBrush(0);
        let white = CreateSolidBrush(0x00FFFFFF);
        if black != 0 && white != 0 {
            let center = Rect {
                left: scaled(99, scale),
                top: scaled(95, scale),
                right: scaled(121, scale),
                bottom: scaled(117, scale),
            };
            let inside = Rect {
                left: scaled(100, scale),
                top: scaled(96, scale),
                right: scaled(120, scale),
                bottom: scaled(116, scale),
            };
            FrameRect(dc, &center, white);
            FrameRect(dc, &inside, black);
        }
        if black != 0 {
            DeleteObject(black);
        }
        if white != 0 {
            DeleteObject(white);
        }
        if picker.font == 0 || (picker.font_scale - scale).abs() > 0.01 {
            if picker.font != 0 {
                DeleteObject(picker.font);
            }
            let family = wide("Segoe UI");
            picker.font = CreateFontW(
                -scaled(15, scale),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                family.as_ptr(),
            );
            picker.font_scale = scale;
        }
        let old_font = if picker.font != 0 {
            SelectObject(dc, picker.font)
        } else {
            0
        };
        SetBkMode(dc, 1);
        SetTextColor(dc, 0x00333333);
        if picker.valid {
            text(dc, &picker.color.hex(), 20, 210, 185, 23, scale);
            text(
                dc,
                &format!(
                    "RGB  {}  {}  {}",
                    picker.color.red, picker.color.green, picker.color.blue
                ),
                20,
                239,
                185,
                23,
                scale,
            );
            let (hue, saturation, lightness) = picker.color.hsl();
            text(
                dc,
                &format!("HSL  {:.0}° {:.0}% {:.0}%", hue, saturation, lightness),
                20,
                268,
                195,
                23,
                scale,
            );
        } else {
            text(dc, "无法读取此屏幕像素", 20, 210, 185, 23, scale);
        }
        SetTextColor(dc, 0x00808080);
        text(
            dc,
            if picker.copy_error {
                "复制失败，请再次左键"
            } else {
                "左键复制 · 右键 / Esc 取消"
            },
            20,
            300,
            195,
            23,
            scale,
        );
        if old_font != 0 {
            SelectObject(dc, old_font);
        }
        EndPaint(window, &paint);
    }

    unsafe extern "system" fn window_proc(
        window: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if message == WM_NCCREATE {
            let parameter = *(lparam as *const *const c_void);
            SetWindowLongPtrW(window, GWLP_USERDATA, parameter as isize);
            return 1;
        }
        let picker = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Picker;
        match message {
            WM_TIMER if !picker.is_null() => {
                refresh(window, &mut *picker);
                0
            }
            WM_PAINT if !picker.is_null() => {
                paint(window, &mut *picker);
                0
            }
            WM_PICK if !picker.is_null() => {
                let packed = lparam as u64;
                let x = packed as u32 as i32;
                let y = (packed >> 32) as u32 as i32;
                clear_overlay_from_sample(window, Point { x, y });
                let screen = GetDC(0);
                let color = if screen != 0 {
                    GetPixel(screen, x, y)
                } else {
                    u32::MAX
                };
                if screen != 0 {
                    ReleaseDC(0, screen);
                }
                if color == u32::MAX {
                    crate::log_error(&(*picker).app, "Color picker: 无法读取当前屏幕像素");
                    (*picker).copy_error = true;
                    refresh(window, &mut *picker);
                    return 0;
                } else {
                    let hex = rgb(color).hex();
                    let result = crate::clipboard::write_text(&hex);
                    match result {
                        Ok(()) => {
                            let _ = (*picker).app.emit("color-picked", &hex);
                        }
                        Err(error) => {
                            crate::log_error(&(*picker).app, &format!("Color picker: {error}"));
                            (*picker).copy_error = true;
                            refresh(window, &mut *picker);
                            return 0;
                        }
                    }
                }
                DestroyWindow(window);
                0
            }
            WM_CLOSE => {
                DestroyWindow(window);
                0
            }
            WM_DESTROY => {
                KillTimer(window, 1);
                WINDOW.store(0, Ordering::Release);
                PostQuitMessage(0);
                0
            }
            // WM_MOUSEACTIVATE -> MA_NOACTIVATE, avoid stealing the user's focus.
            0x0021 => 3,
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    pub fn start(app: tauri::AppHandle) -> Result<(), String> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("取色模式已经开启".into());
        }
        let (ready, response) = mpsc::sync_channel(1);
        if let Err(error) = std::thread::Builder::new()
            .name("xtools-color-picker".into())
            .spawn(move || {
                let _active = ActiveGuard;
                unsafe {
                    SetThreadDpiAwarenessContext(-4);
                }
                let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
                let class_name = wide("XToolsColorPicker");
                let class = WindowClass {
                    size: std::mem::size_of::<WindowClass>() as u32,
                    style: 0x0008,
                    window_proc: Some(window_proc),
                    class_extra: 0,
                    window_extra: 0,
                    instance,
                    icon: 0,
                    cursor: 0,
                    background: 0,
                    menu_name: std::ptr::null(),
                    class_name: class_name.as_ptr(),
                    small_icon: 0,
                };
                unsafe {
                    RegisterClassExW(&class);
                }
                let mut picker = Box::new(Picker {
                    app,
                    tiles: [0x00FFFFFF; (GRID * GRID) as usize],
                    color: Color {
                        red: 0,
                        green: 0,
                        blue: 0,
                    },
                    scale: 1.0,
                    font: 0,
                    font_scale: 0.0,
                    valid: false,
                    sample_dc: 0,
                    sample_bitmap: 0,
                    original_bitmap: 0,
                    copy_error: false,
                });
                let window = unsafe {
                    CreateWindowExW(
                        0x00000008 | 0x00000080 | 0x08000000,
                        class_name.as_ptr(),
                        class_name.as_ptr(),
                        0x80000000,
                        0,
                        0,
                        220,
                        326,
                        0,
                        0,
                        instance,
                        (&mut *picker as *mut Picker).cast(),
                    )
                };
                if window == 0 {
                    let _ = ready.send(Err("无法创建取色浮窗".into()));
                    return;
                }
                WINDOW.store(window, Ordering::Release);
                let hooks = Hooks {
                    mouse: unsafe { SetWindowsHookExW(14, Some(mouse_hook), instance, 0) },
                    keyboard: unsafe { SetWindowsHookExW(13, Some(keyboard_hook), instance, 0) },
                };
                if hooks.mouse == 0 || hooks.keyboard == 0 {
                    unsafe {
                        DestroyWindow(window);
                    }
                    let _ = ready.send(Err("无法注册取色鼠标或键盘监听".into()));
                    return;
                }
                // Newer Windows excludes the floating magnifier from capture. On
                // older Windows its placement still keeps the sampled pixels clear.
                unsafe {
                    SetWindowDisplayAffinity(window, 0x00000011);
                    refresh(window, &mut picker);
                }
                if unsafe { SetTimer(window, 1, 33, std::ptr::null()) } == 0 {
                    unsafe {
                        DestroyWindow(window);
                    }
                    let _ = ready.send(Err("无法启动取色采样".into()));
                    return;
                }
                let _ = ready.send(Ok(()));
                let mut message = Message::default();
                loop {
                    let status = unsafe { GetMessageW(&mut message, 0, 0, 0) };
                    if status <= 0 {
                        break;
                    }
                    unsafe {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                unsafe {
                    KillTimer(window, 1);
                    DestroyWindow(window);
                }
                drop(hooks);
            })
        {
            ACTIVE.store(false, Ordering::Release);
            return Err(error.to_string());
        }
        response
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "取色模式启动超时")?
    }

    pub fn cancel() {
        let window = WINDOW.load(Ordering::Acquire);
        if window != 0 {
            unsafe {
                PostMessageW(window, WM_CLOSE, 0, 0);
            }
        }
    }
}

#[cfg(windows)]
pub use implementation::{cancel, start};
#[cfg(not(windows))]
pub fn start(_: tauri::AppHandle) -> Result<(), String> {
    Err("屏幕取色仅支持 Windows".into())
}
#[cfg(not(windows))]
pub fn cancel() {}
