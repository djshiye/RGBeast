# Design

RGBeast's app is Qt Quick (PySide6 + QML) and shares its visual language with **audius**: the
design source of truth is `audius/ui/css/tokens.css`, and `rgbeast_gui/qml/RGBeast/Theme.qml`
carries the same tokens. The rule from the first version still holds: the chrome stays quiet and
neutral, and the only saturated colour on screen is the light you chose.

## Tokens

| Token | Value |
|---|---|
| Surfaces (dark) | background `#0d0f12`, card `#16191e`, raised `#1e2228`, hover `#262b33`, groove `#0b0d10` |
| Surfaces (light) | background `#f2f2f5`, card and raised `#ffffff`, hover `#f4f4f7`, groove `#e7e7ec` |
| Accent | GNOME's accent colour (`accent-color`, blue `#3584e4` by default); used for selection, focus and the primary button only |
| Radii | 6 / 9 / 13 / 20 / 26 px; cards 20 |
| Spacing | 18 px between cards; card padding 18 top, 20 sides and bottom |
| Type | Adwaita Sans, 28 / 15 / 14 / 13 / 12 / 11 px, weights 400 / 500 / 600 / 700 |
| Motion | ease-out `(0.2, 0.8, 0.2, 1)` and spring `(0.34, 1.36, 0.64, 1)`, 140 / 240 / 420 ms |

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
- A 288 px sidebar on the window background lists All Devices, the devices with their colour
  strips, and the scenes. The selected row is a raised surface. Below 760 px the window shows the
  sidebar or the page, with a back button, and it works down to 560 px.
- The editor is a column of cards, centred up to 980 px: the device with its live preview, Effect,
  Colour, Adjust and Zones. The preview is always a dark stage, in both styles, because lights are
  read against dark.

## Surfaces and depth

- Cards and selected rows have layered shadows, drawn as stacked rectangles so they look the same
  on every renderer.
- Popovers, tooltips and toasts are glass: a translucent fill that blurs what is behind it where
  the GPU allows, and stays legible where it does not.
- Sheets (Preferences, Shortcuts, About, scene name, delete) sit over a dimmed window.

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
