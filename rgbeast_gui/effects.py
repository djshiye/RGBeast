"""The LED preview without Qt: where every LED is drawn (fans as rings of 12, memory sticks as bars,
everything else as strips, flowed into centred rows) and the software approximation of each hardware
effect. Ported from the GTK crate's led_preview.rs.
"""
import math
from dataclasses import dataclass, field
from typing import List, Tuple

from . import color as C
from .model import FANS, STICK, STRIP

PAD = 18.0
FAN_D = 74.0
STICK_W = 24.0
STICK_H = 140.0
DOT = 6.0
LABEL_H = 18.0
PITCH = DOT * 2.0 + 8.0


@dataclass
class Dot:
    x: float
    y: float
    r: float
    zone: int
    led: int
    t: float        # 0..1 along the zone, for travelling effects


@dataclass
class Frame:
    outlines: List[Tuple[float, float, float, float, float]] = field(default_factory=list)  # x, y, w, h, radius
    labels: List[Tuple[float, float, float, str]] = field(default_factory=list)            # x, y, w, text
    dots: List[Dot] = field(default_factory=list)
    height: float = 0.0


def layout(zones: List[dict], width: float) -> Frame:
    """Lay out zones ({name, shape, colors, row_break}) for a width."""
    frame = Frame()
    usable = max(width - PAD * 2.0, 120.0)
    items = []      # (w, h, zone, shape, first_led, count)
    for zi, z in enumerate(zones):
        n = len(z["colors"])
        if n == 0:
            continue
        if z["shape"] == FANS:
            fans = n // 12
            for f in range(fans):
                items.append((FAN_D + 12.0, FAN_D + LABEL_H + 6.0, zi, FANS, f * 12, 12))
            rest = n % 12
            if rest:
                items.append((max(rest * PITCH, 60.0), DOT * 2 + 16 + LABEL_H, zi, STRIP, fans * 12, rest))
        elif z["shape"] == STICK:
            items.append((STICK_W + 56.0, STICK_H + LABEL_H + 6.0, zi, STICK, 0, n))
        else:
            per_row = max(1, int((usable - 24.0) // PITCH))
            start = 0
            while start < n:
                count = min(n - start, per_row)
                items.append((count * PITCH + 24.0, DOT * 2 + 16 + LABEL_H, zi, STRIP, start, count))
                start += count

    y = PAD
    i = 0
    while i < len(items):
        row_w = 0.0
        j = i
        while j < len(items) and (row_w == 0.0 or row_w + items[j][0] <= usable) and (
                j == i or not (zones[items[j][2]].get("row_break") and items[j][2] != items[j - 1][2])):
            row_w += items[j][0]
            j += 1
        row_h = max(it[1] for it in items[i:j])
        x = PAD + (usable - row_w) / 2.0
        spans = []      # [zone, x0, x1, label_y]
        for w, _h, zi, shape, first, count in items[i:j]:
            n_zone = len(zones[zi]["colors"])
            cx = x + w / 2.0
            if shape == FANS:
                cy = y + FAN_D / 2.0 + 2.0
                r = FAN_D / 2.0
                frame.outlines.append((cx - r, cy - r, FAN_D, FAN_D, r))
                frame.outlines.append((cx - 10, cy - 10, 20, 20, 10))
                for k in range(count):
                    a = -math.pi / 2 + k / count * math.tau
                    rr = r - 11.0
                    frame.dots.append(Dot(cx + rr * math.cos(a), cy + rr * math.sin(a), DOT, zi, first + k,
                                          (first + k) / n_zone))
                label_y = y + FAN_D + 6.0
            elif shape == STICK:
                top = y + 2.0
                frame.outlines.append((cx - STICK_W / 2, top, STICK_W, STICK_H, 6.0))
                nn = max(count, 1)
                for k in range(count):
                    ly = top + 10.0 + (STICK_H - 20.0) * (k + 0.5) / nn
                    frame.dots.append(Dot(cx, ly, DOT - 1.0, zi, k, 1.0 - k / nn))
                label_y = top + STICK_H + 6.0
            else:
                cy = y + DOT + 8.0
                x0 = cx - (count * PITCH) / 2.0 + PITCH / 2.0
                for k in range(count):
                    led = first + k
                    frame.dots.append(Dot(x0 + k * PITCH, cy, DOT, zi, led, led / max(n_zone, 1)))
                label_y = cy + DOT + 8.0
            if spans and spans[-1][0] == zi:
                spans[-1][2] = x + w
                spans[-1][3] = max(spans[-1][3], label_y)
            else:
                spans.append([zi, x, x + w, label_y])
            x += w
        for zi, x0, x1, ly in spans:
            # Widen narrow items a little so short names fit, without crossing neighbours.
            pad = min((x1 - x0) * 0.15, 14.0)
            frame.labels.append((x0 - pad, ly, x1 - x0 + pad * 2, zones[zi]["name"]))
        y += row_h + 10.0
        i = j
    frame.height = max(y + PAD - 10.0, 140.0)
    return frame


def is_animated(mode: str) -> bool:
    return mode not in ("static", "direct", "off", "")


def hash01(a: int, b: int) -> float:
    m = 0xFFFFFFFF
    x = ((a * 0x9E3779B9) & m) ^ ((b * 0x85EBCA6B) & m)
    x ^= x >> 15
    x = (x * 0x2C1B3C6D) & m
    x ^= x >> 12
    return (x & 0xFFFF) / 65535.0


TAILED = ("comet", "chase-fade", "wind", "fury")
FLOORED = ("slide", "wind", "voltage", "fury", "teleport", "slither", "rhythm")


def color_at(zones: List[dict], d: Dot, t: float, effect: dict) -> C.Rgb:
    """The colour of one LED at time `t` (seconds) under the effect
    {mode, speed, brightness, colors, reverse, random}."""
    z = zones[d.zone] if d.zone < len(zones) else None
    base = z["colors"][d.led] if z and d.led < len(z["colors"]) else C.BLACK
    mode = effect.get("mode", "")
    rate = 0.35 + effect.get("speed", 50) / 100.0 * 1.9
    sign = -1.0 if effect.get("reverse") else 1.0
    mode_colors = effect.get("colors") or []
    rnd = effect.get("random", False)
    n_zone = float(max(len(z["colors"]) if z else 1, 1))

    def palette(k):
        return base if not mode_colors else mode_colors[k % len(mode_colors)]

    if mode == "off":
        out = C.BLACK
    elif mode in ("static", "direct", ""):
        out = base
    elif mode in ("breathing", "breath"):
        k = 0.5 + 0.5 * math.sin(t * rate * 2.0)
        c = C.from_hsv((t * rate * 40.0) % 360.0, 1, 1) if rnd else base
        out = C.scaled(c, int(k * 100))
    elif mode == "flashing":
        out = base if (t * rate * 1.5) % 1.0 < 0.5 else C.BLACK
    elif mode == "rainbow":
        out = C.from_hsv((d.t * 360.0 + sign * t * rate * 80.0) % 360.0, 1, 1)
    elif mode in ("spectrum", "spectrum-cycle", "prism", "twilight"):
        out = C.from_hsv((t * rate * 50.0 + (d.t * 60.0 if mode == "prism" else 0.0)) % 360.0, 1, 1)
    elif mode == "random-flicker":
        step = int(t * 6.0 * rate)
        k = hash01(d.zone * 131 + d.led, step)
        out = C.from_hsv(hash01(d.led, step // 3) * 360.0, 1, 0.2 + 0.8 * k)
    elif mode == "flame":
        step = int(t * 10.0 * rate)
        k = hash01(d.led + d.zone * 977, step)
        out = C.from_hsv(10.0 + 30.0 * k, 1, 0.35 + 0.65 * k)
    elif mode in ("dynamic", "rhythm"):
        lap = int(t * rate * 0.5)
        phase = (t * rate * 0.5) % 1.0
        out = C.mix(palette(lap), palette(lap + 1), phase)
    else:
        # Travelling highlight: chase, comet, slide, wind, rain, firework, voltage, fury, teleport, slither.
        cycle = t * rate * 1.2
        lap = int(cycle)
        pos = (cycle % 1.0) * (n_zone + 3.0) - 1.5
        led_pos = d.t * n_zone if sign > 0 else (1.0 - d.t) * n_zone
        tail = 4.0 if mode in TAILED else 2.0
        k = min(1.0, max(0.0, 1.0 - abs(led_pos - pos) / tail))
        c = C.from_hsv(lap * 47.0 % 360.0, 1, 1) if rnd else palette(lap)
        floor = mode_colors[-1] if len(mode_colors) >= 2 and mode in FLOORED else C.BLACK
        out = floor if k <= 0.0 else C.scaled(c, int(k * 100))
    return C.scaled(out, effect.get("brightness", 100))


def hit(frame: Frame, x: float, y: float):
    """The (zone, led) under a click, or None."""
    best = None
    bd = float("inf")
    for d in frame.dots:
        dd = (d.x - x) ** 2 + (d.y - y) ** 2
        if dd < bd:
            best, bd = d, dd
    if best is not None and math.sqrt(bd) <= best.r * 2.5:
        return best.zone, best.led
    return None
