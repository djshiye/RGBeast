# Protocol notes

These notes describe the wire protocols RGBeast implements. They were assembled from public
documentation and community reverse-engineering (OpenRGB's device documentation and issue tracker,
the `kfrgb` script for Kingston Fury DDR5). RGBeast's implementation is written from scratch in Rust;
no code is copied from those projects.

## ASUS Aura USB mainboard controller

- USB HID, vendor `0x0B05`. Product IDs seen on mainboards: `0x18F3`, `0x1939`, `0x19AF`,
  `0x1AA6`, `0x1BED`. On `0x19AF` the right interface has usage page `0xFF72`, usage `0x00A1`.
- Every packet is 65 bytes: report ID `0x00` is not used, byte 0 is always `0xEC`. Replies are
  read as 65 bytes.

| Byte 1 | Meaning | Rest of packet |
|---|---|---|
| `0x82` | Firmware version request | Reply: byte 1 = `0x02`, bytes 2..18 = ASCII version |
| `0xB0` | Config table request | Reply: byte 1 = `0x30`, bytes 4..64 = 60-byte table |
| `0x52` | "Gen 1" init: `EC 52 53 00 01` | Sent once after reading the table |
| `0x40` | Direct colours | byte 2 = channel, or `0x80 \| channel` on the last packet ("apply"); byte 3 = first LED index; byte 4 = LED count (max 20 per packet); bytes 5.. = R,G,B per LED |
| `0x35` | Effect | byte 2 = effect channel; byte 3 = 0; byte 4 = 1 for the shutdown effect, else 0; byte 5 = mode |
| `0x36` | Effect colour | bytes 2..3 = big-endian LED mask (`((1<<count)-1)<<start`); byte 4 = shutdown flag; bytes 5 + 3*start.. = R,G,B per LED |
| `0x3F` | Commit to flash: `EC 3F 55` | Makes the current effect the power-on default |

Config table offsets: `0x02` = number of addressable headers, `0x1B` = number of on-board LEDs,
`0x1D` = number of 12 V RGB headers (counted inside the on-board LEDs). Channels: the on-board
zone uses effect channel 0 and direct channel `0x04`; addressable header *i* uses effect channel
`1 + i` and direct channel `i`. Addressable headers report 0 LEDs; the chain length is a user
setting (max 120).

Modes: 0 Off, 1 Static, 2 Breathing, 3 Flashing, 4 Spectrum Cycle, 5 Rainbow, 6 Spectrum Cycle
Breathing, 7 Chase Fade, 8 Spectrum Cycle Chase Fade, 9 Chase, 10 Spectrum Cycle Chase, 11
Spectrum Cycle Wave, 12 Chase Rainbow Pulse, 13 Random Flicker, 14 Music, `0xFF` Direct.

## ENE SMBus (ASUS GPUs, ASUS Aura SMBus boards, ENE DRAM)

- 7-bit I2C address `0x67` on GPUs (`0x70..0x77`, `0x4F`, `0x66`, `0x39..0x3D` for DRAM;
  `0x40`, `0x4E`, `0x4F` for boards).
- Register access is indirect with a 16-bit register address:
  - write register address: SMBus **write word** to command `0x00` with the address byte-swapped
    (`(reg << 8) | (reg >> 8)`),
  - read value: SMBus **read byte data** from command `0x81`,
  - write value: SMBus **write byte data** to command `0x01`,
  - write block: SMBus **write block data** to command `0x03`, 3 bytes max per block; fall back to
    single byte writes if the block write fails.
- Probe: the device answers a receive-byte; registers `0xA0..0xAF` read back as `0x00..0x0F`;
  16 bytes at `0x1030` must not spell "Micron".

| Register | Meaning |
|---|---|
| `0x1000` | 16-byte device string, e.g. `AUMA0-E6K5-0107` (GPU), `LED-0116` (board) |
| `0x1C00` | 64-byte config table: `[0x02]` LED count (v1), `[0x03]` LED count (GPU 0107 family), `[0x03 + zone]` LEDs per zone, `[0x13 + zone]` (v1) or `[0x1B + zone]` (v2) zone channel IDs |
| `0x8000` / `0x8100` | direct colours v1 / v2, 3 bytes per LED in **R, B, G** order |
| `0x8010` / `0x8160` | effect colours v1 / v2, same layout |
| `0x8020` | direct mode enable (1) |
| `0x8021` | mode |
| `0x8022` | speed: 0 fastest .. 4 slowest |
| `0x8023` | direction: 0 forward, 1 reverse |
| `0x80A0` | apply (`0x01`) or save to flash (`0xAA`) |

v2 registers apply to device strings `AUMA0-E6K5-01xx/11xx`, `AUDA0-E6K5-0101` and `ROG STRIX
ARION`; v1 to `LED-0116`, `DIMM_LED-0102`, `AUMA0-E8K4-0101`. The mode list is the same as Aura
USB (0..13) plus 14 Double Fade on some DRAM.

Zone channel IDs: `0x82` Center start, `0x83` Center, `0x84` Audio, `0x85` Back I/O, `0x86` RGB
Header, `0x87` RGB Header 2, `0x88` Backplate, `0x8A`/`0x05`/`0x0E` DRAM, `0x8B` PCIe, `0x91` RGB
Header 3.

## Kingston Fury Beast / Renegade DDR5

- Chipset SMBus. Address `0x60 + slot index` (`0x58 + slot` on DDR4). Two sticks in A2/B2
  typically show up at `0x61` and `0x63`.
- Reads are SMBus **read word data**; the value is the high byte. Writes are **write byte data**.
  Every write needs about 10 ms before the next; retry on `EAGAIN`/`EIO` up to 5 times with
  growing delays.
- Transaction: write `0x53` to register `0x08` (begin), set registers, write `0x44` to `0x08`
  (end/apply). Signature: registers `0x01..0x04` read "F","U","R","Y"; register `0x06` is the model
  (`0x10` Beast, `0x11` Renegade, `0x12` Beast White, `0x15` Beast v2; DDR4 `0x21`, `0x23`).
- When the mode changes, a preamble is sent first: begin, write the slot index to register `0x0B`
  on each stick (0 for unsynchronised modes), apply.

| Register | Meaning |
|---|---|
| `0x09` | mode |
| `0x0C` | direction: 1 bottom-to-top, 2 top-to-bottom |
| `0x0D` | delay |
| `0x0E` | speed (most modes: lower is faster) |
| `0x12..0x15` | Dynamic hold/fade times |
| `0x16..0x1D` | Breath timing and brightness levels |
| `0x20` | brightness 0..100 |
| `0x23..0x25` | background colour R,G,B |
| `0x26` | length |
| `0x27` | number of sticks taking part (max 4) |
| `0x30` | number of mode colours (max 10) |
| `0x31 + 3n` | mode colour n: R,G,B |
| `0x50 + 3n` | per-LED colour n (12 LEDs): R,G,B |

Modes (value written to `0x09`): Static `0x00`, Rainbow/Spectrum `0x01`, Rhythm `0x02`, Breath
`0x03` (`0x13` per-LED), Dynamic `0x04`, Slide/Slither/Teleport/Wind `0x05`, Comet/Rain/Firework
`0x06`, Voltage `0x07`, Countdown `0x08`, Flame `0x09`, Twilight `0x0A`, Fury `0x0B`, Direct
`0x10`, Prism `0x11`. Modes that share a value differ in delay/length/direction registers.

## Linux plumbing

- I2C adapters: `/sys/bus/i2c/devices/i2c-N/name`. `SMBus PIIX4 adapter port 0 at 0b00` is the
  AMD chipset bus that carries the DIMMs. `AMDGPU DM i2c OEM bus` (kernel 6.15+) is the GPU
  lighting bus. `/sys/bus/i2c/devices/i2c-N` is a symlink into the owning device's tree
  (`/sys/devices/pci…/0000:03:00.0/i2c-N`), so the PCI parent is found by resolving the link and
  walking up until a directory with `vendor` and `device` files appears; `subsystem_vendor`
  `0x1043` is ASUS. Only PIIX4 port 0 is treated as the chipset bus, and only DIMM slots that
  have an SPD device (`<bus>-005<slot>`) are probed for lighting controllers.
- Opening `/dev/i2c-N` and selecting the slave uses `I2C_SLAVE`; RGBeast never uses
  `I2C_SLAVE_FORCE`, so it cannot talk to an address a kernel driver already owns.
- hidraw devices: `/sys/class/hidraw/hidrawN/device/uevent` (`HID_ID=0003:00000B05:000019AF`),
  report descriptor at `.../device/report_descriptor` for the usage page.
