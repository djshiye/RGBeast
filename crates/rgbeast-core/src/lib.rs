//! RGBeast core: the device model, the lighting protocols and the transports
//! they run on. This crate knows nothing about D-Bus or GTK, so every
//! protocol can be unit-tested against a recording mock transport.

pub mod color;
pub mod discover;
pub mod drivers;
pub mod error;
pub mod model;
pub mod sim;
pub mod transport;

pub use color::Rgb;
pub use error::{Error, Result};
pub use model::{
    DeviceInfo, DeviceKind, DeviceState, KeyboardOptions, ModeInfo, ZoneInfo, ZoneState,
};

/// A lighting device the daemon can drive.
pub trait Driver: Send {
    /// Static description: zones, modes, identity.
    fn info(&self) -> &DeviceInfo;

    /// Push a complete state to the hardware. The state has already been
    /// validated against `info()` by [`model::validate`].
    fn apply(&mut self, state: &DeviceState) -> Result<()>;

    /// Read the current state back from the hardware, where the protocol
    /// allows it. `None` means "not readable".
    fn read_state(&mut self) -> Result<Option<DeviceState>> {
        Ok(None)
    }

    /// Make the current effect the power-on default, where supported.
    fn save(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "this device cannot store a power-on effect".into(),
        ))
    }

    /// The device keeps its lighting by itself across power cycles, so the
    /// daemon never re-sends the stored state at boot or after sleep.
    fn keeps_state(&self) -> bool {
        false
    }

    /// Keyboard settings carried in the lighting report (Glorious keyboards).
    fn keyboard_options(&self) -> Option<KeyboardOptions> {
        None
    }

    /// Change those settings. `lighting` is the state the device shows, so
    /// the shared report keeps it.
    fn set_keyboard_options(
        &mut self,
        _options: &KeyboardOptions,
        _lighting: &DeviceState,
    ) -> Result<()> {
        Err(Error::Unsupported(
            "this device has no keyboard settings".into(),
        ))
    }

    /// Change the LED count of a user-sizable zone (addressable headers).
    fn set_zone_leds(&mut self, _zone: &str, _leds: u32) -> Result<()> {
        Err(Error::Unsupported(
            "zone size is fixed on this device".into(),
        ))
    }
}
