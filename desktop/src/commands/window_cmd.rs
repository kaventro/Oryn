use tauri::Manager;

const WINDOW_ICON_PX: u32 = 256;

fn png_to_square_rgba(bytes: &[u8], size: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("icon decode failed: {e}"))?;
    let rgba = img
        .resize_exact(size, size, image::imageops::FilterType::Triangle)
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

fn window_icon_from_png(bytes: &[u8]) -> Result<tauri::image::Image<'static>, String> {
    let (rgba, width, height) = png_to_square_rgba(bytes, WINDOW_ICON_PX)?;
    Ok(tauri::image::Image::new_owned(rgba, width, height))
}

#[tauri::command]
pub fn window_close(app: tauri::AppHandle, window: tauri::Window) {
    let _ = window.close();
    if app.webview_windows().is_empty() {
        app.exit(0);
    }
}

#[tauri::command]
pub fn window_minimize(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
pub fn window_toggle_maximize(window: tauri::Window) {
    if let Ok(is_max) = window.is_maximized() {
        if is_max {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_icon {
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    static PREV_ICONS: parking_lot::Mutex<(Option<usize>, Option<usize>)> =
        parking_lot::Mutex::new((None, None));

    pub unsafe fn create_hicon_from_rgba(
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<HICON, String> {
        let pixel_count = (width * height) as usize;
        if rgba.len() != pixel_count * 4 {
            return Err("Invalid RGBA buffer length".into());
        }

        // Win32 32-bit bitmaps expect BGRA byte order (little-endian 0xAARRGGBB)
        let mut bgra = Vec::with_capacity(pixel_count * 4);
        for chunk in rgba.chunks_exact(4) {
            bgra.push(chunk[2]); // B
            bgra.push(chunk[1]); // G
            bgra.push(chunk[0]); // R
            bgra.push(chunk[3]); // A
        }

        let hbm_color = CreateBitmap(
            width as i32,
            height as i32,
            1,
            32,
            bgra.as_ptr() as *const _,
        );
        if hbm_color.is_null() {
            return Err("CreateBitmap for color failed".into());
        }

        // 1bpp monochrome mask; row length must be a multiple of 2 bytes (WORD aligned)
        let row_bytes = ((width + 15) / 16) * 2;
        let mask_data = vec![0u8; (row_bytes * height) as usize];
        let hbm_mask = CreateBitmap(
            width as i32,
            height as i32,
            1,
            1,
            mask_data.as_ptr() as *const _,
        );
        if hbm_mask.is_null() {
            DeleteObject(hbm_color as _);
            return Err("CreateBitmap for mask failed".into());
        }

        let icon_info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hicon = CreateIconIndirect(&icon_info);
        DeleteObject(hbm_color as _);
        DeleteObject(hbm_mask as _);

        if hicon.is_null() {
            Err("CreateIconIndirect failed".into())
        } else {
            Ok(hicon)
        }
    }

    pub unsafe fn set_window_icons(hwnd: HWND, hicon_big: HICON, hicon_small: HICON) {
        // Send WM_SETICON for big icon (Taskbar and Alt+Tab)
        SendMessageW(
            hwnd,
            WM_SETICON,
            ICON_BIG as usize,
            hicon_big as isize,
        );
        // Send WM_SETICON for small icon (Titlebar/Tray)
        SendMessageW(
            hwnd,
            WM_SETICON,
            ICON_SMALL as usize,
            hicon_small as isize,
        );

        // Update Class icons so Windows system caches and child windows inherit them
        #[cfg(target_pointer_width = "64")]
        {
            SetClassLongPtrW(hwnd, GCLP_HICON, hicon_big as isize);
            SetClassLongPtrW(hwnd, GCLP_HICONSM, hicon_small as isize);
        }
        #[cfg(target_pointer_width = "32")]
        {
            SetClassLongW(hwnd, GCL_HICON, hicon_big as i32);
            SetClassLongW(hwnd, GCL_HICONSM, hicon_small as i32);
        }

        // Force Windows DWM to redraw the window frame and refresh taskbar button
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }

    pub unsafe fn record_and_cleanup_old_icons(hicon_big: HICON, hicon_small: HICON) {
        let old = {
            let mut lock = PREV_ICONS.lock();
            let old = *lock;
            *lock = (Some(hicon_big as usize), Some(hicon_small as usize));
            old
        };
        if let Some(h) = old.0 {
            DestroyIcon(h as _);
        }
        if let Some(h) = old.1 {
            DestroyIcon(h as _);
        }
    }
}

#[tauri::command]
pub fn set_dock_icon(app: tauri::AppHandle, icon_id: String) -> Result<(), String> {
    let raw_id = icon_id.trim_end_matches(".png");
    let bytes: &[u8] = match raw_id {
        "1" => include_bytes!("../../dock-icons/1.png"),
        "2" => include_bytes!("../../dock-icons/2.png"),
        "3" => include_bytes!("../../dock-icons/3.png"),
        "4" => include_bytes!("../../dock-icons/4.png"),
        "5" => include_bytes!("../../dock-icons/5.png"),
        "6" => include_bytes!("../../dock-icons/6.png"),
        "7" => include_bytes!("../../dock-icons/7.png"),
        "8" => include_bytes!("../../dock-icons/8.png"),
        "9" => include_bytes!("../../dock-icons/9.png"),
        "10" => include_bytes!("../../dock-icons/10.png"),
        _ => include_bytes!("../../icons/icon.png"),
    };

    #[cfg(target_os = "macos")]
    {
        use objc2::AnyThread;
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::{MainThreadMarker, NSData};

        if let Some(mtm) = MainThreadMarker::new() {
            let app_kit_app = NSApplication::sharedApplication(mtm);
            let data = NSData::with_bytes(bytes);
            if let Some(ns_image) = NSImage::initWithData(NSImage::alloc(), &data) {
                unsafe {
                    app_kit_app.setApplicationIconImage(Some(&ns_image));
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let (rgba_big, w_big, h_big) = png_to_square_rgba(bytes, 256)?;
        let (rgba_small, w_small, h_small) = png_to_square_rgba(bytes, 32)?;
        unsafe {
            let hicon_big = win32_icon::create_hicon_from_rgba(&rgba_big, w_big, h_big)?;
            let hicon_small = win32_icon::create_hicon_from_rgba(&rgba_small, w_small, h_small)?;

            for win in app.webview_windows().values() {
                if let Ok(hwnd) = win.hwnd() {
                    let hwnd_sys = hwnd.0 as windows_sys::Win32::Foundation::HWND;
                    win32_icon::set_window_icons(hwnd_sys, hicon_big, hicon_small);
                }
            }

            win32_icon::record_and_cleanup_old_icons(hicon_big, hicon_small);
        }
    }

    let tauri_img = window_icon_from_png(bytes)?;
    let mut applied = 0usize;
    let mut last_err: Option<String> = None;
    for win in app.webview_windows().values() {
        match win.set_icon(tauri_img.clone()) {
            Ok(()) => applied += 1,
            Err(e) => last_err = Some(e.to_string()),
        }
    }

    if applied == 0 {
        return Err(last_err.unwrap_or_else(|| "no window to apply icon".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_png_is_scaled_to_window_icon_size() {
        let img = image::RgbaImage::from_pixel(1254, 1254, image::Rgba([10, 20, 30, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        let (rgba, width, height) = png_to_square_rgba(&buf, WINDOW_ICON_PX).unwrap();
        assert_eq!(width, 256);
        assert_eq!(height, 256);
        assert_eq!(rgba.len(), 256 * 256 * 4);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn creates_valid_win32_hicon_from_rgba() {
        use windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon;

        let rgba = vec![255u8; 32 * 32 * 4];
        unsafe {
            let hicon = win32_icon::create_hicon_from_rgba(&rgba, 32, 32).unwrap();
            assert!(!hicon.is_null());
            DestroyIcon(hicon);
        }
    }
}
