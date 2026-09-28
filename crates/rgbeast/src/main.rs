mod application;
mod client;
mod config;
mod i18n;
mod model;
mod preferences;
mod scenes;
mod settings;
mod ui;
mod widgets;
mod window;

use gtk::{gio, prelude::*};

fn main() -> gtk::glib::ExitCode {
    // Development convenience: use the schema compiled by build.rs when the
    // app is not installed. Release builds rely on the system schema dir.
    #[cfg(debug_assertions)]
    if std::env::var_os("GSETTINGS_SCHEMA_DIR").is_none() {
        // SAFETY: called before any other thread exists.
        unsafe { std::env::set_var("GSETTINGS_SCHEMA_DIR", concat!(env!("OUT_DIR"), "/schemas")) };
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .without_time()
        .init();

    i18n::init();
    gio::resources_register_include!("rgbeast.gresource").expect("failed to register resources");

    let app = application::RGBeastApplication::new();
    app.run()
}
