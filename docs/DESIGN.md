# Design: Apple's Human Interface Guidelines, applied to RGBeast

Sources: Apple HIG [Design principles](https://developer.apple.com/design/human-interface-guidelines/design-principles),
[Materials](https://developer.apple.com/design/human-interface-guidelines/materials),
[Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos),
[Design resources](https://developer.apple.com/design/resources/). Apple's own first rule is that an
app should feel at home on its platform, so every principle below is realised with GNOME's
components (libadwaita 1.9) rather than by imitating macOS chrome. The result should feel like an
app Apple would ship if Apple shipped GNOME apps.

Reviewed on the target machine on 2026-09-29 (Fedora 44, GNOME 50, a 2560×1440 display at 125 %,
the Orchis-Dark GTK theme as well as stock Adwaita in light, dark and high contrast).

## The eight principles

| Principle (Apple's words) | What it means in RGBeast |
|---|---|
| **Purpose** — "Make something meaningful. Identify what matters most to the people you're designing for." | The one job is: see what your lights are doing and change it. So the preview comes first on every page, the controls follow, and nothing else competes for the space. No telemetry, no accounts, no marketplace, no plug-in chrome. |
| **Agency** — "Let people do things their own way. Give them freedom to act, keep them informed, make it easy to recover." | Every control applies instantly with a quiet "Applied" mark; nothing is modal and there is no Save button. Scenes let you keep states you like; "Lights Off" is one click. Deleting a scene asks first; it never touches the lights. The daemon restores the last state after a reboot or sleep, so experimenting costs nothing. |
| **Responsibility** — "Act in people's best interest. Earn trust by prioritising safety and privacy, and being transparent." | The app never opens hardware. A sandboxed system daemon with no capabilities and no network does, through a narrow API that only knows colours and modes, guarded by polkit. The permission dialog text explains why. Discovery only probes bus types and addresses the protocols document. Everything the daemon found and skipped is readable in Preferences › Detection Log. |
| **Familiarity** — "Build on what people know. Apply concepts consistently." | Standard GNOME navigation: sidebar of things on the left, the selected thing on the right, collapsing to a stack on narrow windows with a back button. Devices and scenes are rows of the same sidebar list. Effects are chips, direction is a segmented control, colours are dots, sliders are sliders. The colour wheel works like every colour wheel: hue on the ring, saturation and brightness in the square. `Ctrl+?` opens the standard shortcuts dialog. |
| **Flexibility** — "Adapt to diverse contexts and needs. Consider a variety of input methods." | The window works from 360 px to full screen; the colour section stacks vertically under 620 px. Everything is reachable by keyboard: chips and swatches are buttons, the wheel takes arrow keys and reports its colour to assistive technology, `Ctrl+R`, `Ctrl+,`, `Ctrl+W`, `Ctrl+Q`, `Ctrl+?`. Every icon button and swatch has an accessible name. Light, dark and high-contrast styles, the system accent colour and the user's GTK theme all follow GNOME because every colour is a libadwaita token; animation stops when the system asks for reduced motion. |
| **Simplicity** — "Be clear and direct. Every element earns its place." | Controls appear only when the selected effect uses them: no speed slider for a static colour, no colour wheel for a rainbow, no direction buttons for breathing. Header LED counts are set where the fans appear in the preview (the device's Zones list) and, with the explanation of what to count, in Preferences. The status line says "3 devices", not a daemon version. One typeface, three text styles (`title`, `body`, `caption`). |
| **Craft** — "Care about every detail." | LEDs are drawn as lenses with a glow, a core and a specular; fans are rings of twelve, memory sticks are bars, in the proportions of the real parts. Radii are concentric: 12 px surfaces, 10 px tiles inside them, pills for choices. Transitions are 120–150 ms ease-out. The hue ring is rasterised once per size and cached; the preview animates at 30 fps at most and never while hidden. The focus ring on the wheel appears only for keyboard focus, in the system accent. Packets and registers are unit-tested byte for byte. |
| **Delight** — "Make it human. People remember how a product makes them feel." | The preview is alive: the effect you picked plays on the card before the hardware confirms it. The chrome stays neutral so the only vivid colour on screen is the light you chose, and it shows up in the sidebar strips too, so the whole window reflects the room. |

## Materials, translated

Apple's [Materials](https://developer.apple.com/design/human-interface-guidelines/materials) page
separates a *functional layer* (navigation and controls, on Liquid Glass) from the *content layer*
(on standard materials), asks that hierarchy come from the material rather than from decoration,
that Liquid Glass be used sparingly and never in the content layer, and that legibility be kept
with vibrant colours on top of materials. GTK has no blur, so the translation is about layering:

| Apple | RGBeast |
|---|---|
| Functional layer: sidebars and toolbars float on their own material | The sidebar is libadwaita's sidebar material (`--sidebar-bg-color`, the `navigation-sidebar` list style): flat rows, a rounded selection, no per-row cards or shadows. The header bars are the toolbar view's flat style; nothing overrides them. |
| Content layer: standard materials for grouping | The editor's sections are libadwaita `.card` surfaces on the window background. The preview is the one deliberately dark surface, in both styles, because lights are read against dark. |
| Hierarchy from the material, not decoration | No drop shadows on rows, no custom sidebar tint, no hairline rings around everything. A 1 px inset ring on the preview and on colour tiles is the only "edge" drawn by hand, and only where a colour surface would otherwise merge with its background. |
| Vibrant, legible colour on materials | Text and icons use the tokens (`--view-fg-color`, `--accent-color`); the only saturated fills are the user's own colours. Selected chips tint with the accent at 20 % and switch the label to `--accent-color`, which libadwaita keeps legible on both styles. |
| Respect system settings (reduced transparency, increased contrast) | Everything is `var(--…)` and `color-mix()`, so libadwaita's high-contrast stylesheet (borders on cards, stronger text) and the user's own GTK theme apply unchanged. Verified with `ADW_DEBUG_HIGH_CONTRAST=1` and with the Orchis-Dark theme. |

## "Designing for macOS" best practices, translated

| Apple's practice | RGBeast |
|---|---|
| Leverage large displays: more content in fewer nested levels, less modality | Sidebar + editor side by side; devices, scenes and the editor are all visible at once above 680 px. Preferences is the only dialog, and it is a sheet, not a window. |
| Let people resize, hide, show and move windows | Free resizing with a 360 × 480 minimum; size and maximised state remembered; the sidebar can be collapsed by resizing. |
| Use the menu bar for every command | GNOME has no menu bar; the primary menu (☰) holds Scan for Devices, Preferences, Keyboard Shortcuts and About, and every command has a keyboard shortcut. |
| High-precision input | The wheel is drag-driven with pixel precision, the hex field takes exact values, and `Shift` with the arrow keys steps the wheel by 10 instead of 2. |
| Keyboard shortcuts and keyboard-only work styles | Tab order follows the visual order: chips, colour slots, wheel, swatches, hex, sliders. Enter in the hex field applies. |
| Personalisation | Header LED counts, animation, restore-after-sleep, and the selected device are remembered per user. Scenes are the user's own presets. |

## Design resources, translated to GNOME

| Apple resource | GNOME equivalent used |
|---|---|
| SF Pro at Dynamic Type sizes | The system UI font (Adwaita Sans on Fedora) through Adwaita style classes only (`.caption-heading`, `.caption`, body); no hard-coded sizes, so Large Text works. |
| SF Mono | The `monospace` class for the hex field. |
| SF Symbols | Adwaita symbolic icons with consistent stroke weight; the app's own symbolic icon is drawn on the 16 px grid. |
| App icon template | The app icon is drawn on the GNOME 128 px grid: a rounded tile, a spectrum ring, a bright centre; it reads at 16 px because the ring survives downscaling. |
| System colours, light and dark | libadwaita tokens only (`var(--sidebar-bg-color)`, `var(--card-bg-color)`, `var(--accent-bg-color)`, `var(--accent-color)`, `var(--success-color)`) mixed with `color-mix()`, so light, dark, high contrast, the accent and user themes follow the system. The stylesheet is loaded one notch above the user stylesheet so a third-party GTK theme's `button {}` rules cannot unshape the chips and swatches; it only targets RGBeast's own classes. |

## Motion

| Moment | Treatment |
|---|---|
| Effect preview | The effect itself plays, 30 fps cap, paused when unmapped, off with reduced motion or the Preferences switch. |
| Page and pane switches | libadwaita stock crossfade / slide. |
| Hover, press, selection | 120–150 ms ease-out on background, ring and colour. |
| "Applied" mark | Fades in over 150 ms, fades out over 400 ms after 1.2 s; both are CSS transitions, so they follow the system's animation setting. |
| Toasts, dialogs | libadwaita stock. |

Rule: no custom animation may run on anything larger than the preview card, and none may block input.
