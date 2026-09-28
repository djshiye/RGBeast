use gtk::gio;

use crate::config;

pub fn settings() -> gio::Settings {
    thread_local! {
        static SETTINGS: gio::Settings = gio::Settings::new(config::APP_ID);
    }
    SETTINGS.with(|s| s.clone())
}

pub const WINDOW_WIDTH: &str = "window-width";
pub const WINDOW_HEIGHT: &str = "window-height";
pub const WINDOW_MAXIMIZED: &str = "window-maximized";
pub const LAST_DEVICE: &str = "last-device";
pub const ANIMATE_PREVIEW: &str = "animate-preview";
