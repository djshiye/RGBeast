//! Find the lighting controllers on this machine. Only buses whose type is
//! known are probed, at the addresses the protocols document, so discovery
//! never touches monitors (DDC/CI), SPD EEPROMs or unknown chips.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Driver,
    drivers::{aura_usb, ene, fury},
    model::DeviceKind,
    transport::{hid::HidrawTransport, smbus::LinuxSmbus},
};

/// Persisted per-device settings that discovery needs up front.
#[derive(Clone, Debug, Default)]
pub struct DiscoveryConfig {
    /// LED counts per addressable header for each Aura USB controller id.
    pub header_leds: Vec<(String, Vec<u32>)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PciIds {
    pub vendor: u16,
    pub device: u16,
    pub subsystem_vendor: u16,
    pub subsystem_device: u16,
    pub class: u32,
}

#[derive(Clone, Debug)]
pub struct I2cBus {
    pub number: u32,
    pub name: String,
    pub pci: Option<PciIds>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusRole {
    /// Chipset SMBus: DIMMs and Aura SMBus boards live here.
    Chipset,
    /// A graphics card's own I2C bus that may carry a lighting MCU.
    Gpu,
    /// Anything else: never probed.
    Other,
}

const AMD_VENDOR: u16 = 0x1002;
const NVIDIA_VENDOR: u16 = 0x10DE;
const ASUS_VENDOR: u16 = 0x1043;

/// Classify a bus from its adapter name and PCI parent.
pub fn bus_role(bus: &I2cBus) -> BusRole {
    let n = bus.name.as_str();
    if n.starts_with("SMBus PIIX4 adapter") || n.starts_with("SMBus I801 adapter") {
        return BusRole::Chipset;
    }
    if n.starts_with("AMDGPU DM i2c OEM bus") || n.starts_with("AMDGPU i2c bit bus OEM 0x97") {
        return BusRole::Gpu;
    }
    if let Some(p) = &bus.pci
        && (p.class >> 16) == 0x03
        && (p.vendor == AMD_VENDOR || p.vendor == NVIDIA_VENDOR)
        && p.subsystem_vendor == ASUS_VENDOR
        && !n.starts_with("AMDGPU DM i2c hw bus")
        && !n.starts_with("AMDGPU DM aux hw bus")
    {
        return BusRole::Gpu;
    }
    BusRole::Other
}

fn read_hex(path: &Path) -> Option<u32> {
    let s = fs::read_to_string(path).ok()?;
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// Walk up from the adapter's device link until a PCI device appears.
fn pci_parent(adapter: &Path) -> Option<PciIds> {
    let mut p = adapter.join("device").canonicalize().ok()?;
    for _ in 0..8 {
        if p.join("vendor").exists() && p.join("device").exists() {
            return Some(PciIds {
                vendor: read_hex(&p.join("vendor"))? as u16,
                device: read_hex(&p.join("device"))? as u16,
                subsystem_vendor: read_hex(&p.join("subsystem_vendor")).unwrap_or(0) as u16,
                subsystem_device: read_hex(&p.join("subsystem_device")).unwrap_or(0) as u16,
                class: read_hex(&p.join("class")).unwrap_or(0),
            });
        }
        p = p.parent()?.to_path_buf();
    }
    None
}

pub fn i2c_buses() -> Vec<I2cBus> {
    i2c_buses_in(Path::new("/sys/bus/i2c/devices"))
}

pub fn i2c_buses_in(root: &Path) -> Vec<I2cBus> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for e in entries.flatten() {
        let fname = e.file_name();
        let Some(num) = fname
            .to_str()
            .and_then(|s| s.strip_prefix("i2c-"))
            .and_then(|s| s.parse().ok())
        else {
            continue;
        };
        let name = fs::read_to_string(e.path().join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();
        out.push(I2cBus {
            number: num,
            name,
            pci: pci_parent(&e.path()),
        });
    }
    out.sort_by_key(|b| b.number);
    out
}

/// Motherboard name from DMI, e.g. "ASUS TUF GAMING B650-PLUS WIFI".
pub fn board_name() -> String {
    let vendor = fs::read_to_string("/sys/class/dmi/id/board_vendor").unwrap_or_default();
    let name = fs::read_to_string("/sys/class/dmi/id/board_name").unwrap_or_default();
    let vendor = vendor.trim();
    let vendor = if vendor.eq_ignore_ascii_case("ASUSTeK COMPUTER INC.") {
        "ASUS"
    } else {
        vendor
    };
    let s = format!("{vendor} {}", name.trim()).trim().to_string();
    if s.is_empty() {
        "ASUS Motherboard".into()
    } else {
        s
    }
}

fn gpu_name(p: &PciIds) -> String {
    let family = match (p.vendor, p.device) {
        (AMD_VENDOR, 0x7550) => "Radeon RX 9070",
        (AMD_VENDOR, 0x7551) => "Radeon RX 9070 XT",
        (AMD_VENDOR, 0x7590) => "Radeon RX 9060 XT",
        (AMD_VENDOR, 0x744C) => "Radeon RX 7900 XTX",
        (AMD_VENDOR, 0x747E) => "Radeon RX 7800 XT",
        (AMD_VENDOR, 0x7480) => "Radeon RX 7600",
        (AMD_VENDOR, 0x73BF) => "Radeon RX 6800 XT",
        (AMD_VENDOR, 0x73DF) => "Radeon RX 6700 XT",
        (AMD_VENDOR, _) => "Radeon graphics card",
        (NVIDIA_VENDOR, _) => "GeForce graphics card",
        _ => "graphics card",
    };
    let brand = if p.subsystem_vendor == ASUS_VENDOR {
        "ASUS "
    } else {
        ""
    };
    format!("{brand}{family}")
}

pub struct Discovered {
    pub devices: Vec<Box<dyn Driver>>,
    /// Human-readable notes about what was and was not found.
    pub log: Vec<String>,
}

pub fn discover(config: &DiscoveryConfig) -> Discovered {
    let mut devices: Vec<Box<dyn Driver>> = Vec::new();
    let mut log = Vec::new();

    // USB HID controllers.
    match hidapi::HidApi::new() {
        Ok(api) => {
            let mut seen = std::collections::HashSet::new();
            for d in api.device_list() {
                if d.vendor_id() != aura_usb::VENDOR_ID
                    || !aura_usb::PRODUCT_IDS.contains(&d.product_id())
                {
                    continue;
                }
                if d.product_id() == 0x19AF
                    && (d.usage_page() != aura_usb::USAGE_PAGE || d.usage() != aura_usb::USAGE)
                {
                    continue;
                }
                if !seen.insert(d.product_id()) {
                    continue;
                }
                let path = d.path().to_string_lossy().to_string();
                let id = format!("aura-usb:{:04x}:{:04x}", d.vendor_id(), d.product_id());
                let header_leds = config
                    .header_leds
                    .iter()
                    .find(|(k, _)| *k == id)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                match HidrawTransport::open(&api, &path) {
                    Ok(t) => match aura_usb::AuraUsb::new(
                        t,
                        id.clone(),
                        board_name(),
                        format!("USB {:04x}:{:04x}", d.vendor_id(), d.product_id()),
                        &header_leds,
                    ) {
                        Ok(dev) => {
                            log.push(format!("Aura USB controller {id} ({})", dev.info().version));
                            devices.push(Box::new(dev));
                        }
                        Err(e) => log.push(format!("Aura USB controller {id}: {e}")),
                    },
                    Err(e) => log.push(format!("cannot open {path}: {e}")),
                }
            }
        }
        Err(e) => log.push(format!("HID enumeration failed: {e}")),
    }

    // I2C buses.
    let buses = i2c_buses();
    if buses.is_empty() {
        log.push("no I2C buses visible (is i2c-dev loaded?)".into());
    }
    for bus in &buses {
        let role = bus_role(bus);
        match role {
            BusRole::Other => continue,
            BusRole::Gpu => {
                let mut smbus = match LinuxSmbus::open(bus.number) {
                    Ok(s) => s,
                    Err(e) => {
                        log.push(format!("i2c-{}: {e}", bus.number));
                        continue;
                    }
                };
                if !ene::probe(&mut smbus, ene::GPU_ADDRESS) {
                    log.push(format!(
                        "i2c-{} ({}): no ENE controller at 0x67",
                        bus.number, bus.name
                    ));
                    continue;
                }
                let name = bus
                    .pci
                    .as_ref()
                    .map(gpu_name)
                    .unwrap_or_else(|| "ASUS graphics card".into());
                let id = format!("ene:i2c-{}:0x67", bus.number);
                match ene::Ene::new(
                    smbus,
                    ene::GPU_ADDRESS,
                    id.clone(),
                    name,
                    DeviceKind::Gpu,
                    format!("I2C bus {} ({}), address 0x67", bus.number, bus.name),
                ) {
                    Ok(d) => {
                        log.push(format!(
                            "ENE GPU controller on i2c-{} ({})",
                            bus.number,
                            d.info().version
                        ));
                        devices.push(Box::new(d));
                    }
                    Err(e) => log.push(format!("i2c-{}: {e}", bus.number)),
                }
            }
            BusRole::Chipset => {
                // Kingston Fury DDR5, then DDR4, then ENE DRAM and Aura SMBus boards.
                for (base, label) in [
                    (fury::BASE_ADDR_DDR5, "DDR5"),
                    (fury::BASE_ADDR_DDR4, "DDR4"),
                ] {
                    let mut smbus = match LinuxSmbus::open(bus.number) {
                        Ok(s) => s,
                        Err(e) => {
                            log.push(format!("i2c-{}: {e}", bus.number));
                            break;
                        }
                    };
                    let mut slots = Vec::new();
                    let mut model = None;
                    for slot in 0u8..8 {
                        if let Some(m) = fury::probe(&mut smbus, base + slot) {
                            slots.push(slot);
                            model.get_or_insert(m);
                        }
                    }
                    if let Some(m) = model {
                        let id = format!("fury:i2c-{}:{label}", bus.number);
                        let addrs: Vec<String> = slots
                            .iter()
                            .map(|s| format!("0x{:02X}", base + s))
                            .collect();
                        match fury::Fury::new(
                            smbus,
                            base,
                            slots,
                            m,
                            id,
                            format!(
                                "I2C bus {} ({}), addresses {}",
                                bus.number,
                                bus.name,
                                addrs.join(", ")
                            ),
                        ) {
                            Ok(d) => {
                                log.push(format!("{} on i2c-{}", d.info().name, bus.number));
                                devices.push(Box::new(d));
                            }
                            Err(e) => log.push(format!("i2c-{}: {e}", bus.number)),
                        }
                    }
                }
                for (addrs, kind) in [
                    (ene::DRAM_ADDRESSES, DeviceKind::Dram),
                    (ene::MOBO_ADDRESSES, DeviceKind::Motherboard),
                ] {
                    for addr in addrs {
                        let mut smbus = match LinuxSmbus::open(bus.number) {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        if !ene::probe(&mut smbus, *addr) {
                            continue;
                        }
                        let id = format!("ene:i2c-{}:0x{addr:02X}", bus.number);
                        let name = if kind == DeviceKind::Dram {
                            "ENE RGB memory".to_string()
                        } else {
                            board_name()
                        };
                        match ene::Ene::new(
                            smbus,
                            *addr,
                            id,
                            name,
                            kind,
                            format!(
                                "I2C bus {} ({}), address 0x{addr:02X}",
                                bus.number, bus.name
                            ),
                        ) {
                            Ok(d) => {
                                log.push(format!(
                                    "ENE controller at 0x{addr:02X} on i2c-{}",
                                    bus.number
                                ));
                                devices.push(Box::new(d));
                            }
                            Err(e) => log.push(format!("i2c-{} 0x{addr:02X}: {e}", bus.number)),
                        }
                    }
                }
            }
        }
    }
    Discovered { devices, log }
}

/// Where the state file lives when running as the system daemon.
pub fn default_state_dir() -> PathBuf {
    PathBuf::from("/var/lib/rgbeast")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus(name: &str, pci: Option<PciIds>) -> I2cBus {
        I2cBus {
            number: 0,
            name: name.into(),
            pci,
        }
    }

    #[test]
    fn classifies_buses() {
        assert_eq!(
            bus_role(&bus("SMBus PIIX4 adapter port 0 at 0b00", None)),
            BusRole::Chipset
        );
        assert_eq!(
            bus_role(&bus("SMBus I801 adapter at efa0", None)),
            BusRole::Chipset
        );
        assert_eq!(bus_role(&bus("AMDGPU DM i2c OEM bus", None)), BusRole::Gpu);
        assert_eq!(
            bus_role(&bus("AMDGPU DM i2c hw bus 0", None)),
            BusRole::Other
        );
        assert_eq!(
            bus_role(&bus("AMDGPU DM aux hw bus 1", None)),
            BusRole::Other
        );
        let asus_gpu = PciIds {
            vendor: 0x1002,
            device: 0x7550,
            subsystem_vendor: 0x1043,
            subsystem_device: 0x0000,
            class: 0x030000,
        };
        assert_eq!(
            bus_role(&bus(
                "NVIDIA i2c adapter 3 at 1:00.0",
                Some(PciIds {
                    vendor: 0x10DE,
                    ..asus_gpu.clone()
                })
            )),
            BusRole::Gpu
        );
        assert_eq!(
            bus_role(&bus("AMDGPU i2c bit bus OEM 0x97", Some(asus_gpu.clone()))),
            BusRole::Gpu
        );
        let other_gpu = PciIds {
            subsystem_vendor: 0x1DA2,
            ..asus_gpu
        };
        assert_eq!(
            bus_role(&bus("NVIDIA i2c adapter 3 at 1:00.0", Some(other_gpu))),
            BusRole::Other
        );
        assert_eq!(bus_role(&bus("i915 gmbus dpb", None)), BusRole::Other);
    }

    #[test]
    fn reads_sysfs_layout() {
        let dir = std::env::temp_dir().join(format!("rgbeast-sysfs-{}", std::process::id()));
        let adapter = dir.join("i2c-7");
        fs::create_dir_all(&adapter).unwrap();
        fs::write(adapter.join("name"), "AMDGPU DM i2c OEM bus\n").unwrap();
        let pci = dir.join("pci0000:03");
        fs::create_dir_all(pci.join("child")).unwrap();
        fs::write(pci.join("vendor"), "0x1002\n").unwrap();
        fs::write(pci.join("device"), "0x7550\n").unwrap();
        fs::write(pci.join("subsystem_vendor"), "0x1043\n").unwrap();
        fs::write(pci.join("subsystem_device"), "0x0000\n").unwrap();
        fs::write(pci.join("class"), "0x030000\n").unwrap();
        std::os::unix::fs::symlink(pci.join("child"), adapter.join("device")).unwrap();
        let buses = i2c_buses_in(&dir);
        assert_eq!(buses.len(), 1);
        assert_eq!(buses[0].number, 7);
        assert_eq!(buses[0].pci.as_ref().unwrap().device, 0x7550);
        assert_eq!(
            gpu_name(buses[0].pci.as_ref().unwrap()),
            "ASUS Radeon RX 9070"
        );
        fs::remove_dir_all(&dir).ok();
    }
}
