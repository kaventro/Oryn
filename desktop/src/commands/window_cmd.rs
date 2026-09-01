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
