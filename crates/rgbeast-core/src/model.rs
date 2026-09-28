//! The device model shared by the daemon and the app. Everything here is
//! plain data that serialises to JSON, which is also what crosses D-Bus.

use serde::{Deserialize, Serialize};

use crate::{Error, Result, Rgb};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceKind {
    Motherboard,
    Dram,
    Gpu,
    Fans,
    Strip,
    Cooler,
    #[default]
    Unknown,
}

/// One controllable group of LEDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneInfo {
    pub id: String,
    pub name: String,
    /// Current LED count.
    pub leds: u32,
    /// Minimum and maximum LED count. Equal to `leds` on fixed zones.
    pub leds_min: u32,
    pub leds_max: u32,
}

impl ZoneInfo {
    pub fn fixed(id: &str, name: &str, leds: u32) -> Self {
        ZoneInfo {
            id: id.into(),
            name: name.into(),
            leds,
            leds_min: leds,
            leds_max: leds,
        }
    }
    pub fn sizable(id: &str, name: &str, leds: u32, max: u32) -> Self {
        ZoneInfo {
            id: id.into(),
            name: name.into(),
            leds,
            leds_min: 0,
            leds_max: max,
        }
    }
    pub fn is_sizable(&self) -> bool {
        self.leds_min != self.leds_max
    }
}

/// How a mode takes its colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    /// The mode has no colour of its own (rainbow, spectrum cycle).
    None,
    /// The mode takes `colors_min..=colors_max` colours from `DeviceState::colors`.
    ModeColors,
    /// Every LED has its own colour from `DeviceState::zones[..].colors`.
    PerLed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub color_mode: ColorMode,
    pub colors_min: u32,
    pub colors_max: u32,
    /// Speed is a universal 0..=100 scale; each driver maps it to hardware.
    pub has_speed: bool,
    /// Brightness 0..=100. Drivers without a brightness register scale colours.
    pub has_brightness: bool,
    /// Allowed values of `DeviceState::direction`; empty when the mode has none.
    pub directions: Vec<String>,
    /// The mode can pick random colours instead of the given ones.
    pub has_random: bool,
}

impl ModeInfo {
    pub fn new(id: &str, name: &str) -> Self {
        ModeInfo {
            id: id.into(),
            name: name.into(),
            color_mode: ColorMode::None,
            colors_min: 0,
            colors_max: 0,
            has_speed: false,
            has_brightness: false,
            directions: Vec::new(),
            has_random: false,
        }
    }
    pub fn per_led(mut self) -> Self {
        self.color_mode = ColorMode::PerLed;
        self
    }
    pub fn colors(mut self, min: u32, max: u32) -> Self {
        self.color_mode = ColorMode::ModeColors;
        self.colors_min = min;
        self.colors_max = max;
        self
    }
    pub fn speed(mut self) -> Self {
        self.has_speed = true;
        self
    }
    pub fn brightness(mut self) -> Self {
        self.has_brightness = true;
        self
    }
    pub fn directions(mut self, dirs: &[&str]) -> Self {
        self.directions = dirs.iter().map(|s| s.to_string()).collect();
        self
    }
    pub fn random(mut self) -> Self {
        self.has_random = true;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceInfo {
    /// Stable identifier, e.g. `aura-usb:0b05:19af:1-3` or `fury:i2c-1:0x61`.
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    /// Human-readable bus location.
    pub location: String,
    pub driver: String,
    pub version: String,
    pub zones: Vec<ZoneInfo>,
    pub modes: Vec<ModeInfo>,
    /// The device can store the current effect as its power-on default.
    pub can_save: bool,
}

impl DeviceInfo {
    pub fn mode(&self, id: &str) -> Option<&ModeInfo> {
        self.modes.iter().find(|m| m.id == id)
    }
    pub fn zone(&self, id: &str) -> Option<&ZoneInfo> {
        self.zones.iter().find(|z| z.id == id)
    }
    pub fn led_count(&self) -> u32 {
        self.zones.iter().map(|z| z.leds).sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneState {
    pub id: String,
    /// One colour per LED. Shorter vectors are padded with the last colour.
    pub colors: Vec<Rgb>,
}

/// A complete lighting state for one device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceState {
    pub mode: String,
    /// Mode colours (for `ColorMode::ModeColors`).
    #[serde(default)]
    pub colors: Vec<Rgb>,
    #[serde(default = "default_brightness")]
    pub brightness: u32,
    #[serde(default = "default_speed")]
    pub speed: u32,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub random: bool,
    /// Per-LED colours by zone (for `ColorMode::PerLed`).
    #[serde(default)]
    pub zones: Vec<ZoneState>,
}

fn default_brightness() -> u32 {
    100
}
fn default_speed() -> u32 {
    50
}

impl Default for DeviceState {
    fn default() -> Self {
        DeviceState {
            mode: "static".into(),
            colors: vec![Rgb::new(255, 255, 255)],
            brightness: 100,
            speed: 50,
            direction: String::new(),
            random: false,
            zones: Vec::new(),
        }
    }
}

impl DeviceState {
    pub fn static_color(color: Rgb) -> Self {
        DeviceState {
            mode: "static".into(),
            colors: vec![color],
            ..Default::default()
        }
    }

    pub fn off() -> Self {
        DeviceState {
            mode: "off".into(),
            colors: vec![],
            ..Default::default()
        }
    }

    /// The first mode colour, or the first per-LED colour, or white.
    pub fn primary_color(&self) -> Rgb {
        self.colors
            .first()
            .copied()
            .or_else(|| {
                self.zones
                    .iter()
                    .flat_map(|z| z.colors.first().copied())
                    .next()
            })
            .unwrap_or(Rgb::WHITE)
    }

    /// Per-LED colours for a zone, padded/truncated to `leds`.
    pub fn zone_colors(&self, zone_id: &str, leds: u32) -> Vec<Rgb> {
        let base: Vec<Rgb> = self
            .zones
            .iter()
            .find(|z| z.id == zone_id)
            .map(|z| z.colors.clone())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| vec![self.primary_color()]);
        let last = *base.last().expect("non-empty");
        let mut out: Vec<Rgb> = base.into_iter().take(leds as usize).collect();
        while (out.len() as u32) < leds {
            out.push(last);
        }
        out
    }
}

/// Check a state against a device's capabilities and normalise it: unknown
/// modes are rejected, colours are clamped to the allowed count, per-LED
/// colours are padded to the zone size, directions default to the first
/// allowed one.
pub fn validate(info: &DeviceInfo, state: &DeviceState) -> Result<DeviceState> {
    let mode = info.mode(&state.mode).ok_or_else(|| {
        Error::invalid(format!(
            "mode '{}' not supported by {}",
            state.mode, info.name
        ))
    })?;
    let mut out = state.clone();
    out.brightness = out.brightness.min(100);
    out.speed = out.speed.min(100);

    match mode.color_mode {
        ColorMode::None => out.colors.clear(),
        ColorMode::ModeColors => {
            if out.colors.is_empty() {
                out.colors.push(Rgb::WHITE);
            }
            out.colors.truncate(mode.colors_max.max(1) as usize);
            while (out.colors.len() as u32) < mode.colors_min {
                let last = *out.colors.last().expect("non-empty");
                out.colors.push(last);
            }
        }
        ColorMode::PerLed => {
            let primary = state.primary_color();
            let mut zones = Vec::with_capacity(info.zones.len());
            for z in &info.zones {
                let mut colors = out.zone_colors(&z.id, z.leds);
                if colors.is_empty() {
                    colors.push(primary);
                }
                zones.push(ZoneState {
                    id: z.id.clone(),
                    colors,
                });
            }
            out.zones = zones;
            if out.colors.is_empty() {
                out.colors.push(primary);
            }
            out.colors.truncate(1);
        }
    }

    if mode.directions.is_empty() {
        out.direction.clear();
    } else if !mode.directions.contains(&out.direction) {
        out.direction = mode.directions[0].clone();
    }
    if !mode.has_random {
        out.random = false;
    }
    if !mode.has_speed {
        out.speed = 50;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> DeviceInfo {
        DeviceInfo {
            id: "t".into(),
            name: "Test".into(),
            vendor: "RGBeast".into(),
            kind: DeviceKind::Strip,
            location: "sim".into(),
            driver: "sim".into(),
            version: "1".into(),
            zones: vec![
                ZoneInfo::fixed("a", "A", 3),
                ZoneInfo::sizable("b", "B", 2, 10),
            ],
            modes: vec![
                ModeInfo::new("direct", "Direct").per_led().brightness(),
                ModeInfo::new("static", "Static").colors(1, 1),
                ModeInfo::new("wave", "Wave")
                    .speed()
                    .directions(&["forward", "reverse"]),
                ModeInfo::new("multi", "Multi").colors(2, 4),
            ],
            can_save: false,
        }
    }

    #[test]
    fn rejects_unknown_mode() {
        let s = DeviceState {
            mode: "nope".into(),
            ..Default::default()
        };
        assert!(matches!(validate(&info(), &s), Err(Error::Invalid(_))));
    }

    #[test]
    fn pads_per_led_colors() {
        let s = DeviceState {
            mode: "direct".into(),
            zones: vec![ZoneState {
                id: "a".into(),
                colors: vec![Rgb::new(1, 2, 3)],
            }],
            colors: vec![],
            ..Default::default()
        };
        let v = validate(&info(), &s).unwrap();
        assert_eq!(v.zones.len(), 2);
        assert_eq!(v.zones[0].colors, vec![Rgb::new(1, 2, 3); 3]);
        assert_eq!(v.zones[1].colors.len(), 2);
        assert_eq!(v.zones[1].colors[0], Rgb::new(1, 2, 3));
        assert_eq!(v.colors, vec![Rgb::new(1, 2, 3)]);
    }

    #[test]
    fn clamps_mode_colors_and_direction() {
        let s = DeviceState {
            mode: "multi".into(),
            colors: vec![Rgb::WHITE],
            direction: "sideways".into(),
            brightness: 250,
            ..Default::default()
        };
        let v = validate(&info(), &s).unwrap();
        assert_eq!(v.colors.len(), 2);
        assert_eq!(v.direction, "");
        assert_eq!(v.brightness, 100);
        let s = DeviceState {
            mode: "wave".into(),
            direction: "sideways".into(),
            ..Default::default()
        };
        let v = validate(&info(), &s).unwrap();
        assert_eq!(v.direction, "forward");
        assert!(v.colors.is_empty());
    }

    #[test]
    fn json_roundtrip_and_strictness() {
        let s = DeviceState::static_color(Rgb::new(9, 8, 7));
        let j = serde_json::to_string(&s).unwrap();
        let back: DeviceState = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
        assert!(serde_json::from_str::<DeviceState>(r#"{"mode":"static","evil":1}"#).is_err());
        let minimal: DeviceState = serde_json::from_str(r#"{"mode":"off"}"#).unwrap();
        assert_eq!(minimal.brightness, 100);
    }
}
