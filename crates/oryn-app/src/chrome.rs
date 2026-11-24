use gpui_kit::{App, Window, WindowControlArea};

pub trait WindowChrome {
    fn app_drives_window(&self) -> bool;
    fn shows_captions(&self) -> bool;
    fn start_move(&self, window: &mut Window);
    fn minimize(&self, window: &mut Window);
    fn zoom(&self, window: &mut Window);
    fn close(&self, window: &mut Window, cx: &mut App);

    fn press_caption(&self, area: WindowControlArea, window: &mut Window, cx: &mut App) {
        match area {
            WindowControlArea::Min => self.minimize(window),
            WindowControlArea::Max => self.zoom(window),
            WindowControlArea::Close => self.close(window, cx),
            WindowControlArea::Drag => {}
        }
    }
}

pub struct NativeChrome {
    app_drives_window: bool,
    shows_captions: bool,
}

impl NativeChrome {
    pub fn for_platform() -> Self {
        Self {
            app_drives_window: cfg!(target_os = "linux"),
            shows_captions: !cfg!(target_os = "macos"),
        }
    }
}

impl WindowChrome for NativeChrome {
    fn app_drives_window(&self) -> bool {
        self.app_drives_window
    }

    fn shows_captions(&self) -> bool {
        self.shows_captions
    }

    fn start_move(&self, window: &mut Window) {
        window.start_window_move();
    }

    fn minimize(&self, window: &mut Window) {
        window.minimize_window();
    }

    fn zoom(&self, window: &mut Window) {
        window.zoom_window();
    }

    fn close(&self, _window: &mut Window, cx: &mut App) {
        cx.quit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_chrome_matches_the_platform() {
        let chrome = NativeChrome::for_platform();
        assert_eq!(chrome.app_drives_window(), cfg!(target_os = "linux"));
        assert_eq!(chrome.shows_captions(), !cfg!(target_os = "macos"));
    }
}
