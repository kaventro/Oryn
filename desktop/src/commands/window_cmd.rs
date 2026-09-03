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
        img.write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        )
        .unwrap();
        let (rgba, width, height) = png_to_square_rgba(&buf, WINDOW_ICON_PX).unwrap();
        assert_eq!(width, 256);
        assert_eq!(height, 256);
        assert_eq!(rgba.len(), 256 * 256 * 4);
    }
}
