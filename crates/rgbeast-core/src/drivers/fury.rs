//! Kingston Fury Beast / Renegade DDR5 (and DDR4) RGB over the chipset SMBus.
//! One controller per stick at `base + slot`; all sticks are driven together
//! as one device with a zone per stick.

use std::{collections::HashMap, thread, time::Duration};

use crate::{
    Driver, Error, Result, Rgb,
    model::{DeviceInfo, DeviceKind, DeviceState, ModeInfo, ZoneInfo},
    transport::Smbus,
};

use super::map_speed;

pub const BASE_ADDR_DDR5: u8 = 0x60;
pub const BASE_ADDR_DDR4: u8 = 0x58;
pub const LEDS_DDR5: u32 = 12;
pub const LEDS_DDR4: u32 = 10;
const DELAY: Duration = Duration::from_millis(10);
const MAX_MODE_COLORS: usize = 10;

const REG_MODEL: u8 = 0x06;
const REG_APPLY: u8 = 0x08;
const REG_MODE: u8 = 0x09;
const REG_INDEX: u8 = 0x0B;
const REG_DIRECTION: u8 = 0x0C;
const REG_DELAY: u8 = 0x0D;
const REG_SPEED: u8 = 0x0E;
const REG_DYNAMIC_HOLD_A: u8 = 0x12;
const REG_DYNAMIC_HOLD_B: u8 = 0x13;
const REG_DYNAMIC_FADE_A: u8 = 0x14;
const REG_DYNAMIC_FADE_B: u8 = 0x15;
const REG_BREATH_MIN_TO_MID: u8 = 0x16;
const REG_BREATH_MID_TO_MAX: u8 = 0x17;
const REG_BREATH_MAX_TO_MID: u8 = 0x18;
const REG_BREATH_MID_TO_MIN: u8 = 0x19;
const REG_BREATH_MIN_HOLD: u8 = 0x1A;
const REG_BREATH_MAX_BRIGHTNESS: u8 = 0x1B;
const REG_BREATH_MID_BRIGHTNESS: u8 = 0x1C;
const REG_BREATH_MIN_BRIGHTNESS: u8 = 0x1D;
const REG_BRIGHTNESS: u8 = 0x20;
const REG_BG_RED: u8 = 0x23;
const REG_LENGTH: u8 = 0x26;
const REG_NUM_SLOTS: u8 = 0x27;
const REG_NUM_COLORS: u8 = 0x30;
const REG_MODE_COLORS: u8 = 0x31;
const REG_LED_COLORS: u8 = 0x50;

const BEGIN: u8 = 0x53;
const END: u8 = 0x44;
const DIR_UP: u8 = 0x01;
const DIR_DOWN: u8 = 0x02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    BeastDdr5,
    RenegadeDdr5,
    BeastWhiteDdr5,
    Beast2Ddr5,
    BeastWhiteDdr4,
    BeastDdr4,
}

impl Model {
    fn from_code(code: u8) -> Option<Model> {
        Some(match code {
            0x10 => Model::BeastDdr5,
            0x11 => Model::RenegadeDdr5,
            0x12 => Model::BeastWhiteDdr5,
            0x15 => Model::Beast2Ddr5,
            0x21 => Model::BeastWhiteDdr4,
            0x23 => Model::BeastDdr4,
            _ => return None,
        })
    }
    pub fn name(&self) -> &'static str {
        match self {
            Model::BeastDdr5 => "Kingston Fury Beast DDR5 RGB",
            Model::RenegadeDdr5 => "Kingston Fury Renegade DDR5 RGB",
            Model::BeastWhiteDdr5 => "Kingston Fury Beast DDR5 RGB (White)",
            Model::Beast2Ddr5 => "Kingston Fury Beast DDR5 RGB",
            Model::BeastWhiteDdr4 => "Kingston Fury Beast DDR4 RGB (White)",
            Model::BeastDdr4 => "Kingston Fury Beast DDR4 RGB",
        }
    }
}

/// Mode table: id, name, mode byte, colour handling, speed range
/// (slowest, fastest), has direction, background colour as last mode colour.
struct ModeDef {
    id: &'static str,
    name: &'static str,
    byte: u8,
    colors: (u32, u32),
    per_led: bool,
    speed: Option<(i32, i32)>,
    direction: bool,
    background: bool,
}

const MODES: &[ModeDef] = &[
    ModeDef {
        id: "direct",
        name: "Direct",
        byte: 0x10,
        colors: (0, 0),
        per_led: true,
        speed: None,
        direction: false,
        background: false,
    },
    ModeDef {
        id: "static",
        name: "Static",
        byte: 0x00,
        colors: (1, 1),
        per_led: false,
        speed: None,
        direction: false,
        background: false,
    },
    ModeDef {
        id: "rainbow",
        name: "Rainbow",
        byte: 0x01,
        colors: (0, 0),
        per_led: false,
        speed: Some((60, 0)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "spectrum",
        name: "Spectrum",
        byte: 0x01,
        colors: (0, 0),
        per_led: false,
        speed: Some((60, 0)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "rhythm",
        name: "Rhythm",
        byte: 0x02,
        colors: (2, 11),
        per_led: false,
        speed: Some((10, 0)),
        direction: false,
        background: true,
    },
    ModeDef {
        id: "breath",
        name: "Breath",
        byte: 0x03,
        colors: (1, 10),
        per_led: false,
        speed: Some((10, 1)),
        direction: false,
        background: false,
    },
    ModeDef {
        id: "dynamic",
        name: "Dynamic",
        byte: 0x04,
        colors: (1, 10),
        per_led: false,
        speed: Some((1000, 100)),
        direction: false,
        background: false,
    },
    ModeDef {
        id: "slide",
        name: "Slide",
        byte: 0x05,
        colors: (2, 11),
        per_led: false,
        speed: Some((255, 0)),
        direction: true,
        background: true,
    },
    ModeDef {
        id: "slither",
        name: "Slither",
        byte: 0x05,
        colors: (2, 11),
        per_led: false,
        speed: Some((255, 0)),
        direction: false,
        background: true,
    },
    ModeDef {
        id: "teleport",
        name: "Teleport",
        byte: 0x05,
        colors: (2, 11),
        per_led: false,
        speed: Some((255, 0)),
        direction: false,
        background: true,
    },
    ModeDef {
        id: "wind",
        name: "Wind",
        byte: 0x05,
        colors: (2, 11),
        per_led: false,
        speed: Some((255, 0)),
        direction: true,
        background: true,
    },
    ModeDef {
        id: "comet",
        name: "Comet",
        byte: 0x06,
        colors: (1, 10),
        per_led: false,
        speed: Some((255, 0)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "rain",
        name: "Rain",
        byte: 0x06,
        colors: (1, 10),
        per_led: false,
        speed: Some((28, 8)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "firework",
        name: "Firework",
        byte: 0x06,
        colors: (1, 10),
        per_led: false,
        speed: Some((83, 33)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "voltage",
        name: "Voltage",
        byte: 0x07,
        colors: (2, 11),
        per_led: false,
        speed: Some((18, 5)),
        direction: true,
        background: true,
    },
    ModeDef {
        id: "flame",
        name: "Flame",
        byte: 0x09,
        colors: (0, 0),
        per_led: false,
        speed: Some((64, 40)),
        direction: true,
        background: false,
    },
    ModeDef {
        id: "twilight",
        name: "Twilight",
        byte: 0x0A,
        colors: (0, 0),
        per_led: false,
        speed: Some((255, 0)),
        direction: false,
        background: false,
    },
    ModeDef {
        id: "fury",
        name: "Fury",
        byte: 0x0B,
        colors: (2, 11),
        per_led: false,
        speed: Some((255, 0)),
        direction: true,
        background: true,
    },
    ModeDef {
        id: "prism",
        name: "Prism",
        byte: 0x11,
        colors: (0, 0),
        per_led: false,
        speed: Some((60, 0)),
        direction: false,
        background: false,
    },
];

fn mode_list() -> Vec<ModeInfo> {
    MODES
        .iter()
        .map(|m| {
            let mut info = ModeInfo::new(m.id, m.name).brightness();
            if m.per_led {
                info = info.per_led();
            } else if m.colors.1 > 0 {
                info = info.colors(m.colors.0, m.colors.1);
            }
            if m.speed.is_some() {
                info = info.speed();
            }
            if m.direction {
                info = info.directions(&["up", "down"]);
            }
            info
        })
        .collect()
}

fn mode_def(id: &str) -> Result<&'static ModeDef> {
    MODES
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| Error::invalid(format!("unknown Fury mode {id}")))
}

/// Signature and model check at one address. Returns the model when a
/// Fury controller answers. Leaves the transaction closed.
///
/// Real sticks are lenient about the signature: on the target machine the
/// third register ("R") reads 0x02 on one stick and intermittently on the
/// other, and single reads sometimes return 0xFFFF. So a stick counts when
/// the first byte is "F", at least three of the four bytes match, and the
/// model register holds a known code; every read is retried with pacing.
pub fn probe<B: Smbus>(bus: &mut B, addr: u8) -> Option<Model> {
    if bus.write_byte_data(addr, REG_APPLY, BEGIN).is_err() {
        return None;
    }
    thread::sleep(DELAY);
    let mut matches = 0;
    let mut first = false;
    for (i, expected) in b"FURY".iter().enumerate() {
        let mut got = None;
        for _ in 0..5 {
            if let Ok(w) = bus.read_word_data(addr, (i + 1) as u8)
                && w != 0xFFFF
            {
                got = Some((w >> 8) as u8);
                break;
            }
            thread::sleep(DELAY * 2);
        }
        if i == 0 {
            first = got == Some(*expected);
            if !first {
                break;
            }
        }
        if got == Some(*expected) {
            matches += 1;
        }
        thread::sleep(DELAY);
    }
    let ok = first && matches >= 3;
    let model = if ok {
        bus.read_word_data(addr, REG_MODEL)
            .ok()
            .and_then(|w| Model::from_code((w >> 8) as u8))
    } else {
        None
    };
    thread::sleep(DELAY);
    bus.write_byte_data(addr, REG_APPLY, END).ok();
    thread::sleep(DELAY);
    model
}

pub struct Fury<B: Smbus> {
    bus: B,
    base: u8,
    slots: Vec<u8>,
    leds_per_dimm: u32,
    info: DeviceInfo,
    cache: Vec<HashMap<u8, u8>>,
    current_mode: Option<u8>,
}

impl<B: Smbus> Fury<B> {
    pub fn new(
        bus: B,
        base: u8,
        slots: Vec<u8>,
        model: Model,
        id: String,
        location: String,
    ) -> Result<Self> {
        if slots.is_empty() {
            return Err(Error::protocol("no Fury sticks"));
        }
        let leds_per_dimm = if base == BASE_ADDR_DDR4 {
            LEDS_DDR4
        } else {
            LEDS_DDR5
        };
        let zones = slots
            .iter()
            .map(|s| {
                ZoneInfo::fixed(
                    &format!("slot{}", s + 1),
                    &format!("Slot {}", s + 1),
                    leds_per_dimm,
                )
            })
            .collect();
        let info = DeviceInfo {
            id,
            name: format!("{} ×{}", model.name(), slots.len()),
            vendor: "Kingston".into(),
            kind: DeviceKind::Dram,
            location,
            driver: "fury".into(),
            version: format!("{model:?}"),
            zones,
            modes: mode_list(),
            can_save: false,
        };
        let cache = vec![HashMap::new(); slots.len()];
        Ok(Fury {
            bus,
            base,
            slots,
            leds_per_dimm,
            info,
            cache,
            current_mode: None,
        })
    }

    fn write_raw(&mut self, slot_idx: usize, reg: u8, val: u8) -> Result<()> {
        let addr = self.base + self.slots[slot_idx];
        let mut last = None;
        for attempt in 1..=5u32 {
            match self.bus.write_byte_data(addr, reg, val) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last = Some(e);
                    thread::sleep(DELAY * 3 * attempt);
                }
            }
        }
        Err(last.expect("at least one attempt"))
    }

    fn read_raw(&mut self, slot_idx: usize, reg: u8) -> Result<u8> {
        let addr = self.base + self.slots[slot_idx];
        let mut last = None;
        for attempt in 1..=5u32 {
            match self.bus.read_word_data(addr, reg) {
                Ok(w) => return Ok((w >> 8) as u8),
                Err(e) => {
                    last = Some(e);
                    thread::sleep(DELAY * 3 * attempt);
                }
            }
        }
        Err(last.expect("at least one attempt"))
    }

    /// Write a register on every stick, skipping values already set.
    fn set_all(&mut self, reg: u8, val: u8) -> Result<()> {
        let vals = vec![val; self.slots.len()];
        self.set_each(reg, &vals)
    }

    fn set_each(&mut self, reg: u8, vals: &[u8]) -> Result<()> {
        let mut wrote = false;
        for i in 0..self.slots.len() {
            let v = vals[i.min(vals.len() - 1)];
            if self.cache[i].get(&reg) != Some(&v) {
                self.write_raw(i, reg, v)?;
                self.cache[i].insert(reg, v);
                wrote = true;
            }
        }
        if wrote {
            thread::sleep(DELAY);
        }
        Ok(())
    }

    fn begin(&mut self) -> Result<()> {
        for i in 0..self.slots.len() {
            self.write_raw(i, REG_APPLY, BEGIN)?;
        }
        thread::sleep(DELAY);
        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        for i in 0..self.slots.len() {
            self.write_raw(i, REG_APPLY, END)?;
        }
        thread::sleep(DELAY);
        Ok(())
    }

    /// Sent when the mode changes: begin, index register on each stick, end.
    ///
    /// The index is always 0. Giving each stick its slot position here (the
    /// documented "synchronise" value) makes a Beast DDR5 stick with a
    /// non-zero index accept every later transaction into its registers and
    /// never render it; verified on real hardware, where writing 0 back
    /// unfroze the stick at once.
    fn preamble(&mut self, _synchronise: bool) -> Result<()> {
        self.begin()?;
        for i in 0..self.slots.len() {
            self.write_raw(i, REG_INDEX, 0)?;
        }
        thread::sleep(DELAY);
        self.end()
    }

    fn set_mode_colors(&mut self, colors: &[Rgb]) -> Result<()> {
        let n = colors.len().min(MAX_MODE_COLORS);
        if n == 0 {
            return Ok(());
        }
        self.set_all(REG_NUM_COLORS, n as u8)?;
        for (i, c) in colors.iter().take(n).enumerate() {
            let base = REG_MODE_COLORS + (i as u8) * 3;
            self.set_all(base, c.r)?;
            self.set_all(base + 1, c.g)?;
            self.set_all(base + 2, c.b)?;
        }
        Ok(())
    }

    fn set_led_colors(&mut self, state: &DeviceState) -> Result<()> {
        let per_slot: Vec<Vec<Rgb>> = self
            .info
            .zones
            .iter()
            .map(|z| state.zone_colors(&z.id, self.leds_per_dimm))
            .collect();
        for led in 0..self.leds_per_dimm as usize {
            let base = REG_LED_COLORS + (led as u8) * 3;
            let reds: Vec<u8> = per_slot.iter().map(|c| c[led].r).collect();
            let greens: Vec<u8> = per_slot.iter().map(|c| c[led].g).collect();
            let blues: Vec<u8> = per_slot.iter().map(|c| c[led].b).collect();
            self.set_each(base, &reds)?;
            self.set_each(base + 2, &blues)?;
            self.set_each(base + 1, &greens)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn bus(&self) -> &B {
        &self.bus
    }
}

impl<B: Smbus> Driver for Fury<B> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        let def = mode_def(&state.mode)?;
        let mode_byte = def.byte;
        if self.current_mode.is_none() {
            // First contact: learn the active mode so the preamble is only
            // sent on a real change.
            self.begin()?;
            self.current_mode = self.read_raw(0, REG_MODE).ok();
            self.end()?;
        }
        if self.current_mode != Some(mode_byte) {
            let sync = !matches!(def.id, "rain" | "firework" | "direct");
            self.preamble(sync)?;
        }

        self.begin()?;
        self.set_all(REG_MODE, mode_byte)?;
        self.current_mode = Some(mode_byte);

        // Fixed per-mode parameters and the tunables Kingston's software exposes.
        match def.id {
            "static" => {
                self.set_all(REG_DIRECTION, DIR_UP)?;
                self.set_all(REG_DELAY, 0)?;
                self.set_all(REG_SPEED, 0)?;
            }
            "rainbow" | "voltage" | "flame" | "twilight" | "fury" => self.set_all(REG_DELAY, 0)?,
            "spectrum" => self.set_all(REG_DELAY, 4)?,
            "rhythm" => {
                self.set_all(REG_DIRECTION, DIR_UP)?;
                self.set_all(REG_DELAY, 3)?;
            }
            "breath" | "dynamic" => {
                self.set_all(REG_DIRECTION, DIR_UP)?;
                self.set_all(REG_DELAY, 0)?;
            }
            "slide" => {
                self.set_all(REG_DELAY, 3)?;
                self.set_all(REG_LENGTH, 4)?;
            }
            "slither" => {
                self.set_all(REG_DELAY, 12)?;
                let dirs: Vec<u8> = (0..self.slots.len())
                    .map(|i| if i % 2 == 0 { DIR_UP } else { DIR_DOWN })
                    .collect();
                self.set_each(REG_DIRECTION, &dirs)?;
                self.set_all(REG_LENGTH, 12)?;
            }
            "teleport" => {
                self.set_all(REG_DELAY, 0)?;
                let dirs: Vec<u8> = (0..self.slots.len())
                    .map(|i| if i % 2 == 0 { DIR_UP } else { DIR_DOWN })
                    .collect();
                self.set_each(REG_DIRECTION, &dirs)?;
                self.set_all(REG_LENGTH, 3)?;
            }
            "wind" => {
                self.set_all(REG_DELAY, 0)?;
                self.set_all(REG_LENGTH, 12)?;
            }
            "comet" => {
                self.set_all(REG_DELAY, 0)?;
                self.set_all(REG_LENGTH, 7)?;
            }
            "rain" => {
                self.set_all(REG_DELAY, 0)?;
                self.set_all(REG_LENGTH, 3)?;
            }
            "firework" => {
                self.set_all(REG_DELAY, 0)?;
                self.set_all(REG_LENGTH, 7)?;
            }
            "prism" => self.set_all(REG_DELAY, 2)?,
            _ => {}
        }

        // Colours.
        if def.per_led {
            self.set_led_colors(state)?;
        } else if def.colors.1 > 0 {
            let colors = &state.colors;
            if def.background && colors.len() >= 2 {
                let (fg, bg) = colors.split_at(colors.len() - 1);
                self.set_mode_colors(fg)?;
                self.set_all(REG_BG_RED, bg[0].r)?;
                self.set_all(REG_BG_RED + 1, bg[0].g)?;
                self.set_all(REG_BG_RED + 2, bg[0].b)?;
            } else {
                self.set_mode_colors(colors)?;
            }
        }

        if def.direction {
            let d = if state.direction == "down" {
                DIR_DOWN
            } else {
                DIR_UP
            };
            self.set_all(REG_DIRECTION, d)?;
        }

        if let Some((slowest, fastest)) = def.speed {
            let raw = map_speed(state.speed, slowest.min(255), fastest.min(255));
            match def.id {
                "dynamic" => {
                    // Hold and fade times in units the controller expects.
                    let ms = slowest + (fastest - slowest) * state.speed.min(100) as i32 / 100;
                    self.set_all(REG_SPEED, 0)?;
                    self.set_all(REG_DYNAMIC_HOLD_A, (ms >> 8) as u8)?;
                    self.set_all(REG_DYNAMIC_HOLD_B, 1)?;
                    self.set_all(REG_DYNAMIC_FADE_A, ((ms * 5) >> 8) as u8)?;
                    self.set_all(REG_DYNAMIC_FADE_B, 1)?;
                }
                "breath" => {
                    self.set_all(REG_SPEED, 0)?;
                    self.set_all(REG_BREATH_MIN_TO_MID, raw.saturating_mul(3))?;
                    self.set_all(REG_BREATH_MID_TO_MAX, raw)?;
                    self.set_all(REG_BREATH_MAX_TO_MID, raw)?;
                    self.set_all(REG_BREATH_MID_TO_MIN, raw.saturating_mul(3))?;
                    self.set_all(REG_BREATH_MIN_HOLD, 1)?;
                    self.set_all(REG_BREATH_MAX_BRIGHTNESS, 100)?;
                    self.set_all(REG_BREATH_MID_BRIGHTNESS, 64)?;
                    self.set_all(REG_BREATH_MIN_BRIGHTNESS, 0)?;
                }
                "rain" | "firework" => {
                    let offsets: [u8; 4] = if def.id == "rain" {
                        [11, 0, 15, 9]
                    } else {
                        [15, 0, 19, 4]
                    };
                    let speeds: Vec<u8> = (0..self.slots.len())
                        .map(|i| raw.saturating_add(offsets[i % 4]))
                        .collect();
                    self.set_each(REG_SPEED, &speeds)?;
                }
                _ => self.set_all(REG_SPEED, raw)?,
            }
        }

        self.set_all(REG_BRIGHTNESS, state.brightness.min(100) as u8)?;
        self.set_all(REG_NUM_SLOTS, (self.slots.len().min(4)) as u8)?;
        self.end()
    }
}

pub fn modes() -> Vec<ModeInfo> {
    mode_list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::{MockSmbus, Op};

    fn bus_with_sticks(addrs: &[u8], model: u8) -> MockSmbus {
        let mut bus = MockSmbus::with_addresses(addrs);
        for a in addrs {
            for (i, b) in b"FURY".iter().enumerate() {
                bus.regs.insert((*a, (i + 1) as u8), *b);
            }
            bus.regs.insert((*a, REG_MODEL), model);
        }
        bus
    }

    #[test]
    fn probe_finds_beast_and_ignores_empty_slots() {
        let mut bus = bus_with_sticks(&[0x61, 0x63], 0x10);
        assert_eq!(probe(&mut bus, 0x61), Some(Model::BeastDdr5));
        assert_eq!(probe(&mut bus, 0x63), Some(Model::BeastDdr5));
        assert_eq!(probe(&mut bus, 0x60), None);
        // Transaction was opened and closed on the probed stick.
        assert!(bus.ops.contains(&Op::WriteByteData(0x61, REG_APPLY, BEGIN)));
        assert!(bus.ops.contains(&Op::WriteByteData(0x61, REG_APPLY, END)));
        // One odd signature byte is tolerated (seen on real Beast DDR5
        // sticks: "R" reads 0x02); two are not, nor a wrong first byte.
        let mut odd = bus_with_sticks(&[0x62], 0x15);
        odd.regs.insert((0x62, 3), 0x02);
        assert_eq!(probe(&mut odd, 0x62), Some(Model::Beast2Ddr5));
        let mut wrong = bus_with_sticks(&[0x62], 0x10);
        wrong.regs.insert((0x62, 2), b'X');
        wrong.regs.insert((0x62, 3), b'X');
        assert_eq!(probe(&mut wrong, 0x62), None);
        let mut not_f = bus_with_sticks(&[0x62], 0x10);
        not_f.regs.insert((0x62, 1), b'X');
        assert_eq!(probe(&mut not_f, 0x62), None);
        let mut unknown = bus_with_sticks(&[0x62], 0x77);
        assert_eq!(probe(&mut unknown, 0x62), None);
    }

    fn device() -> Fury<MockSmbus> {
        let bus = bus_with_sticks(&[0x61, 0x63], 0x11);
        Fury::new(
            bus,
            BASE_ADDR_DDR5,
            vec![1, 3],
            Model::RenegadeDdr5,
            "fury:test".into(),
            "i2c-1".into(),
        )
        .unwrap()
    }

    #[test]
    fn zones_and_modes() {
        let d = device();
        assert_eq!(d.info().zones.len(), 2);
        assert_eq!(d.info().zones[0].id, "slot2");
        assert_eq!(d.info().zones[0].name, "Slot 2");
        assert_eq!(d.info().zones[0].leds, 12);
        assert!(d.info().mode("firework").unwrap().has_speed);
        assert_eq!(d.info().mode("rhythm").unwrap().colors_max, 11);
        assert_eq!(
            d.info().mode("rainbow").unwrap().directions,
            vec!["up", "down"]
        );
    }

    #[test]
    fn static_transaction_sequence() {
        let mut d = device();
        let mut st = DeviceState::static_color(Rgb::new(255, 0, 128));
        st.brightness = 80;
        d.apply(&st).unwrap();
        let ops: Vec<&Op> = d.bus().writes();
        // Begin on both sticks first (initial mode read), then the preamble
        // (mode differs from the register's 0 default? static is 0x00, so no
        // preamble), then the real transaction.
        assert_eq!(*ops[0], Op::WriteByteData(0x61, REG_APPLY, BEGIN));
        assert_eq!(*ops[1], Op::WriteByteData(0x63, REG_APPLY, BEGIN));
        assert!(
            ops.iter()
                .all(|o| !matches!(o, Op::WriteByteData(_, REG_INDEX, _))),
            "no preamble for unchanged mode"
        );
        let regs = &d.bus().regs;
        assert_eq!(regs[&(0x61, REG_MODE)], 0x00);
        assert_eq!(regs[&(0x63, REG_MODE)], 0x00);
        assert_eq!(regs[&(0x61, REG_NUM_COLORS)], 1);
        assert_eq!(regs[&(0x61, REG_MODE_COLORS)], 255);
        assert_eq!(regs[&(0x61, REG_MODE_COLORS + 1)], 0);
        assert_eq!(regs[&(0x63, REG_MODE_COLORS + 2)], 128);
        assert_eq!(regs[&(0x61, REG_BRIGHTNESS)], 80);
        assert_eq!(regs[&(0x61, REG_NUM_SLOTS)], 2);
        assert_eq!(regs[&(0x63, REG_APPLY)], END);
        assert_eq!(
            **ops.last().unwrap(),
            Op::WriteByteData(0x63, REG_APPLY, END)
        );
    }

    #[test]
    fn mode_change_sends_preamble_with_slot_indices() {
        let mut d = device();
        d.apply(&DeviceState::static_color(Rgb::WHITE)).unwrap();
        d.bus.ops.clear();
        let st = DeviceState {
            mode: "rainbow".into(),
            speed: 100,
            direction: "down".into(),
            ..Default::default()
        };
        d.apply(&st).unwrap();
        let ops = d.bus().writes();
        assert_eq!(*ops[0], Op::WriteByteData(0x61, REG_APPLY, BEGIN));
        // Index 0 on every stick: a non-zero index freezes real sticks.
        assert_eq!(*ops[2], Op::WriteByteData(0x61, REG_INDEX, 0));
        assert_eq!(*ops[3], Op::WriteByteData(0x63, REG_INDEX, 0));
        assert_eq!(*ops[4], Op::WriteByteData(0x61, REG_APPLY, END));
        let regs = &d.bus().regs;
        assert_eq!(regs[&(0x61, REG_MODE)], 0x01);
        assert_eq!(regs[&(0x61, REG_SPEED)], 0); // fastest
        assert_eq!(regs[&(0x61, REG_DIRECTION)], DIR_DOWN);
        assert_eq!(regs[&(0x61, REG_DELAY)], 0);
    }

    #[test]
    fn per_led_direct_and_background_colors() {
        let mut d = device();
        let st = DeviceState {
            mode: "direct".into(),
            zones: vec![
                crate::model::ZoneState {
                    id: "slot2".into(),
                    colors: vec![Rgb::new(1, 2, 3)],
                },
                crate::model::ZoneState {
                    id: "slot4".into(),
                    colors: vec![Rgb::new(4, 5, 6)],
                },
            ],
            ..Default::default()
        };
        d.apply(&st).unwrap();
        let regs = &d.bus().regs;
        assert_eq!(regs[&(0x61, REG_MODE)], 0x10);
        assert_eq!(regs[&(0x61, REG_LED_COLORS)], 1);
        assert_eq!(regs[&(0x61, REG_LED_COLORS + 1)], 2);
        assert_eq!(regs[&(0x61, REG_LED_COLORS + 2)], 3);
        assert_eq!(regs[&(0x63, REG_LED_COLORS + 11 * 3)], 4);
        assert_eq!(regs[&(0x63, REG_LED_COLORS + 11 * 3 + 2)], 6);
        // Direct mode preamble is unsynchronised: both indices 0.
        assert!(d.bus().ops.contains(&Op::WriteByteData(0x63, REG_INDEX, 0)));

        let st = DeviceState {
            mode: "slide".into(),
            colors: vec![Rgb::new(255, 0, 0), Rgb::new(0, 255, 0), Rgb::new(9, 9, 9)],
            speed: 0,
            ..Default::default()
        };
        d.apply(&st).unwrap();
        let regs = &d.bus().regs;
        assert_eq!(regs[&(0x61, REG_NUM_COLORS)], 2);
        assert_eq!(regs[&(0x61, REG_BG_RED)], 9);
        assert_eq!(regs[&(0x61, REG_SPEED)], 255);
        assert_eq!(regs[&(0x61, REG_LENGTH)], 4);
    }

    #[test]
    fn rain_speed_offsets_per_stick() {
        let mut d = device();
        let st = DeviceState {
            mode: "rain".into(),
            speed: 100,
            ..Default::default()
        };
        d.apply(&st).unwrap();
        let regs = &d.bus().regs;
        assert_eq!(regs[&(0x61, REG_SPEED)], 8 + 11);
        assert_eq!(regs[&(0x63, REG_SPEED)], 8);
    }

    #[test]
    fn cache_skips_repeated_writes() {
        let mut d = device();
        d.apply(&DeviceState::static_color(Rgb::WHITE)).unwrap();
        let n = d.bus().ops.len();
        d.apply(&DeviceState::static_color(Rgb::WHITE)).unwrap();
        // Only begin/end (4 writes) are repeated.
        assert_eq!(d.bus().ops.len() - n, 4);
    }
}
