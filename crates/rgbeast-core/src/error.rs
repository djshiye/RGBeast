use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HID error: {0}")]
    Hid(String),
    #[error("SMBus error: {0}")]
    Smbus(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("no such device: {0}")]
    NoDevice(String),
}

impl Error {
    pub fn invalid(msg: impl fmt::Display) -> Self {
        Error::Invalid(msg.to_string())
    }
    pub fn protocol(msg: impl fmt::Display) -> Self {
        Error::Protocol(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
