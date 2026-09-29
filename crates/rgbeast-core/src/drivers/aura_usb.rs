//! ASUS Aura USB mainboard controller (the lighting MCU inside current ASUS
//! boards, including TUF Gaming AM5 models). Drives the on-board LEDs, the
//! 12 V RGB headers and the addressable Gen 2 headers.

use crate::{
    Driver, Error, Result, Rgb,
    model::{DeviceInfo, DeviceKind, DeviceState, ModeInfo, ZoneInfo},
    transport::HidTransport,
};

pub const VENDOR_ID: u16 = 0x0B05;
/// Product IDs of the mainboard controller generation this driver speaks.
pub const PRODUCT_IDS: &[u16] = &[0x18F3, 0x1939, 0x19AF, 0x1AA6, 0x1BED];
/// Interface filter for `0x19AF`: usage page and usage of the lighting interface.
pub const USAGE_PAGE: u16 = 0xFF72;
pub const USAGE: u16 = 0x00A1;

pub const MAX_ADDRESSABLE_LEDS: u32 = 120;
const REPORT_LEN: usize = 65;
const LEDS_PER_DIRECT_PACKET: usize = 20;
/// Colours carried by one effect-colour packet (60 bytes after the header).
const EFFECT_COLOR_LEDS: u8 = 20;
const READ_TIMEOUT_MS: u32 = 1000;

const CMD_PREFIX: u8 = 0xEC;
const CMD_FIRMWARE: u8 = 0x82;
const CMD_CONFIG: u8 = 0xB0;
const CMD_DIRECT: u8 = 0x40;
const CMD_EFFECT: u8 = 0x35;
const CMD_EFFECT_COLOR: u8 = 0x36;
const CMD_COMMIT: u8 = 0x3F;

const MODE_OFF: u8 = 0;
const MODE_STATIC: u8 = 1;
const MODE_BREATHING: u8 = 2;
const MODE_FLASHING: u8 = 3;
const MODE_SPECTRUM_CYCLE: u8 = 4;
const MODE_RAINBOW: u8 = 5;
const MODE_CHASE_FADE: u8 = 7;
const MODE_CHASE: u8 = 9;
const MODE_DIRECT: u8 = 0xFF;

#[derive(Clone, Debug)]
struct Channel {
    #[allow(dead_code)]
    zone: String,
    effect_channel: u8,
    direct_channel: u8,
    /// LEDs as counted by the controller's effect engine (1 for a header).
    effect_leds: u8,
    addressable: bool,
}

pub struct AuraUsb<T: HidTransport> {
    hid: T,
    info: DeviceInfo,
    channels: Vec<Channel>,
}

fn mode_list() -> Vec<ModeInfo> {
    vec![
        ModeInfo::new("direct", "Direct").per_led().brightness(),
        ModeInfo::new("off", "Off"),
        ModeInfo::new("static", "Static").colors(1, 1).brightness(),
        ModeInfo::new("breathing", "Breathing")
            .colors(1, 1)
            .brightness(),
        ModeInfo::new("flashing", "Flashing")
            .colors(1, 1)
            .brightness(),
        ModeInfo::new("spectrum-cycle", "Spectrum Cycle"),
        ModeInfo::new("rainbow", "Rainbow"),
        ModeInfo::new("chase-fade", "Chase Fade")
            .colors(1, 1)
            .brightness(),
        ModeInfo::new("chase", "Chase").colors(1, 1).brightness(),
    ]
}

fn mode_byte(id: &str) -> Result<u8> {
    Ok(match id {
        "direct" => MODE_DIRECT,
        "off" => MODE_OFF,
        "static" => MODE_STATIC,
        "breathing" => MODE_BREATHING,
        "flashing" => MODE_FLASHING,
        "spectrum-cycle" => MODE_SPECTRUM_CYCLE,
        "rainbow" => MODE_RAINBOW,
        "chase-fade" => MODE_CHASE_FADE,
        "chase" => MODE_CHASE,
        other => return Err(Error::invalid(format!("unknown Aura mode {other}"))),
    })
}

fn packet(cmd: u8) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = CMD_PREFIX;
    p[1] = cmd;
    p
}

impl<T: HidTransport> AuraUsb<T> {
    /// Talk to the controller, read its identity and build the device
    /// description. `header_leds` gives the LED count previously configured
    /// for each addressable header (missing entries mean 0).
    pub fn new(
        mut hid: T,
        id: String,
        name: String,
        location: String,
        header_leds: &[u32],
    ) -> Result<Self> {
        let version = Self::firmware(&mut hid)?;
        let table = Self::config_table(&mut hid)?;
        // Documented layout of the 60-byte table.
        let mut onboard_leds = table[0x1B];
        let addressable = table[0x02];
        if onboard_leds > EFFECT_COLOR_LEDS {
            // An effect-colour packet carries 20 colours (60 bytes) and a
            // 16-bit mask; a bigger count would be a misread table.
            onboard_leds = EFFECT_COLOR_LEDS;
        }

        let mut zones = Vec::new();
        let mut channels = Vec::new();
        let mut effect_channel = 0u8;
        if onboard_leds > 0 {
            // The on-board LEDs plus the 12 V RGB headers form one channel.
            zones.push(ZoneInfo::fixed(
                "mainboard",
                "Mainboard",
                onboard_leds as u32,
            ));
            channels.push(Channel {
                zone: "mainboard".into(),
                effect_channel,
                direct_channel: 0x04,
                effect_leds: onboard_leds,
                addressable: false,
            });
            effect_channel += 1;
        }
        for i in 0..addressable.min(8) {
            let zone_id = format!("argb{}", i + 1);
            let leds = header_leds
                .get(i as usize)
                .copied()
                .unwrap_or(0)
                .min(MAX_ADDRESSABLE_LEDS);
            zones.push(ZoneInfo::sizable(
                &zone_id,
                &format!("Addressable Header {}", i + 1),
                leds,
                MAX_ADDRESSABLE_LEDS,
            ));
            channels.push(Channel {
                zone: zone_id,
                effect_channel,
                direct_channel: i,
                effect_leds: 1,
                addressable: true,
            });
            effect_channel += 1;
        }
        if zones.is_empty() {
            return Err(Error::protocol(
                "Aura controller reports no LEDs and no headers",
            ));
        }

        // "Gen 1" handshake, sent once after reading the table.
        let mut p = packet(0x52);
        p[2] = 0x53;
        p[3] = 0x00;
        p[4] = 0x01;
        hid.write(&p)?;

        let info = DeviceInfo {
            id,
            name,
            vendor: "ASUS".into(),
            kind: DeviceKind::Motherboard,
            location,
            driver: "aura-usb".into(),
            version,
            zones,
            modes: mode_list(),
            can_save: true,
        };
        Ok(AuraUsb {
            hid,
            info,
            channels,
        })
    }

    fn firmware(hid: &mut T) -> Result<String> {
        hid.write(&packet(CMD_FIRMWARE))?;
        let mut buf = [0u8; REPORT_LEN];
        let n = hid.read(&mut buf, READ_TIMEOUT_MS)?;
        if n < 18 || buf[1] != 0x02 {
            return Err(Error::protocol("no firmware reply from Aura controller"));
        }
        let end = buf[2..18].iter().position(|b| *b == 0).unwrap_or(16);
        Ok(String::from_utf8_lossy(&buf[2..2 + end]).trim().to_string())
    }

    fn config_table(hid: &mut T) -> Result<[u8; 60]> {
        hid.write(&packet(CMD_CONFIG))?;
        let mut buf = [0u8; REPORT_LEN];
        let n = hid.read(&mut buf, READ_TIMEOUT_MS)?;
        if n < 64 || buf[1] != 0x30 {
            return Err(Error::protocol(
                "no config table reply from Aura controller",
            ));
        }
        let mut table = [0u8; 60];
        table.copy_from_slice(&buf[4..64]);
        Ok(table)
    }

    fn send_direct(&mut self, channel: u8, colors: &[Rgb]) -> Result<()> {
        let total = colors.len().min(MAX_ADDRESSABLE_LEDS as usize);
        let mut offset = 0usize;
        loop {
            let count = (total - offset).min(LEDS_PER_DIRECT_PACKET);
            let last = offset + count >= total;
            let mut p = packet(CMD_DIRECT);
            p[2] = if last { 0x80 | channel } else { channel };
            p[3] = offset as u8;
            p[4] = count as u8;
            for (i, c) in colors[offset..offset + count].iter().enumerate() {
                p[5 + i * 3] = c.r;
                p[6 + i * 3] = c.g;
                p[7 + i * 3] = c.b;
            }
            self.hid.write(&p)?;
            offset += count;
            if last {
                break;
            }
        }
        Ok(())
    }

    fn send_effect(&mut self, effect_channel: u8, mode: u8, shutdown: bool) -> Result<()> {
        let mut p = packet(CMD_EFFECT);
        p[2] = effect_channel;
        p[3] = 0;
        p[4] = shutdown as u8;
        p[5] = mode;
        self.hid.write(&p)
    }

    fn send_effect_color(
        &mut self,
        start_led: u8,
        count: u8,
        color: Rgb,
        shutdown: bool,
    ) -> Result<()> {
        let count = count.min(EFFECT_COLOR_LEDS);
        if start_led as usize + count as usize > 16 {
            return Err(Error::Protocol(format!(
                "effect colour range {start_led}+{count} exceeds the 16-bit LED mask"
            )));
        }
        let mask: u16 = (((1u32 << count) - 1) << start_led) as u16;
        let mut p = packet(CMD_EFFECT_COLOR);
        p[2] = (mask >> 8) as u8;
        p[3] = (mask & 0xFF) as u8;
        p[4] = shutdown as u8;
        for i in 0..count as usize {
            let base = 5 + 3 * (start_led as usize + i);
            if base + 2 >= REPORT_LEN {
                break;
            }
            p[base] = color.r;
            p[base + 1] = color.g;
            p[base + 2] = color.b;
        }
        self.hid.write(&p)
    }

    /// Effect mode for one channel: the effect packet, then (unless direct
    /// or off) the colour packet covering that channel's LEDs.
    fn set_channel_mode(&mut self, idx: usize, mode: u8, color: Rgb, shutdown: bool) -> Result<()> {
        let ch = self.channels[idx].clone();
        self.send_effect(ch.effect_channel, mode, shutdown)?;
        if mode == MODE_DIRECT || mode == MODE_OFF {
            return Ok(());
        }
        let start: u8 = self.channels[..idx].iter().map(|c| c.effect_leds).sum();
        self.send_effect_color(start, ch.effect_leds, color, shutdown)
    }

    fn active_channels(&self) -> Vec<usize> {
        (0..self.channels.len())
            .filter(|i| self.info.zones[*i].leds > 0)
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn transport(&self) -> &T {
        &self.hid
    }
}

impl<T: HidTransport> Driver for AuraUsb<T> {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        let mode = mode_byte(&state.mode)?;
        let color = state.primary_color().scaled(state.brightness);
        for idx in self.active_channels() {
            self.set_channel_mode(idx, mode, color, false)?;
        }
        if mode == MODE_DIRECT {
            for idx in self.active_channels() {
                let zone = self.info.zones[idx].clone();
                let colors: Vec<Rgb> = state
                    .zone_colors(&zone.id, zone.leds)
                    .into_iter()
                    .map(|c| c.scaled(state.brightness))
                    .collect();
                let ch = self.channels[idx].direct_channel;
                self.send_direct(ch, &colors)?;
            }
        }
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        // The shutdown effect only applies to on-board lighting; the commit
        // stores whatever effect is active as the power-on default.
        let mut p = packet(CMD_COMMIT);
        p[2] = 0x55;
        self.hid.write(&p)
    }

    fn set_zone_leds(&mut self, zone: &str, leds: u32) -> Result<()> {
        let (i, z) = self
            .info
            .zones
            .iter_mut()
            .enumerate()
            .find(|(_, z)| z.id == zone)
            .ok_or_else(|| Error::invalid(format!("no zone {zone}")))?;
        if !self.channels[i].addressable {
            return Err(Error::Unsupported(
                "only addressable headers can be resized".into(),
            ));
        }
        z.leds = leds.min(MAX_ADDRESSABLE_LEDS);
        Ok(())
    }
}

/// Which modes exist, for callers that build sim devices.
pub fn modes() -> Vec<ModeInfo> {
    mode_list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::ZoneState, transport::mock::MockHid};

    fn firmware_reply() -> Vec<u8> {
        let mut r = vec![0u8; 65];
        r[1] = 0x02;
        r[2..2 + 8].copy_from_slice(b"AUMA0-E6");
        r
    }

    /// 2 addressable headers, 8 on-board LEDs of which 2 are RGB headers.
    fn config_reply() -> Vec<u8> {
        let mut r = vec![0u8; 65];
        r[1] = 0x30;
        r[4 + 0x02] = 2;
        r[4 + 0x1B] = 8;
        r[4 + 0x1D] = 2;
        r
    }

    fn device(header_leds: &[u32]) -> AuraUsb<MockHid> {
        let hid = MockHid::with_replies(vec![firmware_reply(), config_reply()]);
        AuraUsb::new(
            hid,
            "aura-usb:test".into(),
            "ASUS TUF".into(),
            "hidraw0".into(),
            header_leds,
        )
        .unwrap()
    }

    #[test]
    fn handshake_and_zones() {
        let d = device(&[12, 0]);
        let w = &d.transport().written;
        assert_eq!(w[0][..2], [0xEC, 0x82]);
        assert_eq!(w[1][..2], [0xEC, 0xB0]);
        assert_eq!(w[2][..5], [0xEC, 0x52, 0x53, 0x00, 0x01]);
        assert_eq!(d.info().version, "AUMA0-E6");
        assert_eq!(d.info().zones.len(), 3);
        assert_eq!(d.info().zones[0].leds, 8);
        assert_eq!(d.info().zones[1].leds, 12);
        assert!(d.info().zones[1].is_sizable());
        assert_eq!(d.info().zones[2].leds, 0);
        assert_eq!(d.info().led_count(), 20);
    }

    #[test]
    fn static_mode_packets() {
        let mut d = device(&[12, 0]);
        let n0 = d.transport().written.len();
        let mut st = DeviceState::static_color(Rgb::new(255, 128, 0));
        st.brightness = 100;
        d.apply(&st).unwrap();
        let w = &d.transport().written[n0..];
        // Mainboard: effect on channel 0 + colour packet for 8 LEDs at start 0.
        assert_eq!(w[0][..6], [0xEC, 0x35, 0x00, 0x00, 0x00, 0x01]);
        assert_eq!(w[1][..5], [0xEC, 0x36, 0x00, 0xFF, 0x00]); // mask 0x00FF
        assert_eq!(w[1][5..8], [255, 128, 0]);
        assert_eq!(w[1][5 + 7 * 3..5 + 8 * 3], [255, 128, 0]);
        // Header 1 (12 LEDs): effect channel 1, one effect LED at start 8.
        assert_eq!(w[2][..6], [0xEC, 0x35, 0x01, 0x00, 0x00, 0x01]);
        assert_eq!(w[3][..5], [0xEC, 0x36, 0x01, 0x00, 0x00]); // mask 0x0100
        assert_eq!(w[3][5 + 8 * 3..5 + 9 * 3], [255, 128, 0]);
        // Header 2 has 0 LEDs: nothing sent.
        assert_eq!(w.len(), 4);
    }

    #[test]
    fn direct_mode_splits_packets_and_sets_apply_bit() {
        let mut d = device(&[45, 0]);
        let n0 = d.transport().written.len();
        let colors: Vec<Rgb> = (0..45)
            .map(|i| Rgb::new(i as u8, 0, 255 - i as u8))
            .collect();
        let st = DeviceState {
            mode: "direct".into(),
            colors: vec![],
            zones: vec![ZoneState {
                id: "argb1".into(),
                colors: colors.clone(),
            }],
            brightness: 100,
            ..Default::default()
        };
        d.apply(&st).unwrap();
        let w = &d.transport().written[n0..];
        // Effect packets first (mainboard, header 1), no colour packets in direct.
        assert_eq!(w[0][..6], [0xEC, 0x35, 0x00, 0x00, 0x00, 0xFF]);
        assert_eq!(w[1][..6], [0xEC, 0x35, 0x01, 0x00, 0x00, 0xFF]);
        // Mainboard direct: channel 4, 8 LEDs (white default), apply bit set.
        assert_eq!(w[2][..5], [0xEC, 0x40, 0x84, 0x00, 0x08]);
        // Header 1 direct: 45 LEDs in 20 + 20 + 5.
        assert_eq!(w[3][..5], [0xEC, 0x40, 0x00, 0x00, 0x14]);
        assert_eq!(w[3][5..8], [0, 0, 255]);
        assert_eq!(w[4][..5], [0xEC, 0x40, 0x00, 0x14, 0x14]);
        assert_eq!(w[5][..5], [0xEC, 0x40, 0x80, 0x28, 0x05]);
        assert_eq!(w[5][5 + 4 * 3..5 + 5 * 3], [44, 0, 211]);
        assert_eq!(w.len(), 6);
    }

    #[test]
    fn brightness_scales_colors_and_save_commits() {
        let mut d = device(&[]);
        let n0 = d.transport().written.len();
        let mut st = DeviceState::static_color(Rgb::new(200, 100, 50));
        st.brightness = 50;
        d.apply(&st).unwrap();
        let w = &d.transport().written[n0..];
        assert_eq!(w[1][5..8], [100, 50, 25]);
        d.save().unwrap();
        let last = d.transport().written.last().unwrap();
        assert_eq!(last[..3], [0xEC, 0x3F, 0x55]);
    }

    #[test]
    fn resize_header() {
        let mut d = device(&[0, 0]);
        d.set_zone_leds("argb2", 500).unwrap();
        assert_eq!(d.info().zones[2].leds, 120);
        assert!(d.set_zone_leds("mainboard", 3).is_err());
    }

    #[test]
    fn rejects_bad_replies() {
        let hid = MockHid::with_replies(vec![vec![0u8; 65]]);
        assert!(AuraUsb::new(hid, "x".into(), "x".into(), "x".into(), &[]).is_err());
    }
}
