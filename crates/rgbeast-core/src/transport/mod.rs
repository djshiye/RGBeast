//! Byte-level access to devices. Drivers only see these traits, so they can
//! be tested against [`mock`] and run unchanged on real hardware.

pub mod hid;
pub mod mock;
pub mod smbus;

use crate::Result;

/// A raw HID report channel (one hidraw node).
pub trait HidTransport: Send {
    /// Send one output report. `data[0]` is the report ID (0 when unused).
    fn write(&mut self, data: &[u8]) -> Result<()>;
    /// Read one input report, waiting at most `timeout_ms`. Returns bytes read.
    fn read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize>;
}

/// One SMBus/I2C adapter. The 7-bit slave address is given per call.
pub trait Smbus: Send {
    fn read_byte(&mut self, addr: u8) -> Result<u8>;
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8>;
    fn read_word_data(&mut self, addr: u8, reg: u8) -> Result<u16>;
    fn write_byte_data(&mut self, addr: u8, reg: u8, val: u8) -> Result<()>;
    fn write_word_data(&mut self, addr: u8, reg: u8, val: u16) -> Result<()>;
    /// SMBus block write (count byte on the wire).
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()>;
}
