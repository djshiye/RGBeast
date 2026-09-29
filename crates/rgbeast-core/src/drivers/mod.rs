//! Protocol implementations. See `docs/PROTOCOLS.md` for the wire formats.

pub mod aura_usb;
pub mod ene;
pub mod fury;
pub mod sapphire;

/// Map the universal 0..=100 speed onto a hardware range whose ends may be
/// in either order (`slowest` is what 0 maps to, `fastest` what 100 maps to).
pub(crate) fn map_speed(speed: u32, slowest: i32, fastest: i32) -> u8 {
    let s = speed.min(100) as i32;
    let v = slowest + (fastest - slowest) * s / 100;
    v.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::map_speed;

    #[test]
    fn speed_mapping_both_directions() {
        assert_eq!(map_speed(0, 4, 0), 4);
        assert_eq!(map_speed(100, 4, 0), 0);
        assert_eq!(map_speed(50, 4, 0), 2);
        assert_eq!(map_speed(0, 60, 0), 60);
        assert_eq!(map_speed(100, 60, 0), 0);
        assert_eq!(map_speed(0, 10, 1), 10);
        assert_eq!(map_speed(100, 10, 1), 1);
        assert_eq!(map_speed(200, 0, 255), 255);
    }
}
