# Design: Apple's Human Interface Guidelines, applied to RGBeast

Sources: Apple HIG [Design principles](https://developer.apple.com/design/human-interface-guidelines/design-principles),
[Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos),
[Design resources](https://developer.apple.com/design/resources/). Apple's own first rule is that an
app should feel at home on its platform, so every principle below is realised with GNOME's
components (libadwaita) rather than by imitating macOS chrome. The result should feel like an app
Apple would ship if Apple shipped GNOME apps.

## The eight principles

| Principle (Apple's words) | What it means in RGBeast |
|---|---|
| **Purpose** — "Make something meaningful. Identify what matters most to the people you're designing for." | The one job is: see what your lights are doing and change it. So the preview comes first on every page, the controls follow, and nothing else competes for the space. No telemetry, no accounts, no marketplace, no plug-in chrome. |
| **Agency** — "Let people do things their own way. Give them freedom to act, keep them informed, make it easy to recover." | Every control applies instantly with a quiet "Applied" mark; nothing is modal and there is no Save button. Scenes let you keep states you like; "Lights Off" is one click. Deleting a scene asks first; it never touches the lights. The daemon restores the last state after a reboot or sleep, so experimenting costs nothing. |
| **Responsibility** — "Act in people's best interest. Earn trust by prioritising safety and privacy, and being transparent." | The app never opens hardware. A sandboxed system daemon with no capabilities and no network does, through a narrow API that only knows colours and modes, guarded by polkit. The permission dialog text explains why. Discovery only probes bus types and addresses the protocols document. Everything the daemon found and skipped is readable in Preferences › Detection Log. |
| **Familiarity** — "Build on what people know. Apply concepts consistently." | Standard GNOME navigation: sidebar of things on the left, the selected thing on the right, collapsing to a stack on narrow windows with a back button. Devices and scenes are the same kind of card. Effects are chips, colours are dots, sliders are sliders. The colour wheel works like every colour wheel: hue on the ring, saturation and brightness in the square. |
| **Flexibility** — "Adapt to diverse contexts and needs. Consider a variety of input methods." | The window works from 360 px to full screen; the colour section stacks vertically under 620 px. Everything is reachable by keyboard: chips and swatches are buttons, the wheel takes arrow keys, `Ctrl+R`, `Ctrl+,`, `Ctrl+W`, `Ctrl+Q`. Every icon button and swatch has an accessible name. Light and dark styles and the system accent colour follow GNOME; animation stops when the system asks for reduced motion. |
| **Simplicity** — "Be clear and direct. Every element earns its place." | Controls appear only when the selected effect uses them: no speed slider for a static colour, no colour wheel for a rainbow, no direction buttons for breathing. Zone sizes live in Preferences, not on the main page. One typeface, three text sizes. |
| **Craft** — "Care about every detail." | LEDs are drawn as lenses with a glow, a core and a specular; fans are rings of twelve, memory sticks are bars, in the proportions of the real parts. Rings, radii and spacing come from one 6 px grid. Transitions are 120–150 ms ease-out. The hue ring is rasterised once per size and cached; the preview animates at 30 fps at most and never while hidden. Packets and registers are unit-tested byte for byte. |
| **Delight** — "Make it human. People remember how a product makes them feel." | The preview is alive: the effect you picked plays on the card before the hardware confirms it. The chrome stays neutral so the only vivid colour on screen is the light you chose, and it shows up in the sidebar strips too, so the whole window reflects the room. |

## "Designing for macOS" best practices, translated

| Apple's practice | RGBeast |
|---|---|
| Leverage large displays: more content in fewer nested levels, less modality | Sidebar + editor side by side; devices, scenes and the editor are all visible at once above 680 px. Preferences is the only dialog, and it is a sheet, not a window. |
| Let people resize, hide, show and move windows | Free resizing with a 360 × 480 minimum; size and maximised state remembered; the sidebar can be collapsed by resizing. |
| Use the menu bar for every command | GNOME has no menu bar; the primary menu (☰) holds Scan for Devices, Preferences and About, and every command has a keyboard shortcut. |
| High-precision input | The wheel is drag-driven with pixel precision, the hex field takes exact values, and `Shift` with the arrow keys steps the wheel by 10 instead of 2. |
| Keyboard shortcuts and keyboard-only work styles | Tab order follows the visual order: chips, colour slots, wheel, swatches, hex, sliders. Enter in the hex field applies. |
| Personalisation | Header LED counts, animation, restore-after-sleep, and the selected device are remembered per user. Scenes are the user's own presets. |

## Design resources, translated to GNOME

| Apple resource | GNOME equivalent used |
|---|---|
| SF Pro at Dynamic Type sizes | The system UI font (Adwaita Sans on Fedora) through Adwaita style classes only (`.title-4`, `.heading`, `.caption`); no hard-coded sizes, so Large Text works. |
| SF Mono | The `monospace` class for the hex field. |
| SF Symbols | Adwaita symbolic icons with consistent stroke weight; the app's own symbolic icon is drawn on the 16 px grid. |
| App icon template | The app icon is drawn on the GNOME 128 px grid: a rounded tile, a spectrum ring, a bright centre; it reads at 16 px because the ring survives downscaling. |
| System colours, light and dark | Adwaita named colours only (`@window_bg_color`, `@card_bg_color`, `@accent_bg_color`, `@view_fg_color`) with `alpha()` and `mix()`, so light, dark and accent follow the system. |

## Motion

| Moment | Treatment |
|---|---|
| Effect preview | The effect itself plays, 30 fps cap, paused when unmapped, off with reduced motion or the Preferences switch. |
| Page and pane switches | libadwaita stock crossfade / slide. |
| Hover, press, selection | 120–150 ms ease-out on background, ring and colour. |
| "Applied" mark | Appears immediately, fades over 400 ms after 1.2 s. |
| Toasts, dialogs | libadwaita stock. |

Rule: no custom animation may run on anything larger than the preview card, and none may block input.
