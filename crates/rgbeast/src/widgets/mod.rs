pub mod color_strip;
pub mod color_wheel;
pub mod led_preview;

pub use color_strip::ColorStrip;
pub use color_wheel::ColorWheel;
pub use led_preview::{LedPreview, ZoneShape};

use rgbeast_core::Rgb;

pub(crate) fn rgba(c: Rgb, alpha: f32) -> gtk::gdk::RGBA {
    gtk::gdk::RGBA::new(
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        alpha,
    )
}
