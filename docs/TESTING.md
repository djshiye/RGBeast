# Hardware bring-up checklist

Everything below is what could not be verified in the cloud container. Work through it in order;
each step names the file or command to look at when it fails.

## 0. Before installing

```bash
# Kernel 6.15 or newer exposes the GPU's lighting I2C bus (Fedora 44 ships 7.x).
uname -r
# The chipset SMBus (DIMMs) and the GPU bus should be visible once i2c-dev is loaded.
sudo modprobe i2c-dev
ls -l /dev/i2c-*
for d in /sys/bus/i2c/devices/i2c-*; do echo "$d: $(cat $d/name)"; done
```

Expected on the target machine:

- `SMBus PIIX4 adapter port 0 at 0b00` (the AMD chipset SMBus, DIMMs live here).
- `AMDGPU DM i2c OEM bus` (the RX 9070's lighting controller).
- The board's USB controller: `lsusb -d 0b05:` shows product `19af` (or `18f3`, `1939`,
  `1aa6`, `1bed`).

If `i2cdetect -y <chipset-bus>` prints `UU` at `0x60..0x67`, a kernel driver has claimed a
DIMM lighting address. That is unusual; `spd5118` normally claims `0x50..0x57` only. Unload with
`sudo rmmod spd5118` to check, and file the finding.

## 1. Install and check the service

```bash
sudo dnf install ./rgbeast-1.0.0-*.rpm
systemctl status rgbeastd
journalctl -u rgbeastd -b
sudo -u rgbeast /usr/libexec/rgbeastd --scan   # what the daemon sees, with its permissions
```

`--scan` prints one line per bus and device. Typical failures:

| Message | Cause | Fix |
|---|---|---|
| `no I2C buses visible` | `i2c-dev` not loaded | `sudo modprobe i2c-dev`; the package's modules-load file handles reboots |
| `cannot open /dev/hidraw*: Permission denied` | udev rule not applied yet, or the node carries an ACL whose `group::` entry is `---` (check with `getfacl`, not `ls`: with an ACL, `ls` shows the mask) | `sudo udevadm trigger --subsystem-match=hidraw --subsystem-match=i2c-dev` then `sudo systemctl restart rgbeastd`; the rule's `setfacl -m g::rw` fixes the group entry |
| `i2c-N: ... Permission denied` | udev rule for i2c-dev not applied | `sudo udevadm trigger --subsystem-match=i2c-dev` |
| `no ENE controller at 0x67` on the GPU bus | wrong bus, or the card's controller answers only after the driver initialises | note which buses were probed; try `i2cdetect -y <bus>` (as root) and look for `67` |
| `unknown ENE controller 'XXXX'` | the card reports a device string this version does not know | send the string; adding it is one line in `crates/rgbeast-core/src/drivers/ene.rs` |

## 2. Motherboard and fans

1. Select the board in the sidebar. Choose **Static**, pick a colour: the board's own LEDs and
   the 12 V headers change.
2. **Preferences › Addressable Headers**: set the LED count of the header the Arctic fans use
   (12 per fan; PST daisy chains add up). The preview shows one ring per fan.
3. **Direct**: click individual lights in the preview; they should change on the fans.
4. **Rainbow**, **Breathing**, **Chase**: hardware effects run without the app open.
5. **Store as Power-On Lighting** (header button): reboot; the lighting comes back before login.

## 3. Memory

1. Select the Kingston device. The subtitle shows the number of sticks detected. If only one
   shows, run `sudo -u rgbeast /usr/libexec/rgbeastd --scan` and report which addresses answered
   (`0x61`, `0x63`).
2. **Static**, then **Slide** with three colours plus a background: the sticks animate.
3. **Direct**: paint individual LEDs.
4. Brightness slider affects all modes.
5. Timing: SMBus writes are paced at 10 ms and retried; a full per-LED update takes about a
   second. If writes fail (toast "Could not apply"), check `journalctl -u rgbeastd`.

## 4. Graphics card

The card in the target machine is a Sapphire RX 9070 XT Pure (PCI subsystem `1da2:3490`) with
the Nitro Glow V3 controller at 0x28 on the OEM bus; the ENE driver is for ASUS cards.

1. Select the card. Static colour, Rainbow, Spectrum Cycle, Runway and Serial apply immediately.
   The card has no power-on store; it keeps its last state by itself.
2. **Store as Power-On Lighting** writes the ENE save value; reboot to confirm.
3. If the card is missing, capture `sudo i2cdetect -l` and `sudo i2cdump -y <gpu-bus> 0x67 b` and
   attach them to the issue.

## 5. Sleep and resume

Suspend and resume. Within a few seconds `journalctl -u rgbeastd` logs "resumed from sleep, restoring
lighting" and all devices return to their state. Toggle **Restore After Sleep** in Preferences to
confirm it is honoured.

## 6. Security checks

```bash
systemd-analyze security rgbeastd          # expect a low exposure score
ps -o user,cmd -C rgbeastd                  # runs as user "rgbeast"
pkaction --verbose --action-id io.github.djshiye.rgbeast.control
```

From another user's session (or over SSH), `busctl call io.github.djshiye.RGBeast1 ... SetState ...`
must be refused with `AccessDenied`.

## 6a. Never probe the GPU's SMU buses

`i2c-N` adapters named `AMDGPU SMU 0` / `AMDGPU SMU 1` belong to the card's power-management
firmware. Even a read-only `i2cdetect` on them made the SMU stop responding on the target machine
(`amdgpu: SMU: No response`, `Failed to disable gfxoff!` every few seconds), after which every
GPU-rendered application took ten seconds to open until a reboot. Discovery classifies those buses
as `Other` and never opens them; `bus_role` has a test for it. Only the `AMDGPU DM i2c OEM bus`
carries a lighting controller.

## 7. Screenshots and layout checks (debug builds)

`rgbeast` in a debug build honours three environment variables: `RGBEAST_DEBUG_SIZE=WxH`,
`RGBEAST_DEBUG_DEVICE=<id>` and `RGBEAST_DEBUG_SHOT=<file.png>` (render the window after loading,
then quit). GNOME's compositor restores a window's last size, so for exact sizes run a headless
mutter and point the app at it:

```bash
mutter --headless --wayland --no-x11 --virtual-monitor 1600x1100 --wayland-display shots &
./target/debug/rgbeastd --session --simulate &
WAYLAND_DISPLAY=shots RGBEAST_BUS=session RGBEAST_DEBUG_SIZE=980x720 \
  RGBEAST_DEBUG_DEVICE=sim:fury RGBEAST_DEBUG_SHOT=fury.png ./target/debug/rgbeast
```

`XDG_CONFIG_HOME=$(mktemp -d)` bypasses a user GTK theme; `ADW_DEBUG_COLOR_SCHEME=prefer-light`
and `ADW_DEBUG_HIGH_CONTRAST=1` cover the other styles.
