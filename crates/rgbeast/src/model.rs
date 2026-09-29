//! GObject wrappers around the core data so GTK widgets can bind to them.

use std::cell::RefCell;

use gtk::{glib, prelude::*, subclass::prelude::*};
use rgbeast_core::{DeviceInfo, DeviceKind, DeviceState, ModeInfo, Rgb, ZoneInfo};

use crate::i18n::gettext;

pub const ALL_ID: &str = "all";

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Device {
        pub info: RefCell<DeviceInfo>,
        pub state: RefCell<DeviceState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "RGBeastDevice";
        type Type = super::Device;
    }

    impl ObjectImpl for Device {
        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![glib::subclass::Signal::builder("changed").build()])
        }
    }
}

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>);
}

impl Device {
    pub fn new(info: DeviceInfo, state: DeviceState) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().info.replace(info);
        obj.imp().state.replace(state);
        obj
    }

    /// The "All Devices" pseudo-device: the modes every driver can express.
    pub fn all_devices() -> Self {
        let info = DeviceInfo {
            id: ALL_ID.into(),
            name: gettext("All Devices"),
            vendor: String::new(),
            kind: DeviceKind::Unknown,
            location: String::new(),
            driver: "group".into(),
            version: String::new(),
            zones: vec![],
            modes: vec![
                ModeInfo::new("static", &gettext("Static"))
                    .colors(1, 1)
                    .brightness(),
                ModeInfo::new("off", &gettext("Off")),
                ModeInfo::new("breathing", &gettext("Breathing"))
                    .colors(1, 1)
                    .brightness()
                    .speed(),
                ModeInfo::new("rainbow", &gettext("Rainbow"))
                    .speed()
                    .brightness(),
                ModeInfo::new("spectrum-cycle", &gettext("Spectrum Cycle"))
                    .speed()
                    .brightness(),
            ],
            can_save: false,
        };
        Self::new(info, DeviceState::static_color(Rgb::new(0x00, 0x80, 0xFF)))
    }

    pub fn id(&self) -> String {
        self.imp().info.borrow().id.clone()
    }

    pub fn is_group(&self) -> bool {
        self.id() == ALL_ID
    }

    pub fn info(&self) -> DeviceInfo {
        self.imp().info.borrow().clone()
    }

    pub fn state(&self) -> DeviceState {
        self.imp().state.borrow().clone()
    }

    pub fn set_info(&self, info: DeviceInfo) {
        self.imp().info.replace(info);
        self.emit_by_name::<()>("changed", &[]);
    }

    pub fn set_state(&self, state: DeviceState) {
        self.imp().state.replace(state);
        self.emit_by_name::<()>("changed", &[]);
    }

    pub fn connect_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |args| {
            let obj = args[0].get::<Self>().expect("device");
            f(&obj);
            None
        })
    }

    pub fn icon_name(&self) -> &'static str {
        if self.is_group() {
            return "view-grid-symbolic";
        }
        match self.imp().info.borrow().kind {
            DeviceKind::Motherboard => "computer-symbolic",
            DeviceKind::Dram => "drive-harddisk-solidstate-symbolic",
            DeviceKind::Gpu => "video-display-symbolic",
            DeviceKind::Fans => "weather-windy-symbolic",
            DeviceKind::Strip => "display-brightness-symbolic",
            DeviceKind::Cooler => "temperature-symbolic",
            DeviceKind::Unknown => "dialog-question-symbolic",
        }
    }

    /// Mode name and LED count, e.g. "Rainbow · 63 LEDs".
    pub fn subtitle(&self) -> String {
        let info = self.imp().info.borrow();
        let state = self.imp().state.borrow();
        if self.is_group() {
            return gettext("Set everything at once");
        }
        let mode = info
            .mode(&state.mode)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| state.mode.clone());
        let leds = info.led_count();
        format!(
            "{mode} · {}",
            gettext("{n} LEDs").replace("{n}", &leds.to_string())
        )
    }

    /// Colours to show in the sidebar strip.
    pub fn strip_colors(&self) -> Vec<Rgb> {
        let info = self.imp().info.borrow();
        let state = self.imp().state.borrow();
        preview_colors(&info, &state)
            .into_iter()
            .flat_map(|(_, c)| c)
            .collect()
    }
}

/// The colours each zone shows for a state: per-LED colours, a mode colour
/// spread over the LEDs, or a rainbow for colourless modes.
pub fn preview_colors(info: &DeviceInfo, state: &DeviceState) -> Vec<(ZoneInfo, Vec<Rgb>)> {
    let mode = info.mode(&state.mode);
    info.zones
        .iter()
        .map(|z| {
            let n = z.leds as usize;
            let colors: Vec<Rgb> = match mode.map(|m| m.color_mode) {
                Some(rgbeast_core::model::ColorMode::PerLed) => state.zone_colors(&z.id, z.leds),
                Some(rgbeast_core::model::ColorMode::ModeColors) => {
                    let cs = if state.colors.is_empty() {
                        vec![Rgb::WHITE]
                    } else {
                        state.colors.clone()
                    };
                    (0..n).map(|i| cs[i * cs.len() / n.max(1)]).collect()
                }
                _ if state.mode == "off" => vec![Rgb::BLACK; n],
                _ => (0..n)
                    .map(|i| Rgb::from_hsv(i as f64 * 360.0 / n.max(1) as f64, 1.0, 1.0))
                    .collect(),
            };
            (z.clone(), colors)
        })
        .collect()
}

/// Translate a group state to what a specific device can do.
pub fn map_group_state(info: &DeviceInfo, group: &DeviceState) -> Option<DeviceState> {
    let has = |m: &str| info.mode(m).is_some();
    let candidates: &[&str] = match group.mode.as_str() {
        "static" => &["static", "direct"],
        "off" => &["off"],
        "breathing" => &["breathing", "breath"],
        "rainbow" => &["rainbow", "spectrum-cycle", "spectrum"],
        "spectrum-cycle" => &["spectrum-cycle", "spectrum", "prism", "rainbow"],
        _ => &[],
    };
    let mut state = group.clone();
    state.zones.clear();
    if let Some(m) = candidates.iter().find(|m| has(m)) {
        state.mode = m.to_string();
        return Some(state);
    }
    if group.mode == "off" && has("static") {
        // No "off" on this device: static black at zero brightness.
        state.mode = "static".into();
        state.colors = vec![Rgb::BLACK];
        state.brightness = 0;
        return Some(state);
    }
    if has("static") && !group.colors.is_empty() {
        // No such effect on this device: at least show the group's colour.
        state.mode = "static".into();
        return Some(state);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_mapping_prefers_native_modes() {
        let mut info = DeviceInfo {
            id: "x".into(),
            name: "x".into(),
            vendor: String::new(),
            kind: DeviceKind::Dram,
            location: String::new(),
            driver: "fury".into(),
            version: String::new(),
            zones: vec![],
            modes: rgbeast_core::drivers::fury::modes(),
            can_save: false,
        };
        let g = DeviceState {
            mode: "breathing".into(),
            ..Default::default()
        };
        assert_eq!(map_group_state(&info, &g).unwrap().mode, "breath");
        let g = DeviceState::off();
        let m = map_group_state(&info, &g).unwrap();
        assert_eq!(m.mode, "static");
        assert_eq!(m.brightness, 0);
        info.modes = rgbeast_core::drivers::ene::modes();
        assert_eq!(map_group_state(&info, &g).unwrap().mode, "off");
        let g = DeviceState {
            mode: "spectrum-cycle".into(),
            ..Default::default()
        };
        assert_eq!(map_group_state(&info, &g).unwrap().mode, "spectrum-cycle");
        // A device without the effect still shows the group's colour.
        info.modes = rgbeast_core::drivers::sapphire::modes();
        let g = DeviceState {
            mode: "breathing".into(),
            ..Default::default()
        };
        assert_eq!(map_group_state(&info, &g).unwrap().mode, "static");
    }
}
