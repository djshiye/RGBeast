"""The device model the daemon sends (crates/rgbeast-core/src/model.rs) as plain Python, plus the app
logic that used to live in the GTK crate's model.rs: the "All Devices" group, mapping a group state to
what each device can do, and the colours each zone shows in the preview and the sidebar.

Device infos are kept as the daemon's JSON dicts. States are normalised dicts whose colours are
(r, g, b) tuples; `state_from_json` / `state_to_json` convert at the D-Bus boundary.
"""
import copy
from typing import Dict, List, Optional, Tuple

from . import color as C

ALL_ID = "all"

STICK, FANS, STRIP = "stick", "fans", "strip"

# Fully saturated: LEDs render pastel hex values as washed-out white.
PALETTE: List[C.Rgb] = [
    (0xFF, 0x00, 0x00), (0xFF, 0x40, 0x00), (0xFF, 0xA0, 0x00), (0x00, 0xFF, 0x00),
    (0x00, 0xFF, 0xFF), (0x00, 0x80, 0xFF), (0x00, 0x00, 0xFF), (0x60, 0x00, 0xFF),
    (0xA0, 0x00, 0xFF), (0xFF, 0x00, 0xFF), (0xFF, 0x00, 0x60), (0xFF, 0xFF, 0xFF),
]

KIND_ICONS = {
    "motherboard": "board", "dram": "memory", "gpu": "gpu", "fans": "fan",
    "strip": "strip", "cooler": "cooler", "unknown": "question",
}

DIRECTION_LABELS = {"forward": "Forward", "reverse": "Reverse", "up": "Up", "down": "Down"}


# ------------------------------------------------------------------ info helpers

def mode_info(info: dict, mode_id: str) -> Optional[dict]:
    return next((m for m in info.get("modes", []) if m.get("id") == mode_id), None)


def zone_info(info: dict, zone_id: str) -> Optional[dict]:
    return next((z for z in info.get("zones", []) if z.get("id") == zone_id), None)


def led_count(info: dict) -> int:
    return sum(int(z.get("leds", 0)) for z in info.get("zones", []))


def is_sizable(z: dict) -> bool:
    return z.get("leds_min", 0) != z.get("leds_max", 0)


def new_mode(mode_id, name, color_mode="none", cmin=0, cmax=0, speed=False, brightness=False,
             directions=(), random=False) -> dict:
    return {"id": mode_id, "name": name, "color_mode": color_mode, "colors_min": cmin, "colors_max": cmax,
            "has_speed": speed, "has_brightness": brightness, "directions": list(directions),
            "has_random": random}


def all_devices_info() -> dict:
    """The "All Devices" pseudo-device: the modes every driver can express."""
    return {
        "id": ALL_ID, "name": "All Devices", "vendor": "", "kind": "unknown", "location": "",
        "driver": "group", "version": "", "zones": [], "can_save": False,
        "modes": [
            new_mode("static", "Static", "mode-colors", 1, 1, brightness=True),
            new_mode("off", "Off"),
            new_mode("breathing", "Breathing", "mode-colors", 1, 1, speed=True, brightness=True),
            new_mode("rainbow", "Rainbow", speed=True, brightness=True),
            new_mode("spectrum-cycle", "Spectrum Cycle", speed=True, brightness=True),
        ],
    }


# ------------------------------------------------------------------ states

def default_state() -> dict:
    return {"mode": "static", "colors": [C.WHITE], "brightness": 100, "speed": 50,
            "direction": "", "random": False, "zones": []}


def static_state(c: C.Rgb) -> dict:
    s = default_state()
    s["colors"] = [c]
    return s


def off_state() -> dict:
    s = default_state()
    s["mode"] = "off"
    s["colors"] = []
    return s


def state_from_json(d) -> dict:
    """A DeviceState JSON object (serde defaults applied) as a normalised dict."""
    if not isinstance(d, dict):
        return default_state()

    def num(key, default):
        try:
            return max(0, min(100, int(d.get(key, default))))
        except (TypeError, ValueError):
            return default

    zones = []
    for z in d.get("zones") or []:
        if isinstance(z, dict) and isinstance(z.get("id"), str):
            zones.append({"id": z["id"], "colors": [C.from_json(c) for c in z.get("colors") or []]})
    return {
        "mode": str(d.get("mode") or "static"),
        "colors": [C.from_json(c) for c in d.get("colors") or []],
        "brightness": num("brightness", 100),
        "speed": num("speed", 50),
        "direction": str(d.get("direction") or ""),
        "random": bool(d.get("random", False)),
        "zones": zones,
    }


def state_to_json(s: dict) -> dict:
    return {
        "mode": s["mode"],
        "colors": [C.to_json(c) for c in s["colors"]],
        "brightness": int(s["brightness"]),
        "speed": int(s["speed"]),
        "direction": s["direction"],
        "random": bool(s["random"]),
        "zones": [{"id": z["id"], "colors": [C.to_json(c) for c in z["colors"]]} for z in s["zones"]],
    }


def primary_color(s: dict) -> C.Rgb:
    """The first mode colour, or the first per-LED colour, or white."""
    if s["colors"]:
        return s["colors"][0]
    for z in s["zones"]:
        if z["colors"]:
            return z["colors"][0]
    return C.WHITE


def zone_colors(s: dict, zone_id: str, leds: int) -> List[C.Rgb]:
    """Per-LED colours for a zone, padded with the last colour / truncated to `leds`."""
    base = next((z["colors"] for z in s["zones"] if z["id"] == zone_id and z["colors"]), None)
    if not base:
        base = [primary_color(s)]
    out = list(base[:leds])
    while len(out) < leds:
        out.append(base[-1])
    return out


def preview_colors(info: dict, s: dict) -> List[Tuple[dict, List[C.Rgb]]]:
    """The colours each zone shows for a state: per-LED colours, the mode colours spread over the
    LEDs, black when off, or a rainbow for colourless effects."""
    m = mode_info(info, s["mode"])
    cm = m.get("color_mode") if m else None
    out = []
    for z in info.get("zones", []):
        n = int(z.get("leds", 0))
        if cm == "per-led":
            colors = zone_colors(s, z["id"], n)
        elif cm == "mode-colors":
            cs = s["colors"] or [C.WHITE]
            colors = [cs[i * len(cs) // max(n, 1)] for i in range(n)]
        elif s["mode"] == "off":
            colors = [C.BLACK] * n
        else:
            colors = [C.from_hsv(i * 360.0 / max(n, 1), 1.0, 1.0) for i in range(n)]
        out.append((z, colors))
    return out


def map_group_state(info: dict, group: dict) -> Optional[dict]:
    """Translate an "All Devices" state to what one device can do (None: leave the device alone)."""
    def has(mid):
        return mode_info(info, mid) is not None

    candidates = {
        "static": ("static", "direct"),
        "off": ("off",),
        "breathing": ("breathing", "breath"),
        "rainbow": ("rainbow", "spectrum-cycle", "spectrum"),
        "spectrum-cycle": ("spectrum-cycle", "spectrum", "prism", "rainbow"),
    }.get(group["mode"], ())
    s = copy.deepcopy(group)
    s["zones"] = []
    for mid in candidates:
        if has(mid):
            s["mode"] = mid
            return s
    if group["mode"] == "off" and has("static"):
        # No "off" on this device: static black at zero brightness.
        s.update(mode="static", colors=[C.BLACK], brightness=0)
        return s
    if has("static") and group["colors"]:
        # No such effect on this device: at least show the group's colour.
        s["mode"] = "static"
        return s
    return None


# ------------------------------------------------------------------ presentation

def zone_shape(info: dict, z: dict) -> str:
    """Fans on addressable headers, sticks for memory, strips otherwise."""
    leds = int(z.get("leds", 0))
    if info.get("kind") == "dram":
        return STICK
    if str(z.get("id", "")).startswith("argb") and leds > 0 and leds % 12 == 0:
        return FANS
    return STRIP


def device_icon(info: dict) -> str:
    if info.get("id") == ALL_ID:
        return "grid"
    return KIND_ICONS.get(info.get("kind", "unknown"), "question")


def subtitle(info: dict, s: dict) -> str:
    """Mode name and LED count, e.g. "Rainbow · 63 LEDs"."""
    if info.get("id") == ALL_ID:
        return "Set everything at once"
    m = mode_info(info, s["mode"])
    n = led_count(info)
    return f"{m['name'] if m else s['mode']} · {n} LED{'' if n == 1 else 's'}"


def strip_colors(info: dict, s: dict) -> List[C.Rgb]:
    return [c for _, cs in preview_colors(info, s) for c in cs]


def short_name(name: str) -> str:
    words = name.split()
    return name if len(words) <= 3 else " ".join(words[:3])


def direction_label(d: str) -> str:
    return DIRECTION_LABELS.get(d, d)


def group_preview(devices: List[Tuple[dict, dict]]) -> List[dict]:
    """Every real device's zones for the "All Devices" preview, one row per device, short labels."""
    out = []
    for info, s in devices:
        kind = info.get("kind")
        tag = {"motherboard": "Board", "dram": "RAM", "gpu": "GPU"}.get(kind) or short_name(info.get("name", ""))
        first = True
        zones = info.get("zones", [])
        for z, colors in preview_colors(info, s):
            if not colors:
                continue
            zname = str(z.get("name", "")).replace("Addressable Header", "Header")
            if len(zones) <= 1 or z.get("id") == "mainboard":
                name = tag
            elif kind == "dram":
                name = zname
            else:
                name = f"{tag} · {zname}"
            out.append({"name": name, "shape": zone_shape(info, z), "colors": colors, "row_break": first})
            first = False
    return out


def device_preview(info: dict, s: dict) -> List[dict]:
    return [{"name": z.get("name", ""), "shape": zone_shape(info, z), "colors": colors, "row_break": False}
            for z, colors in preview_colors(info, s)]
