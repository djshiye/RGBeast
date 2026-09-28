//! hidraw transport through the `hidapi` crate's pure-Rust Linux backend.

use std::ffi::CString;

use crate::{Error, Result};

use super::HidTransport;

pub struct HidrawTransport {
    dev: hidapi::HidDevice,
}

impl HidrawTransport {
    pub fn open(api: &hidapi::HidApi, path: &str) -> Result<Self> {
        let cpath = CString::new(path).map_err(|e| Error::Hid(e.to_string()))?;
        let dev = api
            .open_path(&cpath)
            .map_err(|e| Error::Hid(format!("{path}: {e}")))?;
        Ok(HidrawTransport { dev })
    }
}

impl HidTransport for HidrawTransport {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        let n = self
            .dev
            .write(data)
            .map_err(|e| Error::Hid(e.to_string()))?;
        if n != data.len() {
            return Err(Error::Hid(format!(
                "short write: {n} of {} bytes",
                data.len()
            )));
        }
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize> {
        self.dev
            .read_timeout(buf, timeout_ms.min(i32::MAX as u32) as i32)
            .map_err(|e| Error::Hid(e.to_string()))
    }
}
