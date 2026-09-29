//! Sapphire Nitro Glow V3: the lighting MCU on Sapphire Nitro+, Pure and
//! Toxic cards from the RX 5700 XT onwards, at I2C address 0x28 on the
//! card's own bus. Plain 8-bit registers, no transaction protocol: a write
//! takes effect at once. The card has one lighting zone.
//!
//! Register map (see `docs/PROTOCOLS.md`): mode at 0x10, one speed register
//! per animated mode, the colour at 0x1A..0x1C, an "external control" flag at
//! 0x0F that hands the LEDs to host software (never set here).

use crate::{
    Driver, Error, Result, Rgb,
    model::{DeviceInfo, DeviceKind, DeviceState, ModeInfo, ZoneInfo},
    transport::Smbus,
};

use super::map_speed;

pub const ADDRESS: u8 = 0x28;
/// PCI subsystem vendor of Sapphire cards.
pub const SAPPHIRE_VENDOR: u16 = 0x1DA2;

const REG_EXTERNAL_CONTROL: u8 = 0x0F;
const REG_MODE: u8 = 0x10;
const REG_RUNWAY_SPEED: u8 = 0x11;
const REG_CYCLE_SPEED: u8 = 0x13;
const REG_RAINBOW_SPEED: u8 = 0x15;
const REG_SERIAL_SPEED: u8 = 0x16;
const REG_RED: u8 = 0x1A;
const REG_GREEN: u8 = 0x1B;
const REG_BLUE: u8 = 0x1C;

const MODE_RAINBOW: u8 = 0x00;
const MODE_RUNWAY: u8 = 0x01;
const MODE_COLOR_CYCLE: u8 = 0x02;
const MODE_SERIAL: u8 = 0x03;
const MODE_CUSTOM: u8 = 0x06;
const MODE_OFF: u8 = 0x07;
const MODE_EXTERNAL: u8 = 0xFF;

/// (id, name, mode byte, speed register, slowest raw, fastest raw)
struct ModeDef {
    id: &'static str,
    name: &'static str,
    byte: u8,
    speed: Option<(u8, i32, i32)>,
}

const MODES: &[ModeDef] = &[
    ModeDef {
        id: "static",
        name: "Static",
        byte: MODE_CUSTOM,
        speed: None,
    },
    ModeDef {
        id: "off",
        name: "Off",
        byte: MODE_OFF,
        speed: None,
    },
    ModeDef {
        id: "rainbow",
        name: "Rainbow",
        byte: MODE_RAINBOW,
        speed: Some((REG_RAINBOW_SPEED, 10, 250)),
    },
    ModeDef {
        id: "spectrum-cycle",
        name: "Spectrum Cycle",
        byte: MODE_COLOR_CYCLE,
        speed: Some((REG_CYCLE_SPEED, 30, 1)),
    },
    ModeDef {
        id: "runway",
        name: "Runway",
        byte: MODE_RUNWAY,
        speed: Some((REG_RUNWAY_SPEED, 5, 50)),
    },
    ModeDef {
        id: "serial",
        name: "Serial",
        byte: MODE_SERIAL,
        speed: Some((REG_SERIAL_SPEED, 255, 5)),
    },
];

fn mode_list() -> Vec<ModeInfo> {
    MODES
        .iter()
        .map(|m| {
            let mut info = ModeInfo::new(m.id, m.name);
            if m.id == "static" {
                info = info.colors(1, 1).brightness();
            }
            if m.speed.is_some() {
                info = info.speed();
            }
            info
        })
        .collect()
}

/// Which modes exist, for callers that build sim devices.
pub fn modes() -> Vec<ModeInfo> {
    mode_list()
}

fn mode_def(id: &str) -> Result<&'static ModeDef> {
    MODES
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| Error::invalid(format!("unknown Sapphire mode {id}")))
}

/// A controller answers at 0x28 and its mode register holds a known mode.
pub fn probe<B: Smbus>(bus: &mut B) -> bool {
    if bus.read_byte(ADDRESS).is_err() {
        return false;
    }
    matches!(
        bus.read_byte_data(ADDRESS, REG_MODE),
        Ok(MODE_RAINBOW..=MODE_OFF) | Ok(MODE_EXTERNAL)
    )
}

pub struct Sapphire<B: Smbus> {
    bus: B,
    info: DeviceInfo,
}

impl<B: Smbus> Sapphire<B> {
    pub fn new(bus: B, id: String, name: String, location: String) -> Result<Self> {
        let info = DeviceInfo {
            id,
            name,
            vendor: "Sapphire".into(),
            kind: DeviceKind::Gpu,
            location,
            driver: "sapphire".into(),
            version: "Nitro Glow V3".into(),
            zones: vec![ZoneInfo::fixed("logo", "Logo", 1)],
            modes: mode_list(),
            can_save: false,
        };
        Ok(Sapphire { bus, info })
    }

    fn write(&mut self, reg: u8, val: u8) -> Result<()> {
        self.bus.write_byte_data(ADDRESS, reg, val)
    }

    fn read(&mut self, reg: u8) -> Result<u8> {
        self.bus.read_byte_data(ADDRESS, reg)
    }
}

impl<B: Smbus> Driver for Sapphire<B> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        let def = mode_def(&state.mode)?;
        // Make sure the MCU, not a host program, drives the LEDs.
        self.write(REG_EXTERNAL_CONTROL, 0)?;
        if let Some((reg, slowest, fastest)) = def.speed {
            self.write(reg, map_speed(state.speed, slowest, fastest))?;
        }
        // The controller has no usable brightness register for the custom
        // colour, so brightness scales the colour itself.
        let color = match def.id {
            "static" => state.primary_color().scaled(state.brightness),
            "off" => Rgb::BLACK,
            _ => state.primary_color(),
        };
        if matches!(def.id, "static" | "off") {
            self.write(REG_RED, color.r)?;
            self.write(REG_GREEN, color.g)?;
            self.write(REG_BLUE, color.b)?;
        }
        self.write(REG_MODE, def.byte)
    }

    fn read_state(&mut self) -> Result<Option<DeviceState>> {
        let byte = self.read(REG_MODE)?;
        let Some(def) = MODES.iter().find(|m| m.byte == byte) else {
            return Ok(None);
        };
        let color = Rgb::new(
            self.read(REG_RED)?,
            self.read(REG_GREEN)?,
            self.read(REG_BLUE)?,
        );
        let speed = match def.speed {
            Some((reg, slowest, fastest)) => {
                let raw = self.read(reg)? as i32;
                ((raw - slowest) * 100 / (fastest - slowest)).clamp(0, 100) as u32
            }
            None => 50,
        };
        Ok(Some(DeviceState {
            mode: def.id.into(),
            colors: if def.id == "static" {
                vec![color]
            } else {
                vec![]
            },
            brightness: 100,
            speed,
            direction: String::new(),
            random: false,
            zones: vec![],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::{MockSmbus, Op};

    fn card() -> MockSmbus {
        let mut bus = MockSmbus::with_addresses(&[ADDRESS]);
        // As read from a real RX 9070 XT Pure: custom mode, magenta.
        bus.regs.insert((ADDRESS, REG_MODE), MODE_CUSTOM);
        bus.regs.insert((ADDRESS, REG_RED), 0xFF);
        bus.regs.insert((ADDRESS, REG_GREEN), 0x00);
        bus.regs.insert((ADDRESS, REG_BLUE), 0xFF);
        bus.regs.insert((ADDRESS, REG_RAINBOW_SPEED), 50);
        bus
    }

    #[test]
    fn probe_needs_a_known_mode() {
        assert!(probe(&mut card()));
        let mut odd = card();
        odd.regs.insert((ADDRESS, REG_MODE), 0x42);
        assert!(!probe(&mut odd));
        assert!(!probe(&mut MockSmbus::with_addresses(&[0x67])));
    }

    #[test]
    fn reads_the_card_state() {
        let mut d = Sapphire::new(card(), "x".into(), "x".into(), "x".into()).unwrap();
        let s = d.read_state().unwrap().unwrap();
        assert_eq!(s.mode, "static");
        assert_eq!(s.colors, vec![Rgb::new(255, 0, 255)]);
    }

    #[test]
    fn static_writes_colour_then_mode() {
        let mut d = Sapphire::new(card(), "x".into(), "x".into(), "x".into()).unwrap();
        let mut st = DeviceState::static_color(Rgb::new(0x91, 0x41, 0xAC));
        st.brightness = 50;
        d.apply(&st).unwrap();
        let w = d.bus.writes();
        assert_eq!(*w[0], Op::WriteByteData(ADDRESS, REG_EXTERNAL_CONTROL, 0));
        assert_eq!(*w[1], Op::WriteByteData(ADDRESS, REG_RED, 0x48));
        assert_eq!(*w[2], Op::WriteByteData(ADDRESS, REG_GREEN, 0x20));
        assert_eq!(*w[3], Op::WriteByteData(ADDRESS, REG_BLUE, 0x56));
        assert_eq!(*w[4], Op::WriteByteData(ADDRESS, REG_MODE, MODE_CUSTOM));
        assert_eq!(w.len(), 5);
    }

    #[test]
    fn animated_modes_write_their_speed_register() {
        let mut d = Sapphire::new(card(), "x".into(), "x".into(), "x".into()).unwrap();
        let st = DeviceState {
            mode: "rainbow".into(),
            speed: 100,
            colors: vec![],
            ..Default::default()
        };
        d.apply(&st).unwrap();
        assert_eq!(d.bus.regs[&(ADDRESS, REG_RAINBOW_SPEED)], 250);
        assert_eq!(d.bus.regs[&(ADDRESS, REG_MODE)], MODE_RAINBOW);
        assert_eq!(d.read_state().unwrap().unwrap().speed, 100);
        let st = DeviceState {
            mode: "spectrum-cycle".into(),
            speed: 0,
            colors: vec![],
            ..Default::default()
        };
        d.apply(&st).unwrap();
        assert_eq!(d.bus.regs[&(ADDRESS, REG_CYCLE_SPEED)], 30); // slowest
        d.apply(&DeviceState::off()).unwrap();
        assert_eq!(d.bus.regs[&(ADDRESS, REG_MODE)], MODE_OFF);
        assert_eq!(d.bus.regs[&(ADDRESS, REG_RED)], 0);
    }
}
