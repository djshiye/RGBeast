# Changelog

## 1.0.0 (2026-09-28)

First release, built in a cloud environment against simulated devices; hardware bring-up is
tracked in `docs/TESTING.md`.

- Drivers: ASUS Aura USB mainboard controller (on-board LEDs, RGB headers, addressable headers),
  ENE SMBus lighting (ASUS graphics cards incl. TUF RX 9070, ENE DRAM, Aura SMBus boards),
  Kingston Fury Beast/Renegade DDR5 and DDR4.
- `rgbeastd`: sandboxed system daemon, D-Bus API with strict JSON validation, polkit, persistent state,
  restore at boot and after sleep, `--scan` and `--simulate`.
- App: sidebar of devices and scenes, animated LED preview (fans as rings, sticks as bars), colour
  wheel, palette, hex entry, brightness, speed, direction, random colours, multi-colour slots,
  per-LED painting, zones with header LED counts, All Devices page, scenes, preferences,
  power-on storage, status pages for missing service or devices.
- Packaging: RPM spec, systemd unit, udev rules, sysusers, modules-load, polkit policy, D-Bus
  policy and activation, man pages, AppStream metadata, CI on Fedora 44 and Rawhide.
