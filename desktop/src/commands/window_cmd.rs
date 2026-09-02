use tauri::Manager;

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
    let bytes: Option<&'static [u8]> = match raw_id {
        "1" => Some(include_bytes!("../../dock-icons/1.png")),
        "2" => Some(include_bytes!("../../dock-icons/2.png")),
        "3" => Some(include_bytes!("../../dock-icons/3.png")),
        "4" => Some(include_bytes!("../../dock-icons/4.png")),
        "5" => Some(include_bytes!("../../dock-icons/5.png")),
        "6" => Some(include_bytes!("../../dock-icons/6.png")),
        "7" => Some(include_bytes!("../../dock-icons/7.png")),
        "8" => Some(include_bytes!("../../dock-icons/8.png")),
        "9" => Some(include_bytes!("../../dock-icons/9.png")),
        "10" => Some(include_bytes!("../../dock-icons/10.png")),
        _ => Some(include_bytes!("../../icons/icon.png")),
    };

    #[cfg(target_os = "macos")]
    if let Some(icon_data) = bytes {
        use objc2::AnyThread;
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::{MainThreadMarker, NSData};

        if let Some(mtm) = MainThreadMarker::new() {
            let app_kit_app = NSApplication::sharedApplication(mtm);
            let data = NSData::with_bytes(icon_data);
            if let Some(ns_image) = NSImage::initWithData(NSImage::alloc(), &data) {
                unsafe {
                    app_kit_app.setApplicationIconImage(Some(&ns_image));
                }
            }
        }
    }

    if let Some(icon_data) = bytes {
        if let Ok(img) = image::load_from_memory(icon_data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let tauri_img = tauri::image::Image::new_owned(rgba.into_raw(), width, height);
            for win in app.webview_windows().values() {
                let _ = win.set_icon(tauri_img.clone());
            }
        }
    }

    Ok(())
}
