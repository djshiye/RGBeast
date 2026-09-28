//! ENE lighting MCU over SMBus: ASUS graphics cards (TUF, ROG Strix, Astral)
//! at address 0x67, ENE-based DRAM, and older ASUS Aura SMBus boards.
//! Registers are 16-bit and reached indirectly through SMBus commands.

use crate::{
    Driver, Error, Result, Rgb,
    model::{DeviceInfo, DeviceKind, DeviceState, ModeInfo, ZoneInfo},
    transport::Smbus,
};

use super::map_speed;

pub const GPU_ADDRESS: u8 = 0x67;
pub const DRAM_ADDRESSES: &[u8] = &[
    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x4F, 0x66, 0x67, 0x39, 0x3A, 0x3B, 0x3C, 0x3D,
];
pub const MOBO_ADDRESSES: &[u8] = &[0x40, 0x4E, 0x4F];

const REG_DEVICE_NAME: u16 = 0x1000;
const REG_MICRON_CHECK: u16 = 0x1030;
const REG_CONFIG_TABLE: u16 = 0x1C00;
const REG_COLORS_DIRECT_V1: u16 = 0x8000;
const REG_COLORS_EFFECT_V1: u16 = 0x8010;
const REG_DIRECT: u16 = 0x8020;
const REG_MODE: u16 = 0x8021;
const REG_SPEED: u16 = 0x8022;
const REG_DIRECTION: u16 = 0x8023;
const REG_APPLY: u16 = 0x80A0;
const REG_COLORS_DIRECT_V2: u16 = 0x8100;
const REG_COLORS_EFFECT_V2: u16 = 0x8160;

const APPLY_VAL: u8 = 0x01;
const SAVE_VAL: u8 = 0xAA;
const MAX_BLOCK: usize = 3;
const MAX_LEDS: usize = 40;

const MODE_OFF: u8 = 0;
const MODE_STATIC: u8 = 1;
const MODE_BREATHING: u8 = 2;
const MODE_FLASHING: u8 = 3;
const MODE_SPECTRUM_CYCLE: u8 = 4;
const MODE_RAINBOW: u8 = 5;
const MODE_SPECTRUM_CYCLE_BREATHING: u8 = 6;
const MODE_CHASE_FADE: u8 = 7;
const MODE_SPECTRUM_CYCLE_CHASE_FADE: u8 = 8;
const MODE_CHASE: u8 = 9;
const MODE_SPECTRUM_CYCLE_CHASE: u8 = 10;
const MODE_RANDOM_FLICKER: u8 = 13;

/// Low-level register access shared by probe and driver.
fn reg_read<B: Smbus>(bus: &mut B, addr: u8, reg: u16) -> Result<u8> {
    bus.write_word_data(addr, 0x00, reg.swap_bytes())?;
    bus.read_byte_data(addr, 0x81)
}

fn reg_write<B: Smbus>(bus: &mut B, addr: u8, reg: u16, val: u8) -> Result<()> {
    bus.write_word_data(addr, 0x00, reg.swap_bytes())?;
    bus.write_byte_data(addr, 0x01, val)
}

fn reg_write_block<B: Smbus>(bus: &mut B, addr: u8, reg: u16, data: &[u8]) -> Result<()> {
    bus.write_word_data(addr, 0x00, reg.swap_bytes())?;
    if bus.write_block_data(addr, 0x03, data).is_err() {
        for b in data {
            bus.write_byte_data(addr, 0x01, *b)?;
        }
    }
    Ok(())
}

/// Is there an ENE lighting controller at `addr`? Mirrors the community
/// probe: the address answers, `0xA0..0xAF` read back as `0..15`, and the
/// name area does not say "Micron" (an SPD hub that would otherwise pass).
pub fn probe<B: Smbus>(bus: &mut B, addr: u8) -> bool {
    if bus.read_byte(addr).is_err() && bus.read_byte_data(addr, 0x00).is_err() {
        return false;
    }
    for i in 0xA0u8..0xB0 {
        match bus.read_byte_data(addr, i) {
            Ok(v) if v == i - 0xA0 => {}
            _ => return false,
        }
    }
    let mut name = [0u8; 16];
    for (i, b) in name.iter_mut().enumerate() {
        *b = reg_read(bus, addr, REG_MICRON_CHECK + i as u16).unwrap_or(0);
    }
    !name.starts_with(b"Micron")
}

fn channel_name(id: u8) -> &'static str {
    match id {
        0x82 | 0x83 => "Center",
        0x84 => "Audio",
        0x85 => "Back I/O",
        0x86 | 0x91 => "RGB Header",
        0x87 => "RGB Header 2",
        0x88 => "Backplate",
        0x8A | 0x05 | 0x0E => "DRAM",
        0x8B => "PCIe",
        _ => "Lighting",
    }
}

fn mode_list() -> Vec<ModeInfo> {
    vec![
        ModeInfo::new("direct", "Direct").per_led().brightness(),
        ModeInfo::new("off", "Off"),
        ModeInfo::new("static", "Static").per_led().brightness(),
        ModeInfo::new("breathing", "Breathing")
            .per_led()
            .brightness()
            .speed()
            .random(),
        ModeInfo::new("flashing", "Flashing")
            .per_led()
            .brightness()
            .speed(),
        ModeInfo::new("spectrum-cycle", "Spectrum Cycle").speed(),
        ModeInfo::new("rainbow", "Rainbow")
            .speed()
            .directions(&["forward", "reverse"]),
        ModeInfo::new("chase-fade", "Chase Fade")
            .per_led()
            .brightness()
            .speed()
            .directions(&["forward", "reverse"])
            .random(),
        ModeInfo::new("chase", "Chase")
            .per_led()
            .brightness()
            .speed()
            .directions(&["forward", "reverse"])
            .random(),
        ModeInfo::new("random-flicker", "Random Flicker").speed(),
    ]
}

fn mode_byte(id: &str, random: bool) -> Result<u8> {
    Ok(match (id, random) {
        ("direct", _) => MODE_STATIC,
        ("off", _) => MODE_OFF,
        ("static", _) => MODE_STATIC,
        ("breathing", false) => MODE_BREATHING,
        ("breathing", true) => MODE_SPECTRUM_CYCLE_BREATHING,
        ("flashing", _) => MODE_FLASHING,
        ("spectrum-cycle", _) => MODE_SPECTRUM_CYCLE,
        ("rainbow", _) => MODE_RAINBOW,
        ("chase-fade", false) => MODE_CHASE_FADE,
        ("chase-fade", true) => MODE_SPECTRUM_CYCLE_CHASE_FADE,
        ("chase", false) => MODE_CHASE,
        ("chase", true) => MODE_SPECTRUM_CYCLE_CHASE,
        ("random-flicker", _) => MODE_RANDOM_FLICKER,
        (other, _) => return Err(Error::invalid(format!("unknown ENE mode {other}"))),
    })
}

fn mode_id(byte: u8) -> (&'static str, bool) {
    match byte {
        MODE_OFF => ("off", false),
        MODE_STATIC => ("static", false),
        MODE_BREATHING => ("breathing", false),
        MODE_SPECTRUM_CYCLE_BREATHING => ("breathing", true),
        MODE_FLASHING => ("flashing", false),
        MODE_SPECTRUM_CYCLE => ("spectrum-cycle", false),
        MODE_RAINBOW => ("rainbow", false),
        MODE_CHASE_FADE => ("chase-fade", false),
        MODE_SPECTRUM_CYCLE_CHASE_FADE => ("chase-fade", true),
        MODE_CHASE => ("chase", false),
        MODE_SPECTRUM_CYCLE_CHASE => ("chase", true),
        MODE_RANDOM_FLICKER => ("random-flicker", false),
        _ => ("static", false),
    }
}

pub struct Ene<B: Smbus> {
    bus: B,
    addr: u8,
    info: DeviceInfo,
    direct_reg: u16,
    effect_reg: u16,
    led_count: usize,
}

impl<B: Smbus> Ene<B> {
    pub fn new(
        mut bus: B,
        addr: u8,
        id: String,
        name: String,
        kind: DeviceKind,
        location: String,
    ) -> Result<Self> {
        let mut raw_name = [0u8; 16];
        for (i, b) in raw_name.iter_mut().enumerate() {
            *b = reg_read(&mut bus, addr, REG_DEVICE_NAME + i as u16)?;
        }
        let end = raw_name.iter().position(|b| *b == 0).unwrap_or(16);
        let version = String::from_utf8_lossy(&raw_name[..end]).trim().to_string();

        let mut table = [0u8; 64];
        for (i, b) in table.iter_mut().enumerate() {
            *b = reg_read(&mut bus, addr, REG_CONFIG_TABLE + i as u16)?;
        }

        // Register layout generation and where the LED count lives.
        let (v2, channel_cfg, led_count) = match version.as_str() {
            "LED-0116" | "DIMM_LED-0102" | "AUMA0-E8K4-0101" => {
                (false, 0x13usize, table[0x02] as usize)
            }
            "AUDA0-E6K5-0101" => (true, 0x13, table[0x02] as usize),
            "AUMA0-E6K5-0104" | "AUMA0-E6K5-0105" | "AUMA0-E6K5-0106" => {
                (true, 0x1B, table[0x02] as usize)
            }
            "AUMA0-E6K5-0008" => (true, 0x13, table[0x03] as usize),
            v if v.starts_with("AUMA0-E6K5-01") || v.starts_with("AUMA0-E6K5-11") => {
                (true, 0x1B, table[0x03] as usize)
            }
            _ => {
                return Err(Error::protocol(format!(
                    "unknown ENE controller '{version}' at 0x{addr:02X}"
                )));
            }
        };
        let led_count = led_count.min(MAX_LEDS);
        if led_count == 0 {
            return Err(Error::protocol(format!(
                "ENE controller '{version}' reports no LEDs"
            )));
        }

        // Zones: consecutive config-table entries with a LED count, named by channel ID.
        let mut zones: Vec<ZoneInfo> = Vec::new();
        let mut assigned = 0usize;
        for z in 0..8usize {
            let n = table[0x03 + z] as usize;
            if n == 0 || assigned >= led_count {
                continue;
            }
            let n = n.min(led_count - assigned);
            let base = channel_name(table[channel_cfg + z]);
            let count = zones.iter().filter(|zz| zz.name.starts_with(base)).count();
            let name = if count == 0 {
                base.to_string()
            } else {
                format!("{base} {}", count + 1)
            };
            zones.push(ZoneInfo::fixed(
                &format!("zone{}", zones.len() + 1),
                &name,
                n as u32,
            ));
            assigned += n;
        }
        if assigned < led_count {
            zones.push(ZoneInfo::fixed(
                "zone-main",
                "Lighting",
                (led_count - assigned) as u32,
            ));
        }

        let info = DeviceInfo {
            id,
            name,
            vendor: "ASUS".into(),
            kind,
            location,
            driver: "ene-smbus".into(),
            version,
            zones,
            modes: mode_list(),
            can_save: true,
        };
        Ok(Ene {
            bus,
            addr,
            info,
            direct_reg: if v2 {
                REG_COLORS_DIRECT_V2
            } else {
                REG_COLORS_DIRECT_V1
            },
            effect_reg: if v2 {
                REG_COLORS_EFFECT_V2
            } else {
                REG_COLORS_EFFECT_V1
            },
            led_count,
        })
    }

    fn write(&mut self, reg: u16, val: u8) -> Result<()> {
        reg_write(&mut self.bus, self.addr, reg, val)
    }

    fn read(&mut self, reg: u16) -> Result<u8> {
        reg_read(&mut self.bus, self.addr, reg)
    }

    /// All LED colours in the controller's R,B,G order, 3 bytes per block.
    fn write_colors(&mut self, base: u16, colors: &[Rgb]) -> Result<()> {
        let mut buf = Vec::with_capacity(colors.len() * 3);
        for c in colors.iter().take(self.led_count) {
            buf.extend_from_slice(&[c.r, c.b, c.g]);
        }
        for (i, chunk) in buf.chunks(MAX_BLOCK).enumerate() {
            reg_write_block(
                &mut self.bus,
                self.addr,
                base + (i * MAX_BLOCK) as u16,
                chunk,
            )?;
        }
        Ok(())
    }

    fn all_colors(&self, state: &DeviceState) -> Vec<Rgb> {
        let mut out = Vec::with_capacity(self.led_count);
        for z in &self.info.zones {
            out.extend(
                state
                    .zone_colors(&z.id, z.leds)
                    .into_iter()
                    .map(|c| c.scaled(state.brightness)),
            );
        }
        out.truncate(self.led_count);
        out
    }

    #[cfg(test)]
    pub(crate) fn bus(&self) -> &B {
        &self.bus
    }
}

impl<B: Smbus> Driver for Ene<B> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        let colors = self.all_colors(state);
        if state.mode == "direct" {
            self.write_colors(self.direct_reg, &colors)?;
            self.write(REG_DIRECT, 1)?;
            self.write(REG_APPLY, APPLY_VAL)?;
            return Ok(());
        }
        let mode = mode_byte(&state.mode, state.random)?;
        let speed = map_speed(state.speed, 4, 0);
        let direction = if state.direction == "reverse" { 1 } else { 0 };
        self.write(REG_DIRECT, 0)?;
        self.write(REG_MODE, mode)?;
        self.write(REG_SPEED, speed)?;
        self.write(REG_DIRECTION, direction)?;
        self.write(REG_APPLY, APPLY_VAL)?;
        if mode != MODE_OFF {
            self.write_colors(self.effect_reg, &colors)?;
            self.write(REG_APPLY, APPLY_VAL)?;
        }
        Ok(())
    }

    fn read_state(&mut self) -> Result<Option<DeviceState>> {
        let direct = self.read(REG_DIRECT)? != 0;
        let mode = self.read(REG_MODE)?;
        let speed = self.read(REG_SPEED)?;
        let direction = self.read(REG_DIRECTION)?;
        let base = if direct {
            self.direct_reg
        } else {
            self.effect_reg
        };
        let mut colors = Vec::with_capacity(self.led_count);
        for i in 0..self.led_count as u16 {
            let r = self.read(base + i * 3)?;
            let b = self.read(base + i * 3 + 1)?;
            let g = self.read(base + i * 3 + 2)?;
            colors.push(Rgb::new(r, g, b));
        }
        let (id, random) = if direct {
            ("direct", false)
        } else {
            mode_id(mode)
        };
        let mut zones = Vec::new();
        let mut it = colors.iter().copied();
        for z in &self.info.zones {
            zones.push(crate::model::ZoneState {
                id: z.id.clone(),
                colors: it.by_ref().take(z.leds as usize).collect(),
            });
        }
        let speed_pct = ((4 - speed.min(4)) as u32) * 25;
        Ok(Some(DeviceState {
            mode: id.into(),
            colors: vec![colors.first().copied().unwrap_or(Rgb::WHITE)],
            brightness: 100,
            speed: speed_pct,
            direction: if direction == 1 {
                "reverse".into()
            } else {
                "forward".into()
            },
            random,
            zones,
        }))
    }

    fn save(&mut self) -> Result<()> {
        self.write(REG_APPLY, SAVE_VAL)
    }
}

pub fn modes() -> Vec<ModeInfo> {
    mode_list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::{MockSmbus, Op};

    /// A TUF GPU controller: AUMA0-E6K5-0107, 4 LEDs in one "Center" zone.
    fn gpu_bus() -> MockSmbus {
        let mut bus = MockSmbus::with_addresses(&[GPU_ADDRESS]);
        for i in 0xA0u8..0xB0 {
            bus.regs.insert((GPU_ADDRESS, i), i - 0xA0);
        }
        for (i, b) in b"AUMA0-E6K5-0107".iter().enumerate() {
            bus.ene
                .insert((GPU_ADDRESS, REG_DEVICE_NAME + i as u16), *b);
        }
        bus.ene.insert((GPU_ADDRESS, REG_CONFIG_TABLE + 0x03), 4);
        bus.ene.insert((GPU_ADDRESS, REG_CONFIG_TABLE + 0x1B), 0x83);
        bus
    }

    #[test]
    fn probe_accepts_ene_and_rejects_others() {
        let mut bus = gpu_bus();
        assert!(probe(&mut bus, GPU_ADDRESS));
        assert!(!probe(&mut bus, 0x50));
        let mut micron = gpu_bus();
        for (i, b) in b"Micron".iter().enumerate() {
            micron
                .ene
                .insert((GPU_ADDRESS, REG_MICRON_CHECK + i as u16), *b);
        }
        assert!(!probe(&mut micron, GPU_ADDRESS));
        let mut broken = gpu_bus();
        broken.regs.insert((GPU_ADDRESS, 0xA5), 0x77);
        assert!(!probe(&mut broken, GPU_ADDRESS));
    }

    #[test]
    fn register_access_encoding() {
        let mut bus = gpu_bus();
        reg_write(&mut bus, GPU_ADDRESS, 0x8021, 5).unwrap();
        assert_eq!(bus.ops[0], Op::WriteWordData(GPU_ADDRESS, 0x00, 0x2180));
        assert_eq!(bus.ops[1], Op::WriteByteData(GPU_ADDRESS, 0x01, 5));
        assert_eq!(bus.ene[&(GPU_ADDRESS, 0x8021)], 5);
        assert_eq!(reg_read(&mut bus, GPU_ADDRESS, 0x8021).unwrap(), 5);
        assert_eq!(bus.ops[3], Op::ReadByteData(GPU_ADDRESS, 0x81));
    }

    #[test]
    fn identifies_gpu_and_zones() {
        let d = Ene::new(
            gpu_bus(),
            GPU_ADDRESS,
            "ene:test".into(),
            "TUF RX 9070".into(),
            DeviceKind::Gpu,
            "i2c-9".into(),
        )
        .unwrap();
        assert_eq!(d.info().version, "AUMA0-E6K5-0107");
        assert_eq!(d.info().zones.len(), 1);
        assert_eq!(d.info().zones[0].name, "Center");
        assert_eq!(d.info().zones[0].leds, 4);
        assert_eq!(d.direct_reg, REG_COLORS_DIRECT_V2);
    }

    #[test]
    fn static_mode_writes_effect_colors_in_rbg_order() {
        let mut d = Ene::new(
            gpu_bus(),
            GPU_ADDRESS,
            "e".into(),
            "g".into(),
            DeviceKind::Gpu,
            "l".into(),
        )
        .unwrap();
        d.bus.ops.clear();
        let st = DeviceState::static_color(Rgb::new(10, 20, 30));
        d.apply(&st).unwrap();
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_DIRECT)], 0);
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_MODE)], MODE_STATIC);
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_SPEED)], 2);
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_DIRECTION)], 0);
        for led in 0..4u16 {
            let base = REG_COLORS_EFFECT_V2 + led * 3;
            assert_eq!(d.bus().ene[&(GPU_ADDRESS, base)], 10);
            assert_eq!(d.bus().ene[&(GPU_ADDRESS, base + 1)], 30);
            assert_eq!(d.bus().ene[&(GPU_ADDRESS, base + 2)], 20);
        }
        let applies = d
            .bus()
            .ops
            .iter()
            .filter(|o| matches!(o, Op::WriteByteData(_, 0x01, APPLY_VAL)))
            .count();
        assert!(applies >= 2);
        // Blocks are 3 bytes at command 0x03.
        assert!(
            d.bus()
                .ops
                .iter()
                .any(|o| matches!(o, Op::WriteBlockData(_, 0x03, v) if v.len() == 3))
        );
    }

    #[test]
    fn direct_mode_and_readback() {
        let mut d = Ene::new(
            gpu_bus(),
            GPU_ADDRESS,
            "e".into(),
            "g".into(),
            DeviceKind::Gpu,
            "l".into(),
        )
        .unwrap();
        let st = DeviceState {
            mode: "direct".into(),
            zones: vec![crate::model::ZoneState {
                id: "zone1".into(),
                colors: vec![
                    Rgb::new(1, 2, 3),
                    Rgb::new(4, 5, 6),
                    Rgb::new(7, 8, 9),
                    Rgb::new(10, 11, 12),
                ],
            }],
            brightness: 100,
            ..Default::default()
        };
        d.apply(&st).unwrap();
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_DIRECT)], 1);
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_COLORS_DIRECT_V2 + 9)], 10);
        let back = d.read_state().unwrap().unwrap();
        assert_eq!(back.mode, "direct");
        assert_eq!(back.zones[0].colors, st.zones[0].colors);
    }

    #[test]
    fn random_variants_and_speed_direction() {
        let mut d = Ene::new(
            gpu_bus(),
            GPU_ADDRESS,
            "e".into(),
            "g".into(),
            DeviceKind::Gpu,
            "l".into(),
        )
        .unwrap();
        let st = DeviceState {
            mode: "chase".into(),
            random: true,
            speed: 100,
            direction: "reverse".into(),
            ..Default::default()
        };
        d.apply(&st).unwrap();
        assert_eq!(
            d.bus().ene[&(GPU_ADDRESS, REG_MODE)],
            MODE_SPECTRUM_CYCLE_CHASE
        );
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_SPEED)], 0);
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_DIRECTION)], 1);
        let back = d.read_state().unwrap().unwrap();
        assert_eq!(back.mode, "chase");
        assert!(back.random);
        assert_eq!(back.speed, 100);
        assert_eq!(back.direction, "reverse");
    }

    #[test]
    fn block_write_fallback() {
        let mut bus = gpu_bus();
        bus.reject_blocks = true;
        let mut d = Ene::new(
            bus,
            GPU_ADDRESS,
            "e".into(),
            "g".into(),
            DeviceKind::Gpu,
            "l".into(),
        )
        .unwrap();
        d.apply(&DeviceState::static_color(Rgb::new(9, 9, 9)))
            .unwrap();
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_COLORS_EFFECT_V2 + 11)], 9);
        d.save().unwrap();
        assert_eq!(d.bus().ene[&(GPU_ADDRESS, REG_APPLY)], SAVE_VAL);
    }

    #[test]
    fn unknown_controller_is_refused() {
        let mut bus = gpu_bus();
        for i in 0..16u16 {
            bus.ene.insert((GPU_ADDRESS, REG_DEVICE_NAME + i), 0);
        }
        for (i, b) in b"WEIRD-0001".iter().enumerate() {
            bus.ene
                .insert((GPU_ADDRESS, REG_DEVICE_NAME + i as u16), *b);
        }
        assert!(
            Ene::new(
                bus,
                GPU_ADDRESS,
                "e".into(),
                "g".into(),
                DeviceKind::Gpu,
                "l".into()
            )
            .is_err()
        );
    }
}
