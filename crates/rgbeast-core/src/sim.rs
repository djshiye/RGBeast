//! Simulated devices: the owner's build, as the daemon would enumerate it.
//! Used by `rgbeastd --simulate`, by the app's tests and for UI development.

use crate::{
    Driver, Result, Rgb,
    drivers::{aura_usb, fury, sapphire},
    model::{DeviceInfo, DeviceKind, DeviceState, ZoneInfo, validate},
};

pub struct SimDevice {
    info: DeviceInfo,
    state: DeviceState,
    saved: bool,
}

impl SimDevice {
    pub fn new(info: DeviceInfo, state: DeviceState) -> Self {
        SimDevice {
            info,
            state,
            saved: false,
        }
    }
    pub fn saved(&self) -> bool {
        self.saved
    }
}

impl Driver for SimDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }
    fn apply(&mut self, state: &DeviceState) -> Result<()> {
        self.state = validate(&self.info, state)?;
        Ok(())
    }
    fn read_state(&mut self) -> Result<Option<DeviceState>> {
        Ok(Some(self.state.clone()))
    }
    fn save(&mut self) -> Result<()> {
        if !self.info.can_save {
            return Err(crate::Error::Unsupported("cannot save".into()));
        }
        self.saved = true;
        Ok(())
    }
    fn set_zone_leds(&mut self, zone: &str, leds: u32) -> Result<()> {
        let z = self
            .info
            .zones
            .iter_mut()
            .find(|z| z.id == zone)
            .ok_or_else(|| crate::Error::invalid(format!("no zone {zone}")))?;
        if !z.is_sizable() {
            return Err(crate::Error::Unsupported("fixed zone".into()));
        }
        z.leds = leds.clamp(z.leds_min, z.leds_max);
        Ok(())
    }
}

/// The three devices of the target machine.
pub fn devices() -> Vec<Box<dyn Driver>> {
    let board = DeviceInfo {
        id: "sim:aura-usb".into(),
        name: "ASUS TUF Gaming B850-Plus WiFi".into(),
        vendor: "ASUS".into(),
        kind: DeviceKind::Motherboard,
        location: "Simulated USB controller".into(),
        driver: "aura-usb".into(),
        version: "AUMA0-E6K5-0106".into(),
        zones: vec![
            ZoneInfo::fixed("mainboard", "Mainboard", 3),
            ZoneInfo::sizable(
                "argb1",
                "Addressable Header 1",
                36,
                aura_usb::MAX_ADDRESSABLE_LEDS,
            ),
            ZoneInfo::sizable(
                "argb2",
                "Addressable Header 2",
                24,
                aura_usb::MAX_ADDRESSABLE_LEDS,
            ),
            ZoneInfo::sizable(
                "argb3",
                "Addressable Header 3",
                0,
                aura_usb::MAX_ADDRESSABLE_LEDS,
            ),
        ],
        modes: aura_usb::modes(),
        can_save: true,
    };
    let ram = DeviceInfo {
        id: "sim:fury".into(),
        name: "Kingston Fury Beast DDR5 RGB ×2".into(),
        vendor: "Kingston".into(),
        kind: DeviceKind::Dram,
        location: "Simulated SMBus, addresses 0x61, 0x63".into(),
        driver: "fury".into(),
        version: "BeastDdr5".into(),
        zones: vec![
            ZoneInfo::fixed("slot2", "Slot 2", fury::LEDS_DDR5),
            ZoneInfo::fixed("slot4", "Slot 4", fury::LEDS_DDR5),
        ],
        modes: fury::modes(),
        can_save: false,
    };
    let gpu = DeviceInfo {
        id: "sim:sapphire-gpu".into(),
        name: "Sapphire Radeon RX 9070 XT Pure".into(),
        vendor: "Sapphire".into(),
        kind: DeviceKind::Gpu,
        location: "Simulated AMDGPU OEM I2C bus, address 0x28".into(),
        driver: "sapphire".into(),
        version: "Nitro Glow V3".into(),
        zones: vec![ZoneInfo::fixed("logo", "Logo", 1)],
        modes: sapphire::modes(),
        can_save: false,
    };
    let mut rainbow = DeviceState {
        mode: "rainbow".into(),
        speed: 50,
        ..Default::default()
    };
    rainbow.colors.clear();
    vec![
        Box::new(SimDevice::new(
            board,
            DeviceState::static_color(Rgb::new(0x35, 0x84, 0xE4)),
        )),
        Box::new(SimDevice::new(ram, rainbow)),
        Box::new(SimDevice::new(
            gpu,
            DeviceState::static_color(Rgb::new(0xFF, 0x00, 0xFF)),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_devices_accept_their_own_modes() {
        for mut d in devices() {
            let modes: Vec<String> = d.info().modes.iter().map(|m| m.id.clone()).collect();
            for m in modes {
                let st = DeviceState {
                    mode: m.clone(),
                    ..Default::default()
                };
                d.apply(&st)
                    .unwrap_or_else(|e| panic!("{} {m}: {e}", d.info().name));
                assert_eq!(d.read_state().unwrap().unwrap().mode, m);
            }
        }
    }
}
