# RGBeast: RGB lighting control for Fedora

Plan written 2026-09-28. Target: Fedora 44 Workstation, GNOME on Wayland, delivered as an RPM.
Stack: Rust, GTK 4, libadwaita, Blueprint, Meson wrapping Cargo, RPM spec, CI on Fedora
containers. Design: Apple's Human Interface Guidelines applied through Adwaita (`DESIGN.md`).

**Status (2026-09-28, evening):** designed and built in a cloud container without the hardware.
Everything that can be verified without hardware is verified: unit tests on every protocol packet,
a simulated device set that drives the full UI, clippy, formatting, Meson install, RPM build.
Hardware bring-up happens on the owner's machine; see `docs/TESTING.md`.

---

## 0. Ground truth

### 0.1 The hardware to control

| Component | How its lighting is reached on Linux | Protocol notes |
|---|---|---|
| **ASUS TUF Gaming motherboard** (AM5 generation) | USB HID controller inside the board, vendor `0x0B05`, product `0x19AF` (also `0x18F3`, `0x1939`, `0x1AA6`, `0x1BED` on other boards), usage page `0xFF72`, usage `0x00A1`. Device node `/dev/hidraw*`. | 65-byte reports, first byte `0xEC`. Config table reports the on-board LED count and the number of addressable headers. See `PROTOCOLS.md` § Aura USB. |
| **Arctic P12 PWM PST A-RGB fans** | They have no controller of their own. They are WS2812-class strips on the board's **Addressable Gen 2** headers, daisy-chained through Arctic's PST connector. | Driven through the motherboard controller's addressable channels. Each fan is 12 LEDs; the chain length is a user setting (Preferences › Headers). |
| **Kingston Fury Beast/Renegade DDR5 RGB** | SMBus on the chipset (`i2c-piix4` on AMD), one controller per stick at `0x60 + slot index`. | Register protocol with a begin/end transaction byte, 12 LEDs per stick, 20 hardware effects. See `PROTOCOLS.md` § Fury DDR5. |
| **ASUS TUF Radeon RX 9070** (Navi 48) | ENE lighting MCU at I2C address `0x67` on the GPU's own I2C bus. Kernel 6.15+ exposes it as `AMDGPU DM i2c OEM bus` (older GPUs: `AMDGPU i2c bit bus OEM 0x97`). Fedora 44 ships 6.18. | 16-bit register protocol shared by ASUS GPUs and ENE-based DRAM. See `PROTOCOLS.md` § ENE SMBus. |
| Anything else with the same chips | ENE-based DRAM (e.g. G.Skill Trident Z, Geil) at `0x70..0x77`, ASUS Aura SMBus motherboards at `0x40/0x4E/0x4F`, other ASUS/ENE GPUs at `0x67`. | Detected by the same ENE driver. |

### 0.2 Kernel and system requirements (Fedora 44)

| Need | How RGBeast handles it |
|---|---|
| `/dev/i2c-*` nodes exist | `modules-load.d/rgbeast.conf` loads `i2c-dev`. `i2c-piix4` (AMD) and `amdgpu` load by themselves. |
| The `spd5118` SPD driver can hold the DIMM SMBus addresses `0x50..0x57` | RGBeast only talks to `0x60..0x67`, so no conflict. If a scan shows `UU` on `0x60+`, `docs/TESTING.md` explains `rmmod spd5118`. |
| Access to device nodes without running the GUI as root | A small system daemon (`rgbeastd`) runs as the unprivileged system user `rgbeast`; udev rules give group `rgbeast` access to exactly the ASUS controller's hidraw node and to `i2c-dev` nodes. The GUI never touches hardware. |
| Authorisation | polkit action `io.github.djshiye.rgbeast.control`, allowed for the active local session without a password, denied otherwise. |
| Lighting after boot and after sleep | `rgbeastd` re-applies the last state at start and on `PrepareForSleep(false)` from logind. |

### 0.3 Toolchain

| Component | Version on Fedora 44 | Version in the build container | Notes |
|---|---|---|---|
| GTK | 4.22 | 4.14 | Code uses the `v4_14` feature gate so it compiles on both. Bump to `v4_22` on Fedora is a one-line change. |
| libadwaita | 1.9 | 1.5 | `v1_5` gate: `AdwDialog`, `AdwNavigationSplitView`, `AdwBreakpoint`, `AdwSpinRow`, `AdwSwitchRow`, `AdwToolbarView`. `AdwButtonRow` (1.6) and `AdwShortcutsDialog` (1.8) are avoided. |
| Rust | 1.98 | 1.94 | Edition 2024. |
| gtk4-rs / libadwaita-rs | 0.11 / 0.9 | same | Fedora packages both. |
| zbus | 5 | same | D-Bus for the daemon and the client; `zbus_polkit` for authorisation. |
| hidapi (crate) | 2.6, `linux-native` backend | same | Pure Rust hidraw access, no libhidapi. Needs libudev. |
| i2cdev | 0.6 | same | Pure Rust `/dev/i2c-*` SMBus calls. |
| tokio | 1 | same | Daemon runtime. The GUI stays on the GLib main loop. |
| blueprint-compiler | 0.20 | 0.12 | UI files use syntax common to both. |

## 1. Architecture

Three crates in one Cargo workspace, one Meson project, one RPM.

```
rgbeast/
  Cargo.toml                workspace
  crates/rgbeast-core/         protocols, device model, transports (no GTK, no D-Bus)
  crates/rgbeastd/             system daemon: D-Bus service, polkit, state restore, sleep/resume
  crates/rgbeast/              GTK 4 + libadwaita app: D-Bus client only
  data/                     desktop, metainfo, gschema, icons, CSS, Blueprint UI,
                            udev rule, sysusers, modules-load, polkit policy,
                            D-Bus system service + policy, systemd unit
  build-aux/                cargo.sh, rgbeast.spec
  docs/                     PLAN.md, PROTOCOLS.md, TESTING.md, DESIGN.md
```

### 1.1 Why a daemon

Controlling RGB means writing to raw USB HID and raw I2C. Raw I2C on the DIMM bus is the same
bus as the SPD EEPROMs: a careless write there can brick a memory stick, and the GPU bus reaches
the card's own EEPROMs. Giving the desktop user's session direct access to `/dev/i2c-*` (what a
plain udev `uaccess` rule would do) is therefore too broad. Instead:

- `rgbeastd` is the only process that opens device nodes. It runs as system user `rgbeast`, with a
  systemd sandbox (`NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`,
  `RestrictAddressFamilies=AF_UNIX`, `SystemCallFilter=@system-service`, `CapabilityBoundingSet=`,
  `DeviceAllow=` limited to hidraw and i2c-dev).
- It offers a narrow, typed D-Bus API on the system bus: list devices, read state, apply a state.
  The API only knows about colours, modes, brightness, speed and direction. There is no
  "write register" call, so a compromised session can at most change the lights.
- Every write goes through polkit. The shipped policy allows the active local session without a
  prompt (`allow_active=yes`) and denies remote or inactive sessions.
- The daemon validates every value against the device's capabilities before touching hardware.

### 1.2 rgbeast-core

- `color.rs`: `Rgb`, `Hsv`, conversions, gamma-aware brightness scaling, hex parsing.
- `model.rs`: `DeviceInfo` (id, name, vendor, kind, location, zones, modes), `Zone` (name, LED
  count, whether the count is user-configurable), `ModeInfo` (id, name, flags: per-LED colours,
  mode colours min/max, speed range, brightness, direction), `DeviceState` (mode, colours,
  brightness, speed, direction, per-zone colours).
- `transport/`: `HidTransport` and `SmbusTransport` traits. Real implementations use `hidapi`
  (hidraw) and `i2cdev`. `mock.rs` records every write so unit tests assert the exact bytes.
- `drivers/aura_usb.rs`, `drivers/ene.rs`, `drivers/fury.rs`: the three protocols, each a
  `Driver` implementation: `probe`, `info`, `apply`, `read_state`.
- `discover.rs`: sysfs walk. HID: enumerate hidraw devices by vendor/product/usage. I2C: read
  `/sys/bus/i2c/devices/i2c-N/name`, resolve the PCI parent (`vendor`, `device`,
  `subsystem_vendor`) so the GPU bus is only probed when the PCI IDs match, and DIMM addresses are
  only probed on the chipset SMBus.
- `sim.rs`: simulated devices (a TUF board with two addressable headers, two Fury sticks, a TUF
  RX 9070) used by `rgbeastd --simulate`, by tests and by UI development.

### 1.3 rgbeastd

- `zbus` service `io.github.djshiye.RGBeast1` at `/io/github/djshiye/RGBeast1`, interface
  `io.github.djshiye.RGBeast1.Manager`:
  - `ListDevices() -> a(DeviceInfo)`; `GetState(id) -> DeviceState`; `SetState(id, DeviceState)`;
    `SetAll(DeviceState)`; `Rescan()`; `Version` property; signal `DevicesChanged`, `StateChanged(id, DeviceState)`.
  - Structures are transported as D-Bus structs generated from `serde`/`zvariant` derives.
- State store: `/var/lib/rgbeast/state.json` (`StateDirectory=rgbeast`), written on each change.
- Restore: at start, after `Rescan`, and on logind `PrepareForSleep(false)`.
- Command line: `rgbeastd` (system bus), `rgbeastd --session` (session bus, for development),
  `rgbeastd --simulate` (fake devices), `rgbeastd --scan` (print what would be detected, then exit).
- The hardware is driven from one blocking thread (`spawn_blocking`) per request; SMBus writes
  with their required 10 ms delays never block the D-Bus loop.

### 1.4 rgbeast (the app)

- `AdwApplicationWindow` with `AdwNavigationSplitView`: sidebar lists devices as cards (kind icon
  in a tinted rounded square, name, a live colour strip); the content page is the device editor.
- The editor, top to bottom: **LED preview** (custom widget drawing every LED as a glowing dot on
  a dark card, animated for effects, respecting reduce-motion); **mode chips**; **colour wheel**
  (custom widget: hue ring, saturation/value square, drag to pick) with hex entry and a brightness
  slider; **effect controls** (speed, direction, extra colours) that only appear when the mode
  supports them; **zones** for multi-zone devices.
- **All Devices** page: one colour and one mode for everything, with the closest capability match
  per device.
- **Scenes**: saved states for all devices, shown as cards with gradient thumbnails. Apply, save,
  rename, delete. "Lights Off" is a built-in scene.
- Preferences: addressable header LED counts (with an Arctic fan preset), restore on wake, start
  lights at login (daemon behaviour), and which devices are shown.
- Status pages for: daemon not reachable, no devices found (with the kernel-module checklist),
  authorisation denied.

## 2. Design system: Apple HIG applied to a GNOME app

`DESIGN.md` maps Apple's eight design principles and the macOS best practices to concrete rules.
The general rules (systemwide appearance, no custom palette, semantic Adwaita tokens only, system
fonts, progressive disclosure, no Save buttons, confirm destructive actions, brief purposeful
motion, keyboard-first, accessible names) apply throughout. What is specific to RGBeast:

| Principle | Rule for RGBeast |
|---|---|
| Colour is the content, not the chrome | The window stays neutral Adwaita. The only saturated colour on screen is the light the user chose: the preview, the swatches, the wheel. The accent colour is used only for selection rings and focus. |
| Show the result before the control | The preview sits above the controls and updates before the hardware confirms, then settles to the confirmed state. |
| Direct manipulation | Drag on the wheel, drag on the brightness slider, click a LED in the preview to select it for per-LED painting. |
| Progressive disclosure | Speed, direction and multi-colour lists appear only when the active mode uses them. Per-LED painting is behind a "Paint" toggle. |
| Feedback | Applying shows a subtle "Applied" state on the header, not a toast, because it happens constantly. Failures are toasts with the reason and a retry. |
| Motion | Preview animation is the effect itself (rainbow moves, breathing breathes) at 30 fps max, paused when the window is hidden, off with reduce-motion. Everything else is Adwaita stock. |
| Materials | Cards with a 1 px hairline ring and soft shadow, 14 px radius. The preview card is the one dark surface in light mode, because lights are read against dark. |

## 3. Security checklist

- GUI runs unprivileged and never opens a device node.
- Daemon: dedicated user, no capabilities, read-only filesystem except its state directory,
  no network, seccomp filter, device allow-list.
- udev rules match on exact vendor/product IDs; i2c-dev nodes go to group `rgbeast`, not to the user.
- polkit: `allow_active=yes`, `allow_inactive=no`, `allow_any=no`.
- D-Bus policy: only user `rgbeast` may own the name; anyone may call it (polkit decides).
- All D-Bus inputs are validated against the device's capabilities; LED counts are capped at
  the controller's maximum (120 per addressable header on Aura USB).
- No `unsafe` in the workspace except what the GTK bindings need (`set_var` before threads exist).
- `cargo deny` style licence check in CI; dependencies pinned in `Cargo.lock`.

## 4. Phases

| Phase | Content | Status |
|---|---|---|
| 0 | Research protocols, verify kernel paths, choose crates | done |
| 1 | Workspace scaffolding, Meson, data files, packaging skeleton | done |
| 2 | rgbeast-core: colour, model, transports, three drivers, mocks, unit tests | done |
| 3 | rgbeastd: D-Bus API, polkit, state store, resume, simulate | done |
| 4 | rgbeast app: split view, preview, wheel, modes, scenes, preferences, status pages | done |
| 5 | Verification in the container: fmt, clippy, tests, Meson, RPM, screenshots | done |
| 6 | Hardware bring-up on the owner's machine (`docs/TESTING.md`) | tomorrow |
| 7 | Polish from real-hardware findings, COPR publishing, translations | later |

## 5. Out of scope for 1.0

- Software-rendered effects streamed to devices at high frame rates (SMBus is too slow for the
  RAM and GPU; the Aura USB headers could do it and this is a 1.1 candidate).
- Devices that need other protocols (Corsair, NZXT, Razer, keyboards). The driver trait makes
  adding them straightforward.
- A status icon.
