# Changelog

## 1.0.1 (2026-09-29)

First run on the target machine (Fedora 44, GNOME 50, libadwaita 1.9). Reviewed against Apple's
Human Interface Guidelines (design principles, materials) and reworked where the app fell short.

Design
- The sidebar is the platform's own navigation material: flat rows on `sidebar-bg-color` with a
  rounded selection, no card shadows. Device rows keep the accent icon tile and the colour strip.
- The editor's sections are libadwaita cards (`.card`), so light, dark, high contrast and the
  system accent all carry through; all colours come from `var(--…)` tokens and `color-mix()`.
- Effect chips wrap in an `AdwWrapBox`; direction is an `AdwToggleGroup`; the "Applied" mark
  fades with CSS transitions; a Keyboard Shortcuts dialog (Ctrl+?) lists every shortcut.
- The colour wheel's focus ring uses the system accent and only appears for keyboard focus; the
  wheel reports its colour to assistive technology.
- Status line reads "3 devices · simulated" instead of a daemon version string.
- The header bar rule that leaked into every dialog is gone; the stylesheet only targets the
  app's own classes.

Daemon and discovery
- "Restore After Sleep" is honoured (it was read but never applied).
- Discovery probes only PIIX4 port 0 and only DIMM slots that have an SPD device, so empty
  slots and the chipset's auxiliary ports are never written to.
- The GPU's PCI identity is read from the real sysfs location, so the card is named
  ("ASUS Radeon RX 9070") and the vendor check works.
- Devices with no stored state are left showing their own power-on effect instead of being set to
  white on every start.
- StateChanged/DevicesChanged are emitted once per change; SIGTERM shuts down cleanly; a device
  missing at boot is looked for once more after 10 s; a corrupt state.json is kept as `.bad`.
- Per-LED colours are bounded to the device's zones in every mode; the Aura effect-colour mask
  is guarded against overflow.

Packaging
- README documents `--prefix=/usr` and the post-install steps for a Meson install; the unit no
  longer lists 21 redundant `DeviceAllow` lines; the CI smoke test can actually fail.

First install on the machine
- The udev rule now sets the nodes' owning-group ACL entry with `setfacl`. When another package
  (openrgb-udev-rules) has already put a console-user ACL on the same nodes, `MODE=0660` only
  changes the ACL mask and the group entry stays at `---`, so the daemon was refused although
  `ls` showed `rw` for the group. The RPM's post-install also waits for udev to settle and restarts
  a running service on upgrade; the daemon scans again 10 s and 30 s after start when nothing was
  found or a node could not be opened, and SMBus permission problems are reported by name.
- The graphics card in the target machine is a Sapphire RX 9070, not an ASUS one; the Detection
  Log now says which vendor a card is from when no ENE controller answers.
- polkit: the action declares the daemon user as its owner; since polkit 124 only root or the
  owner may check authorisation for other identities, so every change was refused.
- Addressable headers get effects and colours whatever their configured length, like Windows
  Dynamic Lighting does (the controller's LampArray reports the board as a single lamp; nothing
  can count the LEDs on a WS2812 chain). The LED count is only needed for painting single lights
  and for the preview; a header of unknown length receives the primary colour in direct mode.
- Fury detection tolerates one odd signature byte and paces its reads: on real Beast DDR5 sticks
  the "R" register reads 0x02 (always on one stick, sometimes on the other) and single reads can
  return 0xFFFF, which left one of two sticks undetected.

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
