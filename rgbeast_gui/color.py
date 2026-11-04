"""sRGB colours as (r, g, b) tuples of 0..255, with the conversions the app needs.

Mirrors crates/rgbeast-core/src/color.rs, so the preview shows what the daemon will send.
"""
import math
from typing import Optional, Tuple

Rgb = Tuple[int, int, int]
BLACK: Rgb = (0, 0, 0)
WHITE: Rgb = (255, 255, 255)


def _u8(x: float) -> int:
    return max(0, min(255, int(round(x * 255.0))))


def from_hex(s: str) -> Optional[Rgb]:
    """`#rrggbb` or `rrggbb`, case-insensitive; anything else is None."""
    s = (s or "").strip().lstrip("#")
    if len(s) != 6 or any(c not in "0123456789abcdefABCDEF" for c in s):
        return None
    v = int(s, 16)
    return (v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF


def to_hex(c: Rgb) -> str:
    return "#{:02X}{:02X}{:02X}".format(*c)


def from_hsv(h: float, s: float, v: float) -> Rgb:
    """Hue in degrees, saturation and value 0..1."""
    h = (h % 360.0) / 60.0
    s = min(1.0, max(0.0, s))
    v = min(1.0, max(0.0, v))
    i = int(math.floor(h))
    f = h - math.floor(h)
    p, q, t = v * (1 - s), v * (1 - s * f), v * (1 - s * (1 - f))
    r, g, b = ((v, t, p), (q, v, p), (p, v, t), (p, q, v), (t, p, v), (v, p, q))[i % 6]
    return _u8(r), _u8(g), _u8(b)


def to_hsv(c: Rgb) -> Tuple[float, float, float]:
    r, g, b = (x / 255.0 for x in c)
    mx, mn = max(r, g, b), min(r, g, b)
    d = mx - mn
    if d == 0:
        h = 0.0
    elif mx == r:
        h = 60.0 * (((g - b) / d) % 6.0)
    elif mx == g:
        h = 60.0 * ((b - r) / d + 2.0)
    else:
        h = 60.0 * ((r - g) / d + 4.0)
    return h, (0.0 if mx == 0 else d / mx), mx


def scaled(c: Rgb, brightness: int) -> Rgb:
    """Scale by a 0..100 brightness, linear in sRGB like the controllers (integer maths as in Rust)."""
    k = max(0, min(100, int(brightness)))
    return tuple(x * k // 100 for x in c)  # type: ignore[return-value]


def mix(a: Rgb, b: Rgb, t: float) -> Rgb:
    return tuple(int(x * (1.0 - t) + y * t) for x, y in zip(a, b))  # type: ignore[return-value]


def from_json(v) -> Rgb:
    """{"r":..,"g":..,"b":..} as serde writes it; bad input becomes black."""
    try:
        return (max(0, min(255, int(v["r"]))), max(0, min(255, int(v["g"]))), max(0, min(255, int(v["b"]))))
    except (KeyError, TypeError, ValueError):
        return BLACK


def to_json(c: Rgb) -> dict:
    return {"r": int(c[0]), "g": int(c[1]), "b": int(c[2])}
