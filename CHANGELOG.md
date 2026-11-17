# Changelog

## 1.3.0 (2026-11-17)

Look
- A new look, "Spectral": a deep canvas lit by an ambient wash of the colours your lights are
  showing, glass cards with a lit top rim, and the desktop's accent as a short glowing gradient on
  the active effect, the primary button, switches, sliders and the selected device.
- Eyebrow headings for sections and cards, the kind of device above its name, and monospaced
  readouts for the effect, light count, brightness and speed.
- The preview stage has a faint dot grid and corner marks; sheets float in a soft accent glow, and
  the status line at the bottom of the sidebar has a slow heartbeat while the service is there.
- The dimmed backdrop behind sheets no longer shows as a grey box past the window's rounded corners.

Using the app
- Lights Off, applying a scene and deleting a scene show a message with Undo (also Ctrl+Z);
  deleting a scene no longer asks first.
- Messages: errors carry a warning mark and stay 7 seconds, hovering holds a message, and a
  repeated message refreshes the one on screen instead of stacking copies.
- Scan for Devices says what it found, and cannot be started twice.
- When the service is not running: Copy Command next to Try Again, and the page says the app
  reconnects by itself. With no devices found: Open Detection Log.
- Lights Off and scenes cannot be applied while the service is away.
- Narrow window: the sidebar keeps the close button.
- Sidebar: Up and Down move between rows, right-click, the Menu key or Shift+F10 open a scene's
  menu, full names show on hover, and a hint for saving the first scene.
- Ctrl+S saves the lighting as a scene (also in the main menu).
- Hex colour field: accepts #RGB, explains what is wrong, and Escape puts the colour back.
- The save sheet says Replace when the name is already taken.

Fixes
- All Devices: a slider drag no longer queues every step, so the lights stop changing as soon as
  you let go.
- Devices that are still turned off because they are untested can no longer be written to through
  "Save to device" or "LED count".
- Change notifications no longer stop for good after the service falls behind once.
- Settings are synced to disk before they replace the old file, so a power cut can no longer lose
  them.
- ENE first-generation controllers: at most 5 LEDs, so colours are never written over the mode
  and speed registers. Aura USB: on-board LEDs are capped at 16, so extra LEDs no longer make every
  effect fail.
- The GMMK Pro udev rule also matches the keyboard's USB ids.
- A change the service refused (password dialog cancelled, not allowed, unsupported) no longer
  stays on the page.
- Lights Off and scenes update the open device page, and a pending edit can no longer undo them
  or an Undo.
- The hex field keeps following the colour after an invalid entry, and Liquid Glass colour changes
  show at once.
- RPM upgrades no longer turn the service back on after you turned it off.

Packaging
- The repository publishes only packages signed with its own key; before, a package signed with
  any key the build machine trusted would have passed.
- The build no longer handles the private signing key: it uses the published public key, checked
  against a pinned fingerprint. Signing also stops if the secret key is not the pinned one.

## 1.2.0 (2026-11-17)

Lighting
- Glorious GMMK Pro (ANSI and ISO, stock firmware): the keyboard's 18 effects with colour,
  multicolour, speed and brightness, per-key colours painted on a drawing of the keyboard, and the
  side light bars (their own colour, on or off). Built from the protocol of Glorious CORE, the
  vendor's Windows app; see `docs/PROTOCOLS.md`.
- The keyboard has not been tried on a real one yet, so RGBeast starts with it turned off and
  sends it nothing until you choose **Turn On Lighting Control** on its page. It can be turned
  off again there. All Devices and scenes leave a turned-off keyboard alone.
- The keyboard keeps its lighting by itself, so RGBeast does not re-send it at boot or after sleep.
- Only the keyboard's two vendor interfaces are given to the daemon (a udev check reads each
  node's HID descriptor); the interface that carries the keys never is, so the daemon cannot read
  what is typed.
- Wheelguard (the keyboard's settings app) sets the debounce through RGBeast, because the board
  keeps debounce and lighting in the same report. New D-Bus methods for it:
  `GetKeyboardOptions`, `SetKeyboardOptions`, and `SetUnlocked` for turning a device on or off.

App
- A slider drag no longer queues up on slow devices: the daemon applies only the newest of the
  states waiting for a device.

## 1.1.0 (2026-11-04)

Packaging
- Packages and repository metadata are signed, and DNF checks both (`gpgcheck=1`,
  `repo_gpgcheck=1`). The key is at `https://djshiye.github.io/RGBeast/RPM-GPG-KEY-djshiye`.
- One package for every Fedora release: it is built once on Fedora 44, the oldest supported
  release, and published at `https://djshiye.github.io/RGBeast/rpm/` without a Fedora version in
  the path. The old `fedora-44/` address keeps working.
- The package installs its own `/etc/yum.repos.d/rgbeast.repo` and the signing key, so the
  repository stays configured.

App
- The GTK 4 / libadwaita app is replaced by a Qt Quick app (PySide6 + QML) in the audius style:
  dark and light following GNOME and its accent colour, glass popovers and toasts, layered card
  shadows, an integrated header with the desktop's window buttons, down to 560 px wide.
- Every feature is kept: All Devices, scenes, the animated preview with per-LED painting, effects,
  colour wheel, palette and hex entry, multi-colour slots, brightness, speed, direction, random,
  header LED counts, Store as Power-On Lighting, Preferences, and the keyboard shortcuts.
- Preferences that were GSettings keys (window size, last device, animate preview) are now in
  `~/.config/rgbeast/settings.json`; scenes keep their file and format.
- The package no longer needs GTK 4, libadwaita or Blueprint; it needs `python3-pyside6`.

Lighting

- Palette swatches and the All Devices default are fully saturated colours; the pastel
  GNOME palette came out washed-out and whitish on LEDs.
- Aura USB: the effect packet is sent again after the colour packet. The board only showed a new
  colour on the next effect packet, so a colour took two clicks to appear.
- Fury Breath: four equal ramps split at 20 % brightness with a short rest at the bottom, and a
  speed range eight times wider (80 ticks per ramp at 0 %, 5 at 100 %, geometric in between).
  The old 3:1 ramps at 64 % spent most of the cycle dim, so the slider read as the gap between
  breaths, and 0 % was still fast.
- Fury: the preamble (index 0 on every stick, apply) is sent before every change to an animated
  mode, not only on a mode change. It restarts all sticks together, so sticks that drifted apart,
  or kept running out of step through a reboot, are back in step at start-up and on every edit.
- Fury: the register cache is dropped on every mode change and after any failed apply. A stick
  whose Breath settings had been changed behind the driver's back kept them, because the driver
  skipped rewriting values it believed were already set; it breathed between pink and light
  pink while the other stick breathed normally.

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
- CI publishes a DNF repository on GitHub Pages (`rgbeast.repo`); each build carries the commit
  count in its release number, so `dnf upgrade rgbeast` always gets the newest. CI caches Cargo
  and builds Rawhide only on manual runs.
- New driver: Sapphire Nitro Glow V3 (Nitro+, Pure and Toxic cards, I2C address 0x28 on the
  card's own bus). Static colour with brightness, Rainbow, Spectrum Cycle, Runway and Serial with
  speed; state is read back at start. Discovery treats Sapphire cards as GPUs and names the model
  from the PCI subsystem id. Verified on an RX 9070 XT Pure.
- All Devices: a device that lacks the chosen effect now shows the group's colour statically
  instead of being left alone.
- Fury: the mode-change preamble writes 0 to the index register on every stick. Giving the second
  stick its slot index froze it on real Beast DDR5 hardware (writes accepted, never rendered).
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
