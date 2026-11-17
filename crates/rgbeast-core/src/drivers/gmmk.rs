//! Glorious GMMK Pro on its stock firmware (the protocol of Glorious CORE,
//! the vendor's Windows app). Lighting only: the keymap, macros, polling
//! rate and debounce belong to Wheelguard, the keyboard's settings app.
//!
//! Every command is one HID feature report, ID 7, on the vendor interface
//! (usage page 0xFF01, usage 1). The board cannot report its settings back.
//! A second vendor interface (0xFF00) sends events; `04 F1 pp ll` says the
//! active profile or layer changed. See `docs/PROTOCOLS.md`.
//!
//! The lighting lives in each profile and layer, like in Glorious CORE:
//! a change goes to the active one, then a commit makes the board show it.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{
    Driver, Error, Result, Rgb,
    model::{
        DeviceInfo, DeviceKind, DeviceState, KeyRect, KeyboardOptions, ModeInfo, SwitchInfo,
        ZoneInfo,
    },
    transport::HidTransport,
};

pub const VENDOR_ID: u16 = 0x320F;
/// ANSI, ISO, and the "Alt" ANSI / ISO boards (`/` and `\` swapped in the matrix).
/// The bootloader IDs (B007, B009, B00F, B010) are never listed.
pub const PRODUCT_IDS: &[u16] = &[0x5044, 0x5046, 0x5092, 0x5093];
pub const SETTINGS_PAGE: u16 = 0xFF01;
pub const SETTINGS_USAGE: u16 = 0x0001;
pub const EVENTS_PAGE: u16 = 0xFF00;
pub const EVENTS_USAGE: u16 = 0xFF00;

pub const REPORT_ID: u8 = 7;
/// A per-key page needs 8 header bytes and 200 data bytes; Glorious CORE
/// sends 264. Anything outside this range is not the board we know.
pub const MIN_REPORT_LEN: usize = 208;
pub const MAX_REPORT_LEN: usize = 1024;

const CMD_COMMIT: u8 = 0x01;
const CMD_EFFECT: u8 = 0x02;
const CMD_PER_KEY: u8 = 0x06;
const CMD_PROPERTIES: u8 = 0x07;

/// Pauses after each frame, as Glorious CORE waits.
const PAUSE_SHORT_MS: u64 = 100;
const PAUSE_LONG_MS: u64 = 550;

const PER_KEY_PAGES: usize = 3;
const PER_KEY_PAGE_LEN: usize = 200;
const LED_SLOTS: usize = PER_KEY_PAGES * PER_KEY_PAGE_LEN / 5;
/// The side light bars: one colour in Glorious CORE, 16 LED slots.
const SIDE_LEDS: [u8; 16] = [
    15, 16, 17, 18, 35, 36, 37, 38, 55, 56, 57, 58, 75, 76, 77, 78,
];

pub const SIDE_SWITCH: &str = "side-lights";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Ansi,
    Iso,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub product_id: u16,
    pub name: &'static str,
    pub layout: Layout,
    /// The Alt boards swap the matrix positions of `/` and `\`.
    pub alt: bool,
}

pub fn model(product_id: u16) -> Option<Model> {
    let (name, layout, alt) = match product_id {
        0x5044 => ("Glorious GMMK Pro", Layout::Ansi, false),
        0x5046 => ("Glorious GMMK Pro (ISO)", Layout::Iso, false),
        0x5092 => ("Glorious GMMK Pro", Layout::Ansi, true),
        0x5093 => ("Glorious GMMK Pro (ISO)", Layout::Iso, true),
        _ => return None,
    };
    Some(Model {
        product_id,
        name,
        layout,
        alt,
    })
}

// ------------------------------------------------------------------ keys

type Row = &'static [(&'static str, u16)];
/// Rows of keys with widths in quarter keys; "" is a gap.
#[rustfmt::skip]
const TOP: Row = &[
    ("Escape", 4), ("", 1), ("F1", 4), ("F2", 4), ("F3", 4), ("F4", 4), ("", 1), ("F5", 4),
    ("F6", 4), ("F7", 4), ("F8", 4), ("", 1), ("F9", 4), ("F10", 4), ("F11", 4), ("F12", 4),
    ("", 1), ("PrintScreen", 4), ("", 4),
];
#[rustfmt::skip]
const NUM: Row = &[
    ("Backquote", 4), ("Digit1", 4), ("Digit2", 4), ("Digit3", 4), ("Digit4", 4), ("Digit5", 4),
    ("Digit6", 4), ("Digit7", 4), ("Digit8", 4), ("Digit9", 4), ("Digit0", 4), ("Minus", 4),
    ("Equal", 4), ("Backspace", 8), ("Delete", 4),
];
#[rustfmt::skip]
const BOTTOM: Row = &[
    ("ControlLeft", 5), ("MetaLeft", 5), ("AltLeft", 5), ("Space", 25), ("AltRight", 4),
    ("Fn", 4), ("ControlRight", 4), ("ArrowLeft", 4), ("ArrowDown", 4), ("ArrowRight", 4),
];
#[rustfmt::skip]
const ANSI_ROWS: [Row; 6] = [
    TOP,
    NUM,
    &[
        ("Tab", 6), ("KeyQ", 4), ("KeyW", 4), ("KeyE", 4), ("KeyR", 4), ("KeyT", 4), ("KeyY", 4),
        ("KeyU", 4), ("KeyI", 4), ("KeyO", 4), ("KeyP", 4), ("BracketLeft", 4),
        ("BracketRight", 4), ("Backslash", 6), ("PageUp", 4),
    ],
    &[
        ("CapsLock", 7), ("KeyA", 4), ("KeyS", 4), ("KeyD", 4), ("KeyF", 4), ("KeyG", 4),
        ("KeyH", 4), ("KeyJ", 4), ("KeyK", 4), ("KeyL", 4), ("Semicolon", 4), ("Quote", 4),
        ("Enter", 9), ("PageDown", 4),
    ],
    &[
        ("ShiftLeft", 9), ("KeyZ", 4), ("KeyX", 4), ("KeyC", 4), ("KeyV", 4), ("KeyB", 4),
        ("KeyN", 4), ("KeyM", 4), ("Comma", 4), ("Period", 4), ("Slash", 4), ("ShiftRight", 7),
        ("ArrowUp", 4), ("End", 4),
    ],
    BOTTOM,
];
/// ISO: the tall Enter spans the Q and A rows ("" of width 6 on the Q row
/// keeps the gap; the key is drawn once, on the A row, two rows high).
#[rustfmt::skip]
const ISO_ROWS: [Row; 6] = [
    TOP,
    NUM,
    &[
        ("Tab", 6), ("KeyQ", 4), ("KeyW", 4), ("KeyE", 4), ("KeyR", 4), ("KeyT", 4), ("KeyY", 4),
        ("KeyU", 4), ("KeyI", 4), ("KeyO", 4), ("KeyP", 4), ("BracketLeft", 4),
        ("BracketRight", 4), ("", 6), ("PageUp", 4),
    ],
    &[
        ("CapsLock", 7), ("KeyA", 4), ("KeyS", 4), ("KeyD", 4), ("KeyF", 4), ("KeyG", 4),
        ("KeyH", 4), ("KeyJ", 4), ("KeyK", 4), ("KeyL", 4), ("Semicolon", 4), ("Quote", 4),
        ("Hashtag", 4), ("Enter", 5), ("PageDown", 4),
    ],
    &[
        ("ShiftLeft", 5), ("Backslash", 4), ("KeyZ", 4), ("KeyX", 4), ("KeyC", 4), ("KeyV", 4),
        ("KeyB", 4), ("KeyN", 4), ("KeyM", 4), ("Comma", 4), ("Period", 4), ("Slash", 4),
        ("ShiftRight", 7), ("ArrowUp", 4), ("End", 4),
    ],
    BOTTOM,
];

/// The LED slot of a key, from Glorious CORE's key index map. The knob
/// (slot 14) has no light.
fn led_slot(m: &Model, key: &str) -> Option<u8> {
    let iso = m.layout == Layout::Iso;
    let slot = match key {
        "Escape" => 0,
        "F1" => 1,
        "F2" => 2,
        "F3" => 3,
        "F4" => 4,
        "F5" => 5,
        "F6" => 6,
        "F7" => 7,
        "F8" => 8,
        "F9" => 9,
        "F10" => 10,
        "F11" => 11,
        "F12" => 12,
        "PrintScreen" => 13,
        "Backquote" => 20,
        "Digit1" => 21,
        "Digit2" => 22,
        "Digit3" => 23,
        "Digit4" => 24,
        "Digit5" => 25,
        "Digit6" => 26,
        "Digit7" => 27,
        "Digit8" => 28,
        "Digit9" => 29,
        "Digit0" => 30,
        "Minus" => 31,
        "Equal" => 32,
        "Backspace" => 33,
        "Delete" => 34,
        "Tab" => 40,
        "KeyQ" => 41,
        "KeyW" => 42,
        "KeyE" => 43,
        "KeyR" => 44,
        "KeyT" => 45,
        "KeyY" => 46,
        "KeyU" => 47,
        "KeyI" => 48,
        "KeyO" => 49,
        "KeyP" => 50,
        "BracketLeft" => 51,
        "BracketRight" => 52,
        "Backslash" if iso => 81,
        "Backslash" if m.alt => 90,
        "Backslash" => 53,
        "PageUp" => 54,
        "CapsLock" => 60,
        "KeyA" => 61,
        "KeyS" => 62,
        "KeyD" => 63,
        "KeyF" => 64,
        "KeyG" => 65,
        "KeyH" => 66,
        "KeyJ" => 67,
        "KeyK" => 68,
        "KeyL" => 69,
        "Semicolon" => 70,
        "Quote" => 71,
        "Hashtag" => 72,
        "Enter" => 73,
        "PageDown" => 74,
        "ShiftLeft" => 80,
        "KeyZ" if iso => 82,
        "KeyZ" => 81,
        "KeyX" if iso => 83,
        "KeyX" => 82,
        "KeyC" if iso => 84,
        "KeyC" => 83,
        "KeyV" if iso => 85,
        "KeyV" => 84,
        "KeyB" if iso => 86,
        "KeyB" => 85,
        "KeyN" if iso => 87,
        "KeyN" => 86,
        "KeyM" if iso => 88,
        "KeyM" => 87,
        "Comma" if iso => 89,
        "Comma" => 88,
        "Period" if iso => 90,
        "Period" => 89,
        "Slash" if iso => 91,
        "Slash" if m.alt => 53,
        "Slash" => 90,
        "ShiftRight" if iso => 92,
        "ShiftRight" => 91,
        "ArrowUp" => 93,
        "End" => 94,
        "ControlLeft" => 100,
        "MetaLeft" => 101,
        "AltLeft" => 102,
        "Space" => 105,
        "AltRight" => 108,
        "Fn" => 109,
        "ControlRight" => 110,
        "ArrowLeft" => 112,
        "ArrowDown" => 113,
        "ArrowRight" => 114,
        _ => return None,
    };
    Some(slot)
}

fn key_label(key: &str) -> String {
    let fixed = match key {
        "Escape" => "Esc",
        "PrintScreen" => "PrtSc",
        "Backquote" => "`",
        "Minus" => "-",
        "Equal" => "=",
        "Backspace" => "Backspace",
        "Delete" => "Del",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Backslash" => "\\",
        "PageUp" => "PgUp",
        "CapsLock" => "Caps",
        "Semicolon" => ";",
        "Quote" => "'",
        "Hashtag" => "#",
        "PageDown" => "PgDn",
        "ShiftLeft" | "ShiftRight" => "Shift",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "ArrowUp" => "↑",
        "ArrowDown" => "↓",
        "ArrowLeft" => "←",
        "ArrowRight" => "→",
        "ControlLeft" | "ControlRight" => "Ctrl",
        "MetaLeft" => "Super",
        "AltLeft" | "AltRight" => "Alt",
        "Space" => "",
        other => {
            return other
                .strip_prefix("Key")
                .or_else(|| other.strip_prefix("Digit"))
                .unwrap_or(other)
                .to_string();
        }
    };
    fixed.to_string()
}

/// The keys of a model in drawing order: (LED slot, rectangle).
pub fn keys(m: &Model) -> Vec<(u8, KeyRect)> {
    let rows = match m.layout {
        Layout::Ansi => &ANSI_ROWS,
        Layout::Iso => &ISO_ROWS,
    };
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let mut x = 0u16;
        for &(key, w) in row.iter() {
            if !key.is_empty() {
                let slot = led_slot(m, key).expect("every drawn key has a light");
                let tall = m.layout == Layout::Iso && key == "Enter";
                let (ry, h) = if tall { (y - 1, 8) } else { (y, 4) };
                out.push((
                    slot,
                    KeyRect {
                        label: key_label(key),
                        x,
                        y: ry as u16 * 4,
                        w,
                        h,
                    },
                ));
            }
            x += w;
        }
    }
    out
}

// ------------------------------------------------------------------ modes

/// (RGBeast id, name, effect byte, takes a colour, has speed). Names follow Glorious CORE.
const EFFECTS: &[(&str, &str, u8, bool, bool)] = &[
    ("static", "Static", 7, true, false),
    ("glorious", "Glorious Mode", 1, false, true),
    ("breathing", "Breathing", 6, true, true),
    ("rainbow", "Rainbow", 13, false, true),
    ("wave-1", "Wave 1", 2, true, true),
    ("wave-2", "Wave 2", 3, true, true),
    ("spiral", "Spiraling Wave", 4, true, true),
    ("acid", "Acid", 5, false, true),
    ("ripple", "Ripple", 8, true, true),
    ("pass-without-trace", "Pass Without Trace", 9, true, true),
    ("fast-run", "Fast Run Without Trace", 10, true, true),
    ("matrix-2", "Matrix 2", 11, true, true),
    ("matrix-3", "Matrix 3", 12, true, true),
    ("heartbeat", "Heartbeat", 14, false, true),
    ("digital-times", "Digital Times", 15, true, true),
    ("kamehameha", "Kamehameha", 16, true, true),
    ("ping-pong", "Ping Pong", 17, true, true),
    ("surmount", "Surmount", 18, true, true),
];
const EFFECT_OFF: u8 = 0;
/// Per-key colours sit on top of the single-colour effect.
const EFFECT_PER_KEY_BASE: u8 = 7;

pub fn modes() -> Vec<ModeInfo> {
    let mut out = vec![
        ModeInfo::new("direct", "Per-Key").per_led().brightness(),
        ModeInfo::new("off", "Off"),
    ];
    for &(id, name, _, color, speed) in EFFECTS {
        let mut m = ModeInfo::new(id, name).brightness();
        if color {
            // Every colour effect can also run in many colours ("multicolour" in CORE).
            m = m.colors(1, 1).random();
        }
        if speed {
            m = m.speed();
        }
        out.push(m);
    }
    out
}

fn effect_byte(mode: &str) -> Result<u8> {
    match mode {
        "off" => Ok(EFFECT_OFF),
        "direct" => Ok(EFFECT_PER_KEY_BASE),
        other => EFFECTS
            .iter()
            .find(|e| e.0 == other)
            .map(|e| e.2)
            .ok_or_else(|| Error::invalid(format!("unknown GMMK Pro effect {other}"))),
    }
}

pub fn switches() -> Vec<SwitchInfo> {
    vec![SwitchInfo {
        id: SIDE_SWITCH.into(),
        name: "Side Lights".into(),
        description: "The light bars on both sides of the keyboard.".into(),
        default: true,
    }]
}

pub fn zones(m: &Model) -> Vec<ZoneInfo> {
    let keys = keys(m);
    let mut z = ZoneInfo::fixed("keys", "Keys", keys.len() as u32);
    z.keys = keys.into_iter().map(|(_, r)| r).collect();
    vec![z, ZoneInfo::fixed("sides", "Side Lights", 1)]
}

/// Firmware version from the USB device release (bcdDevice 0x0045 is "0.45").
pub fn firmware(bcd: u16) -> String {
    format!("{:x}.{:02x}", bcd >> 8, bcd & 0xFF)
}

// ------------------------------------------------------------------ HID descriptors

fn items(desc: &[u8]) -> impl Iterator<Item = (u8, u32)> + '_ {
    let mut i = 0usize;
    std::iter::from_fn(move || {
        while i < desc.len() {
            let prefix = desc[i];
            if prefix == 0xFE {
                // Long item: size byte, tag byte, data.
                let size = *desc.get(i + 1)? as usize;
                i += 3 + size;
                continue;
            }
            let n = [0usize, 1, 2, 4][(prefix & 3) as usize];
            let mut val = 0u32;
            for (k, b) in desc.get(i + 1..i + 1 + n)?.iter().enumerate() {
                val |= (*b as u32) << (8 * k);
            }
            i += 1 + n;
            return Some((prefix & 0xFC, val));
        }
        None
    })
}

/// (usage page, usage) of every top-level collection.
pub fn application_collections(desc: &[u8]) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    let (mut page, mut usage, mut depth) = (0u32, 0u32, 0u32);
    let mut stack = Vec::new();
    for (tag, val) in items(desc) {
        match tag {
            0x04 => page = val,
            0x08 => usage = val,
            0xA4 => stack.push(page),
            0xB4 => page = stack.pop().unwrap_or(page),
            0xA0 => {
                if depth == 0 {
                    // A 4-byte usage carries its own page in the top half.
                    let (p, u) = if usage > 0xFFFF {
                        (usage >> 16, usage & 0xFFFF)
                    } else {
                        (page, usage)
                    };
                    out.push((p as u16, u as u16));
                }
                depth += 1;
                usage = 0;
            }
            0xC0 => depth = depth.saturating_sub(1),
            // Main items clear the local usage.
            0x80 | 0x90 | 0xB0 => usage = 0,
            _ => {}
        }
    }
    out
}

/// The node is one of the board's vendor interfaces (settings or events)
/// and carries nothing else: never the keyboard itself, so the daemon can
/// not read what is typed.
pub fn vendor_only(desc: &[u8]) -> bool {
    let cols = application_collections(desc);
    let ours = cols
        .iter()
        .any(|c| *c == (SETTINGS_PAGE, SETTINGS_USAGE) || *c == (EVENTS_PAGE, EVENTS_USAGE));
    ours && cols.iter().all(|(page, _)| *page >= 0xFF00)
}

/// Bytes of feature report `id`, the report ID byte included (0: none).
pub fn feature_report_len(desc: &[u8], id: u8) -> usize {
    let (mut size, mut count, mut rid) = (0u32, 0u32, 0u32);
    let mut stack = Vec::new();
    let mut bits = 0u64;
    let mut found = false;
    for (tag, val) in items(desc) {
        match tag {
            0x74 => size = val,
            0x94 => count = val,
            0x84 => rid = val,
            0xA4 => stack.push((size, count, rid)),
            0xB4 => (size, count, rid) = stack.pop().unwrap_or((size, count, rid)),
            0xB0 if rid == id as u32 => {
                bits += size as u64 * count as u64;
                found = true;
            }
            _ => {}
        }
    }
    if found {
        bits.div_ceil(8) as usize + 1
    } else {
        0
    }
}

// ------------------------------------------------------------------ frames

fn frame(cmd: u8) -> Vec<u8> {
    let mut f = vec![0u8; MIN_REPORT_LEN];
    f[0] = REPORT_ID;
    f[1] = cmd;
    f
}

/// Brightness 0..=100 on the board's 0..=20 scale.
fn level(brightness: u32) -> u8 {
    ((brightness.min(100) * 20 + 50) / 100) as u8
}

fn sides_on(state: &DeviceState) -> bool {
    state.switches.get(SIDE_SWITCH).copied().unwrap_or(true)
}

pub fn properties_frame(o: &KeyboardOptions, layer: u8, state: &DeviceState) -> Result<Vec<u8>> {
    let mut f = frame(CMD_PROPERTIES);
    f[2] = o.profile;
    f[3] = layer;
    f[4] = effect_byte(&state.mode)?;
    f[5] = 0;
    f[6] = 1;
    f[7] = if sides_on(state) { 0 } else { 1 };
    f[8] = level(state.brightness);
    f[10] = o.debounce_ms;
    Ok(f)
}

pub fn effect_frame(state: &DeviceState) -> Result<Vec<u8>> {
    let mut f = frame(CMD_EFFECT);
    f[2] = effect_byte(&state.mode)?;
    f[8] = (100 - state.speed.min(100)) as u8;
    f[9] = level(state.brightness);
    if state.random {
        f[10] = 1;
        f[12] = 1;
    }
    let c = state.primary_color();
    f[15] = c.r;
    f[16] = c.g;
    f[17] = c.b;
    Ok(f)
}

pub fn commit_frame(o: &KeyboardOptions) -> Vec<u8> {
    let mut f = frame(CMD_COMMIT);
    f[2] = o.profile;
    f[3] = o.layer;
    f
}

/// The 600 per-key bytes: 5 per LED slot (0xFF, slot, R, G, B). All zero
/// when the mode is not per-key, which hands every key back to the effect.
pub fn per_key_data(m: &Model, state: &DeviceState) -> Vec<u8> {
    let mut data = vec![0u8; LED_SLOTS * 5];
    if state.mode != "direct" {
        return data;
    }
    let mut put = |slot: u8, c: Rgb| {
        let i = slot as usize * 5;
        data[i..i + 5].copy_from_slice(&[0xFF, slot, c.r, c.g, c.b]);
    };
    let keys = keys(m);
    let colors = state.zone_colors("keys", keys.len() as u32);
    for ((slot, _), c) in keys.iter().zip(colors) {
        put(*slot, c);
    }
    let side = state.zone_colors("sides", 1)[0];
    for slot in SIDE_LEDS {
        put(slot, side);
    }
    data
}

pub fn per_key_frames(o: &KeyboardOptions, data: &[u8]) -> Vec<Vec<u8>> {
    (0..PER_KEY_PAGES)
        .map(|page| {
            let mut f = frame(CMD_PER_KEY);
            f[2] = o.profile;
            f[3] = o.layer;
            f[4] = 83;
            f[5] = page as u8;
            f[6] = 20;
            f[8..8 + PER_KEY_PAGE_LEN]
                .copy_from_slice(&data[page * PER_KEY_PAGE_LEN..(page + 1) * PER_KEY_PAGE_LEN]);
            f
        })
        .collect()
}

/// A profile/layer event from the event interface: `04 F1 pp ll`.
pub fn parse_event(report: &[u8]) -> Option<(u8, u8)> {
    match report {
        [0x04, 0xF1, p @ 1..=3, l @ 1..=3, ..] => Some((*p, *l)),
        _ => None,
    }
}

// ------------------------------------------------------------------ the driver

pub type Pause = Box<dyn FnMut(u64) + Send>;

pub struct Gmmk<T: HidTransport> {
    hid: T,
    info: DeviceInfo,
    model: Model,
    report_len: usize,
    options: Arc<Mutex<KeyboardOptions>>,
    /// Per-key bytes last sent, and to which profile and layer.
    per_key_sent: Option<(u8, u8, Vec<u8>)>,
    pause: Pause,
    events: Option<EventReader>,
}

impl<T: HidTransport> Gmmk<T> {
    /// Nothing is sent while the driver is built: the board is only written
    /// to when the owner changes its lighting.
    pub fn new(
        hid: T,
        model: Model,
        report_len: usize,
        id: String,
        location: String,
        version: String,
        options: KeyboardOptions,
    ) -> Result<Self> {
        if !(MIN_REPORT_LEN..=MAX_REPORT_LEN).contains(&report_len) {
            return Err(Error::protocol(format!(
                "unexpected settings report size ({report_len} bytes); not touching this keyboard"
            )));
        }
        options.check()?;
        let info = DeviceInfo {
            id,
            name: model.name.into(),
            vendor: "Glorious".into(),
            kind: DeviceKind::Keyboard,
            location,
            driver: "gmmk".into(),
            version,
            zones: zones(&model),
            modes: modes(),
            can_save: false,
            switches: switches(),
            untested: true,
            locked: false,
        };
        Ok(Gmmk {
            hid,
            info,
            model,
            report_len,
            options: Arc::new(Mutex::new(options)),
            per_key_sent: None,
            pause: Box::new(|ms| thread::sleep(Duration::from_millis(ms))),
            events: None,
        })
    }

    /// Replace the pauses between frames (tests).
    pub fn with_pause(mut self, pause: Pause) -> Self {
        self.pause = pause;
        self
    }

    /// Follow profile and layer changes made on the board itself.
    pub fn watch_events<E: HidTransport + 'static>(&mut self, events: E) {
        self.events = Some(EventReader::spawn(events, self.options.clone()));
    }

    fn opts(&self) -> KeyboardOptions {
        *self.options.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn send(&mut self, f: &[u8], pause_ms: u64) -> Result<()> {
        if f.len() > self.report_len {
            return Err(Error::protocol("frame does not fit the settings report"));
        }
        let mut out = f.to_vec();
        out.resize(self.report_len, 0);
        self.hid.send_feature(&out)?;
        (self.pause)(pause_ms);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn transport(&self) -> &T {
        &self.hid
    }
}

impl<T: HidTransport> Driver for Gmmk<T> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Properties (0x07), effect (0x02), the per-key pages (0x06) when they
    /// changed, then the commit (0x01), in Glorious CORE's order.
    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        let o = self.opts();
        let props = properties_frame(&o, o.layer, state)?;
        let effect = effect_frame(state)?;
        let data = per_key_data(&self.model, state);
        self.send(&props, PAUSE_SHORT_MS)?;
        self.send(&effect, PAUSE_SHORT_MS)?;
        let same = matches!(&self.per_key_sent, Some((p, l, d)) if *p == o.profile && *l == o.layer && *d == data);
        if !same {
            self.per_key_sent = None;
            for f in per_key_frames(&o, &data) {
                self.send(&f, PAUSE_SHORT_MS)?;
            }
            self.per_key_sent = Some((o.profile, o.layer, data));
        }
        self.send(&commit_frame(&o), PAUSE_LONG_MS)
    }

    fn keeps_state(&self) -> bool {
        true
    }

    fn keyboard_options(&self) -> Option<KeyboardOptions> {
        Some(self.opts())
    }

    /// A new debounce goes to all three layers of the profile, with the
    /// lighting the board shows, like Glorious CORE. A profile or layer
    /// change is only noted: Wheelguard has already switched the board, and
    /// that profile keeps its own lighting and debounce.
    fn set_keyboard_options(
        &mut self,
        new: &KeyboardOptions,
        lighting: &DeviceState,
    ) -> Result<()> {
        new.check()?;
        let old = self.opts();
        if new.profile == old.profile && new.debounce_ms != old.debounce_ms {
            for layer in 1..=3 {
                let f = properties_frame(new, layer, lighting)?;
                self.send(&f, PAUSE_LONG_MS)?;
            }
        }
        *self.options.lock().unwrap_or_else(|p| p.into_inner()) = *new;
        Ok(())
    }
}

/// Reads the event interface on its own thread and keeps the profile and
/// layer up to date. Stops when the driver goes away or the board is
/// unplugged.
struct EventReader {
    stop: Arc<AtomicBool>,
}

impl EventReader {
    fn spawn<E: HidTransport + 'static>(
        mut events: E,
        options: Arc<Mutex<KeyboardOptions>>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let spawned = thread::Builder::new()
            .name("rgbeast-gmmk-events".into())
            .spawn(move || {
                let mut buf = [0u8; 64];
                while !flag.load(Ordering::Relaxed) {
                    match events.read(&mut buf, 500) {
                        Ok(n) if n > 0 => {
                            if let Some((p, l)) = parse_event(&buf[..n]) {
                                let mut o = options.lock().unwrap_or_else(|e| e.into_inner());
                                o.profile = p;
                                o.layer = l;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            });
        if let Err(e) = spawned {
            // The core crate does not log; without the thread, profile
            // changes made on the board are simply not followed.
            eprintln!("cannot follow GMMK Pro profile changes: {e}");
        }
        EventReader { stop }
    }
}

impl Drop for EventReader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockHid;

    fn board(pid: u16) -> Gmmk<MockHid> {
        Gmmk::new(
            MockHid::default(),
            model(pid).unwrap(),
            264,
            "gmmk".into(),
            "USB".into(),
            firmware(0x0045),
            KeyboardOptions::default(),
        )
        .unwrap()
        .with_pause(Box::new(|_| {}))
    }

    fn state(mode: &str) -> DeviceState {
        let info = board(0x5044).info().clone();
        crate::model::validate(
            &info,
            &DeviceState {
                mode: mode.into(),
                colors: vec![Rgb::new(10, 20, 30)],
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn layouts_have_one_light_per_key() {
        for pid in PRODUCT_IDS {
            let m = model(*pid).unwrap();
            let k = keys(&m);
            let mut slots: Vec<u8> = k.iter().map(|(s, _)| *s).collect();
            slots.sort();
            slots.dedup();
            assert_eq!(slots.len(), k.len(), "{pid:04x}: a slot is used twice");
            assert!(!slots.contains(&14), "the knob has no light");
            assert!(slots.iter().all(|s| !SIDE_LEDS.contains(s)));
            assert!(slots.iter().all(|s| (*s as usize) < LED_SLOTS));
        }
        assert_eq!(keys(&model(0x5044).unwrap()).len(), 82);
        assert_eq!(keys(&model(0x5046).unwrap()).len(), 83);
        // Alt boards swap / and \ in the matrix.
        let alt = model(0x5092).unwrap();
        assert_eq!(led_slot(&alt, "Slash"), Some(53));
        assert_eq!(led_slot(&alt, "Backslash"), Some(90));
        // ISO Enter is drawn once, two rows high.
        let iso = keys(&model(0x5046).unwrap());
        let enter = iso.iter().find(|(s, _)| *s == 73).unwrap();
        assert_eq!((enter.1.y, enter.1.h), (8, 8));
    }

    #[test]
    fn static_colour_frames_in_core_order() {
        let mut b = board(0x5044);
        let s = state("static");
        b.apply(&s).unwrap();
        let f = &b.transport().features;
        // properties, effect, three per-key pages (cleared), commit
        assert_eq!(f.len(), 6);
        assert!(f.iter().all(|x| x.len() == 264 && x[0] == 7));
        assert_eq!(&f[0][1..11], &[0x07, 1, 1, 7, 0, 1, 0, 20, 0, 2]);
        assert_eq!(f[1][1], 0x02);
        assert_eq!(f[1][2], 7);
        assert_eq!(f[1][8], 50);
        assert_eq!(f[1][9], 20);
        assert_eq!(&f[1][15..18], &[10, 20, 30]);
        assert_eq!(f[1][10], 0);
        for (page, frame) in f[2..5].iter().enumerate() {
            assert_eq!(&frame[1..7], &[0x06, 1, 1, 83, page as u8, 20]);
            assert!(frame[8..].iter().all(|b| *b == 0));
        }
        assert_eq!(&f[5][1..4], &[0x01, 1, 1]);
        assert!(f[5][4..].iter().all(|b| *b == 0));

        // Same per-key data again: the pages are not re-sent.
        b.apply(&state("breathing")).unwrap();
        assert_eq!(b.transport().features.len(), 9);
    }

    #[test]
    fn per_key_colours_and_side_lights() {
        let mut b = board(0x5044);
        let mut s = state("direct");
        s.zones[0].colors[0] = Rgb::new(255, 0, 0); // Escape
        s.zones[1].colors[0] = Rgb::new(0, 0, 255); // side lights
        s.switches.insert(SIDE_SWITCH.into(), false);
        s.brightness = 50;
        b.apply(&s).unwrap();
        let f = &b.transport().features;
        assert_eq!(f[0][4], 7, "per-key sits on the single-colour effect");
        assert_eq!(f[0][7], 1, "side lights off");
        assert_eq!(f[0][8], 10);
        let data: Vec<u8> = f[2..5].iter().flat_map(|p| p[8..208].to_vec()).collect();
        assert_eq!(&data[0..5], &[0xFF, 0, 255, 0, 0]);
        assert_eq!(&data[15 * 5..15 * 5 + 5], &[0xFF, 15, 0, 0, 255]);
        assert_eq!(&data[78 * 5..78 * 5 + 5], &[0xFF, 78, 0, 0, 255]);
        // F1 got the primary colour, the knob slot stays empty.
        assert_eq!(&data[5..10], &[0xFF, 1, 10, 20, 30]);
        assert!(data[14 * 5..15 * 5].iter().all(|b| *b == 0));
        assert_eq!(&f[5][1..4], &[0x01, 1, 1]);
    }

    #[test]
    fn multicolour_speed_and_off() {
        let mut b = board(0x5046);
        let mut s = state("wave-1");
        s.random = true;
        s.speed = 80;
        b.apply(&s).unwrap();
        let e = &b.transport().features[1];
        assert_eq!((e[2], e[8], e[10], e[12]), (2, 20, 1, 1));
        b.apply(&state("off")).unwrap();
        let p = &b.transport().features[6];
        assert_eq!(p[4], 0);
    }

    #[test]
    fn writes_go_to_the_active_profile_and_layer() {
        let mut b = board(0x5044);
        let s = state("static");
        let o = KeyboardOptions {
            profile: 2,
            layer: 3,
            debounce_ms: 2,
        };
        // Wheelguard switched the profile: noted, nothing sent.
        b.set_keyboard_options(&o, &s).unwrap();
        assert!(b.transport().features.is_empty());
        let o = KeyboardOptions {
            debounce_ms: 8,
            ..o
        };
        b.set_keyboard_options(&o, &s).unwrap();
        let f = b.transport().features.clone();
        // Debounce changed: properties for the three layers of profile 2.
        assert_eq!(f.len(), 3);
        for (i, x) in f.iter().enumerate() {
            assert_eq!(&x[1..5], &[0x07, 2, i as u8 + 1, 7]);
            assert_eq!(x[10], 8);
        }
        b.apply(&s).unwrap();
        let f = &b.transport().features;
        assert_eq!(&f[3][1..4], &[0x07, 2, 3]);
        assert_eq!(f[3][10], 8);
        assert_eq!(&f[f.len() - 1][1..4], &[0x01, 2, 3]);
        // Only the layer changed: nothing is sent.
        let n = f.len();
        b.set_keyboard_options(&KeyboardOptions { layer: 1, ..o }, &s)
            .unwrap();
        assert_eq!(b.transport().features.len(), n);
        assert_eq!(b.keyboard_options().unwrap().layer, 1);
        assert!(
            b.set_keyboard_options(
                &KeyboardOptions {
                    debounce_ms: 5,
                    ..o
                },
                &s
            )
            .is_err()
        );
    }

    #[test]
    fn refuses_odd_reports() {
        for len in [0, 64, 2000] {
            assert!(
                Gmmk::new(
                    MockHid::default(),
                    model(0x5044).unwrap(),
                    len,
                    "g".into(),
                    "".into(),
                    "".into(),
                    KeyboardOptions::default()
                )
                .is_err()
            );
        }
        let mut b = Gmmk::new(
            MockHid::default(),
            model(0x5044).unwrap(),
            300,
            "g".into(),
            "".into(),
            "".into(),
            KeyboardOptions::default(),
        )
        .unwrap()
        .with_pause(Box::new(|_| {}));
        b.apply(&state("static")).unwrap();
        assert!(b.transport().features.iter().all(|f| f.len() == 300));
    }

    #[test]
    fn events() {
        assert_eq!(parse_event(&[4, 0xF1, 2, 3, 0]), Some((2, 3)));
        assert_eq!(parse_event(&[4, 0xF1, 0, 3]), None);
        assert_eq!(parse_event(&[4, 0xFA, 1, 0]), None);
        assert_eq!(parse_event(&[4]), None);
    }

    /// Descriptors shaped like the board's interfaces.
    #[rustfmt::skip]
    const SETTINGS_DESC: &[u8] = &[
        0x06, 0x01, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x07, 0x15, 0x00, 0x26, 0xFF, 0x00,
        0x75, 0x08, 0x96, 0x07, 0x01, 0x09, 0x02, 0xB1, 0x02, 0xC0,
    ];
    #[rustfmt::skip]
    const EVENTS_DESC: &[u8] = &[
        0x06, 0x00, 0xFF, 0x0A, 0x00, 0xFF, 0xA1, 0x01, 0x85, 0x04, 0x75, 0x08, 0x95, 0x3F,
        0x09, 0x01, 0x81, 0x02, 0xC0,
    ];
    #[rustfmt::skip]
    const KEYBOARD_DESC: &[u8] = &[
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x75, 0x01,
        0x95, 0x08, 0x81, 0x02, 0xC0,
    ];

    #[test]
    fn descriptors() {
        assert_eq!(
            application_collections(SETTINGS_DESC),
            vec![(0xFF01, 0x0001)]
        );
        assert!(vendor_only(SETTINGS_DESC));
        assert!(vendor_only(EVENTS_DESC));
        assert!(!vendor_only(KEYBOARD_DESC));
        let mixed: Vec<u8> = [KEYBOARD_DESC, SETTINGS_DESC].concat();
        assert!(
            !vendor_only(&mixed),
            "a node that also carries the keyboard"
        );
        // QMK's raw HID page is vendor-only but not ours.
        assert!(!vendor_only(&[
            0x06, 0x60, 0xFF, 0x09, 0x61, 0xA1, 0x01, 0xC0
        ]));
        assert_eq!(feature_report_len(SETTINGS_DESC, 7), 264);
        assert_eq!(feature_report_len(SETTINGS_DESC, 8), 0);
        assert_eq!(feature_report_len(EVENTS_DESC, 7), 0);
    }
}
