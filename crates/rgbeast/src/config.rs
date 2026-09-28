pub const APP_ID: &str = "io.github.djshiye.RGBeast";
pub const APP_NAME: &str = "RGBeast";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RESOURCE_PREFIX: &str = "/io/github/djshiye/RGBeast";
pub const GETTEXT_DOMAIN: &str = "rgbeast";
pub const LOCALEDIR: &str = match option_env!("RGBEAST_LOCALEDIR") {
    Some(dir) => dir,
    None => "/usr/share/locale",
};
