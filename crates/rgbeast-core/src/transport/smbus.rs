//! `/dev/i2c-N` transport through the `i2cdev` crate. One file descriptor
//! per (bus, address) pair, opened on first use. `I2C_SLAVE` is used, never
//! `I2C_SLAVE_FORCE`, so an address a kernel driver owns stays untouchable.

use std::{collections::HashMap, path::PathBuf};

use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};

use crate::{Error, Result};

use super::Smbus;

pub struct LinuxSmbus {
    path: PathBuf,
    devs: HashMap<u8, LinuxI2CDevice>,
}

impl LinuxSmbus {
    pub fn open(bus_number: u32) -> Result<Self> {
        let path = PathBuf::from(format!("/dev/i2c-{bus_number}"));
        if !path.exists() {
            return Err(Error::Smbus(format!("{} does not exist", path.display())));
        }
        // Open the node once up front so a permission problem is reported
        // here, by name, instead of making every probe fail silently.
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| Error::Smbus(format!("cannot open {}: {e}", path.display())))?;
        Ok(LinuxSmbus {
            path,
            devs: HashMap::new(),
        })
    }

    fn dev(&mut self, addr: u8) -> Result<&mut LinuxI2CDevice> {
        if !self.devs.contains_key(&addr) {
            let d = LinuxI2CDevice::new(&self.path, addr as u16)
                .map_err(|e| Error::Smbus(format!("{} @0x{addr:02X}: {e}", self.path.display())))?;
            self.devs.insert(addr, d);
        }
        Ok(self.devs.get_mut(&addr).expect("inserted"))
    }
}

fn map<T>(r: std::result::Result<T, i2cdev::linux::LinuxI2CError>) -> Result<T> {
    r.map_err(|e| Error::Smbus(e.to_string()))
}

impl Smbus for LinuxSmbus {
    fn read_byte(&mut self, addr: u8) -> Result<u8> {
        map(self.dev(addr)?.smbus_read_byte())
    }
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
        map(self.dev(addr)?.smbus_read_byte_data(reg))
    }
    fn read_word_data(&mut self, addr: u8, reg: u8) -> Result<u16> {
        map(self.dev(addr)?.smbus_read_word_data(reg))
    }
    fn write_byte_data(&mut self, addr: u8, reg: u8, val: u8) -> Result<()> {
        map(self.dev(addr)?.smbus_write_byte_data(reg, val))
    }
    fn write_word_data(&mut self, addr: u8, reg: u8, val: u16) -> Result<()> {
        map(self.dev(addr)?.smbus_write_word_data(reg, val))
    }
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
        map(self.dev(addr)?.smbus_write_block_data(reg, data))
    }
}
