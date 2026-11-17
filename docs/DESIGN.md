# Design

RGBeast's app is Qt Quick (PySide6 + QML). Its look is **Spectral**: a deep near-black canvas (a cool
frost in the light style) lit by the colours your lights are showing, glass cards with a lit top rim,
and one accent, the desktop's, drawn as a short gradient that glows. `rgbeast_gui/qml/RGBeast/Theme.qml`
holds the tokens. The rule from the first version still holds: the chrome stays neutral, and the
saturated colour on screen is the light you chose (the ambient light *is* your lights).

## Tokens

| Token | Value |
|---|---|
| Canvas | dark `#07080c`, light `#eceef4` |
| Cards (glass) | dark `#13161f` at 80 %, light white at 84 %; raised `#151822` / white; hover `#1f2330` / `#f4f5fa`; groove `#05060a` / `#e3e6ef` |
| Rim | a 1 px highlight along the top edge of every glass surface, brightest in the middle |
| Text | dark `#f2f4fa` / `#a1a9ba` / `#737b8e`; light `#0e1120` / `#4f566a` / `#646b7e` |
| Accent | GNOME's accent colour (`accent-color`, blue `#3584e4` by default), paired with `accent2`, the same colour turned 27° round the hue wheel, for gradients; used for selection, focus, the primary button and values you set |
| Ambient | up to three colours from the page's preview, as soft clouds behind everything; 20 % strength dark, 13 % light |
| Radii | 6 / 10 / 14 / 22 / 28 px; cards 22, sheets 28 |
| Spacing | 16 px between cards; card padding 18 top, 20 sides and bottom |
| Type | Adwaita Sans, 30 / 15 / 14 / 13 / 12 / 11 px, weights 400 / 500 / 600 / 700; Adwaita Mono for numbers, codes and shortcuts |
| Eyebrows | 11 px bold uppercase with 1.3 px tracking: section names, card titles (after a short glowing accent tick) and the device kind above the page title |
| Motion | ease-out `(0.2, 0.8, 0.2, 1)` and spring `(0.34, 1.36, 0.64, 1)`, 140 / 240 / 420 ms |

## Light

- **Ambient light** (`Aurora.qml`): the colours the current page shows drift slowly behind the
  window as three soft clouds, so the app takes on the colour of your rig. One canvas, clipped to the
  window's corners, repainted four times a second; still when animations are off or the window is
  inactive. Lights off (or black) falls back to a faint wash of the accent.
- **Glow** (`Glow.qml`): lit controls carry a soft halo in their own colour: the active effect chip,
  the primary button, a switch that is on, a slider's fill, the selection beam in the sidebar, the
  colour chip and the swatches. Drawn as stacked rectangles like the shadows, so it looks the same in
  every renderer, including the software one.
- **Selection**: the selected sidebar row is a raised glass surface with a short glowing accent beam
  on its left edge and an accent-tinted glyph tile.

## Following the desktop

- Light or dark follows GNOME's `color-scheme`, the accent follows `accent-color`, and animation
  stops when `enable-animations` is off. All three are watched live through `gsettings monitor`.
- The window buttons follow `org.gnome.desktop.wm.preferences button-layout` (left or right,
  minimise and maximise only when GNOME shows them). Close is always present.
- If `~/.config/liquid-glass/tokens-{dark,light}.json` exists, its colours and card radius override
  the defaults, the same way as in wheelguard and audius.

## Layout

- A transparent, frameless window with its own shadow and a 15 px corner. The header is part of the
  window: drag it to move, double-click to maximise.
- A 288 px sidebar on its own faint glass panel lists All Devices, the devices with their colour
  strips, and the scenes, under eyebrow headings. A status chip at the bottom has a slow heartbeat
  while the service is there. Below 760 px the window shows the
  sidebar or the page, with a back button, and it works down to 560 px.
- The editor is a column of cards, centred up to 980 px: the device with its live preview, Effect,
  Colour, Adjust and Zones. The hero names the kind of device in an eyebrow and shows a monospaced
  readout of the effect and light count. The preview is always a dark stage, in both styles, because
  lights are read against dark: a faint dot grid, corner marks like an instrument's viewfinder, and
  light falling off towards the bottom.
- Brightness and speed show their value as a monospaced accent readout beside the slider.

## Surfaces and depth

- Cards are glass: translucent, so the ambient light tints them, with a sheen from the top and the
  lit rim. In the light style they lift on a soft shadow; in the dark style the rim does that job,
  because a shadow would only muddy the glass. Shadows and glows are stacked rectangles, so they
  look the same on every renderer.
- Popovers, tooltips and toasts are glass: a translucent fill that blurs what is behind it where
  the GPU allows, and stays legible where it does not.
- Sheets (Preferences, Shortcuts, About, scene name, delete) float in a faint accent glow over a
  dimmed window; the dimming covers the window's frame only, never past its rounded corners.

## Interaction

- Every control applies at once. Edits are debounced by 70 ms, and the app adopts the state the
  daemon confirms, so a drag never jumps back to an older value.
- Keyboard: every control takes focus and shows a 2 px accent focus ring. The colour wheel takes
  the arrow keys (Shift for bigger steps). Shortcuts: `Ctrl+R`/`F5`, `Ctrl+,`, `Ctrl+?`, `Ctrl+W`,
  `Ctrl+Q`.
- Every icon button, swatch and slider has an accessible name.
- Errors from the service become short toasts: "The lighting service is not running", "Not allowed
  to change the lighting from this session", or "Could not apply: …". A missing service shows a
  full page with Try Again, and no devices shows Scan Again.
