//! sRGB colour with the few conversions the app needs.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// `#rrggbb` or `rrggbb`, case-insensitive.
    pub fn from_hex(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let v = u32::from_str_radix(s, 16).ok()?;
        Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    pub fn hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Hue in degrees 0..360, saturation and value 0..1.
    pub fn from_hsv(h: f64, s: f64, v: f64) -> Rgb {
        let h = h.rem_euclid(360.0) / 60.0;
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let i = h.floor() as i32;
        let f = h - h.floor();
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i.rem_euclid(6) {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Rgb::new(to_u8(r), to_u8(g), to_u8(b))
    }

    /// Returns (hue degrees, saturation, value).
    pub fn to_hsv(&self) -> (f64, f64, f64) {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / d).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { d / max };
        (h, s, max)
    }

    /// Scale by a 0..=100 brightness. Linear in sRGB, which is what the
    /// controllers do themselves.
    pub fn scaled(&self, brightness: u32) -> Rgb {
        let k = brightness.min(100);
        Rgb::new(
            (self.r as u32 * k / 100) as u8,
            (self.g as u32 * k / 100) as u8,
            (self.b as u32 * k / 100) as u8,
        )
    }

    pub fn is_black(&self) -> bool {
        self.r == 0 && self.g == 0 && self.b == 0
    }
}

fn to_u8(x: f64) -> u8 {
    (x * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Rgb::new(0x12, 0xAB, 0xFF);
        assert_eq!(Rgb::from_hex(&c.hex()), Some(c));
        assert_eq!(Rgb::from_hex("#00ff00"), Some(Rgb::new(0, 255, 0)));
        assert_eq!(Rgb::from_hex("zz0000"), None);
        assert_eq!(Rgb::from_hex("#fff"), None);
    }

    #[test]
    fn hsv_roundtrip() {
        for (h, s, v) in [
            (0.0, 1.0, 1.0),
            (120.0, 1.0, 1.0),
            (240.0, 0.5, 0.8),
            (300.0, 0.2, 0.3),
        ] {
            let c = Rgb::from_hsv(h, s, v);
            let (h2, s2, v2) = c.to_hsv();
            assert!((h - h2).abs() < 1.5, "hue {h} vs {h2}");
            assert!((s - s2).abs() < 0.02);
            assert!((v - v2).abs() < 0.02);
        }
        assert_eq!(Rgb::from_hsv(0.0, 1.0, 1.0), Rgb::new(255, 0, 0));
        assert_eq!(Rgb::from_hsv(120.0, 1.0, 1.0), Rgb::new(0, 255, 0));
        assert_eq!(Rgb::from_hsv(240.0, 1.0, 1.0), Rgb::new(0, 0, 255));
    }

    #[test]
    fn scaled_brightness() {
        assert_eq!(Rgb::new(200, 100, 50).scaled(50), Rgb::new(100, 50, 25));
        assert_eq!(Rgb::new(200, 100, 50).scaled(100), Rgb::new(200, 100, 50));
        assert_eq!(Rgb::new(200, 100, 50).scaled(0), Rgb::BLACK);
        assert_eq!(Rgb::new(200, 100, 50).scaled(150), Rgb::new(200, 100, 50));
    }
}
