"""Two painted Qt Quick items: the LED preview (the hero of the editor) and the colour wheel."""
import math
from typing import List, Optional

from PySide6.QtCore import Property, QElapsedTimer, QPointF, QRectF, Qt, QTimer, Signal, Slot
from PySide6.QtGui import (QBrush, QColor, QConicalGradient, QFont, QImage, QLinearGradient, QPainter, QPen,
                           QRadialGradient)
from PySide6.QtQml import QmlElement
from PySide6.QtQuick import QQuickPaintedItem

from . import color as C
from . import effects as E

QML_IMPORT_NAME = "RGBeast.Widgets"
QML_IMPORT_MAJOR_VERSION = 1

FPS_INTERVAL_MS = 33          # at most ~30 fps


def _qc(c: C.Rgb, a: float = 1.0) -> QColor:
    q = QColor(c[0], c[1], c[2])
    q.setAlphaF(max(0.0, min(1.0, a)))
    return q


@QmlElement
class LedPreview(QQuickPaintedItem):
    """Every LED of a device as a glowing lens on a dark surface. Fans are rings of 12, memory sticks
    are bars, everything else is a strip. The selected effect plays as a software approximation at
    most 30 times a second, and never while hidden, paused, or with reduced motion."""

    zonesChanged = Signal()
    effectChanged = Signal()
    animateChanged = Signal()
    contentHeightChanged = Signal()
    labelColorChanged = Signal()
    ledClicked = Signal(int, int)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAntialiasing(True)
        self._zones_raw: list = []
        self._zones: List[dict] = []
        self._effect: dict = {"mode": "static"}
        self._effect_raw: dict = {}
        self._animate = True
        self._frame: Optional[E.Frame] = None
        self._frame_w = -1.0
        self._content_h = 140.0
        self._label = QColor(255, 255, 255, 140)
        self._clock = QElapsedTimer()
        self._timer = QTimer(self, interval=FPS_INTERVAL_MS, timeout=self.update)
        self.widthChanged.connect(self._relayout)
        self.visibleChanged.connect(self._update_animation)

    # ------------------------------------------------------------ properties
    def _get_zones(self):
        return self._zones_raw

    def _set_zones(self, zones):
        zones = list(zones or [])
        if zones == self._zones_raw:
            return
        self._zones_raw = zones
        parsed = []
        for z in zones:
            parsed.append({"name": str(z.get("name", "")), "shape": z.get("shape", "strip"),
                           "row_break": bool(z.get("rowBreak", False)),
                           "colors": [C.from_hex(h) or C.BLACK for h in z.get("colors", [])]})
        layout_changed = [(z["shape"], len(z["colors"]), z["row_break"], z["name"]) for z in parsed] != \
            [(z["shape"], len(z["colors"]), z["row_break"], z["name"]) for z in self._zones]
        self._zones = parsed
        if layout_changed:
            self._frame = None
            self._relayout()
        self.zonesChanged.emit()
        self.update()

    def _get_effect(self):
        return self._effect_raw

    def _set_effect(self, effect):
        effect = dict(effect or {})
        if effect == self._effect_raw:
            return
        self._effect_raw = effect
        self._effect = {
            "mode": str(effect.get("mode", "static")),
            "speed": int(effect.get("speed", 50)),
            "brightness": int(effect.get("brightness", 100)),
            "colors": [C.from_hex(h) or C.BLACK for h in effect.get("colors", [])],
            "reverse": bool(effect.get("reverse", False)),
            "random": bool(effect.get("random", False)),
        }
        self.effectChanged.emit()
        self._update_animation()
        self.update()

    def _set_animate(self, on):
        if bool(on) != self._animate:
            self._animate = bool(on)
            self.animateChanged.emit()
            self._update_animation()
            self.update()

    def _set_label(self, c):
        c = QColor(c)
        if c != self._label:
            self._label = c
            self.labelColorChanged.emit()
            self.update()

    zones = Property("QVariantList", _get_zones, _set_zones, notify=zonesChanged)
    effect = Property("QVariantMap", _get_effect, _set_effect, notify=effectChanged)
    animate = Property(bool, lambda s: s._animate, _set_animate, notify=animateChanged)
    contentHeight = Property(float, lambda s: s._content_h, notify=contentHeightChanged)
    labelColor = Property(QColor, lambda s: s._label, _set_label, notify=labelColorChanged)
    animating = Property(bool, lambda s: s._timer.isActive(), notify=effectChanged)

    # ------------------------------------------------------------ layout and animation
    def _relayout(self):
        w = self.width() if self.width() > 0 else 640.0
        self._frame = E.layout(self._zones, w)
        self._frame_w = w
        if abs(self._frame.height - self._content_h) > 0.5:
            self._content_h = self._frame.height
            self.contentHeightChanged.emit()
        self.update()

    def frame(self) -> E.Frame:
        if self._frame is None or abs(self._frame_w - self.width()) > 0.5:
            self._relayout()
        return self._frame

    def _update_animation(self):
        want = self._animate and self.isVisible() and E.is_animated(self._effect.get("mode", ""))
        if want and not self._timer.isActive():
            self._clock.start()
            self._timer.start()
        elif not want and self._timer.isActive():
            self._timer.stop()

    def _elapsed(self) -> float:
        return self._clock.elapsed() / 1000.0 if self._timer.isActive() else 0.0

    @Slot(float, float)
    def clickAt(self, x, y):
        h = E.hit(self.frame(), x, y)
        if h is not None:
            self.ledClicked.emit(h[0], h[1])

    # ------------------------------------------------------------ painting
    def paint(self, p: QPainter):
        frame = self.frame()
        t = self._elapsed()
        p.setRenderHint(QPainter.Antialiasing, True)

        p.setBrush(Qt.NoBrush)
        p.setPen(QPen(QColor(255, 255, 255, 23), 1.0))
        for x, y, w, h, r in frame.outlines:
            p.drawRoundedRect(QRectF(x + 0.5, y + 0.5, w - 1, h - 1), r, r)

        colors = [E.color_at(self._zones, d, t, self._effect) for d in frame.dots]
        p.setPen(Qt.NoPen)
        # Glow pass, then core pass, so glows never sit on top of cores.
        for d, c in zip(frame.dots, colors):
            if c == C.BLACK:
                continue
            gr = d.r * 3.2
            g = QRadialGradient(QPointF(d.x, d.y), gr)
            g.setColorAt(0.0, _qc(c, 0.55))
            g.setColorAt(0.45, _qc(c, 0.18))
            g.setColorAt(1.0, _qc(c, 0.0))
            p.setBrush(QBrush(g))
            p.drawEllipse(QPointF(d.x, d.y), gr, gr)
        for d, c in zip(frame.dots, colors):
            if c == C.BLACK:
                p.setBrush(QColor(255, 255, 255, 18))
                p.drawEllipse(QPointF(d.x, d.y), d.r, d.r)
                continue
            p.setBrush(_qc(c))
            p.drawEllipse(QPointF(d.x, d.y), d.r, d.r)
            # A small specular so bright colours still read as a lens.
            hr = d.r * 0.45
            hx, hy = d.x - d.r * 0.3, d.y - d.r * 0.3
            g = QRadialGradient(QPointF(hx, hy), hr)
            g.setColorAt(0.0, QColor(255, 255, 255, 178))
            g.setColorAt(1.0, QColor(255, 255, 255, 0))
            p.setBrush(QBrush(g))
            p.drawEllipse(QPointF(hx, hy), hr, hr)

        f = QFont(p.font())
        f.setPixelSize(11)
        f.setWeight(QFont.Medium)
        p.setFont(f)
        p.setPen(self._label)
        fm = p.fontMetrics()
        for x, y, w, text in frame.labels:
            p.drawText(QRectF(x, y, w, 16), Qt.AlignHCenter | Qt.AlignTop,
                       fm.elidedText(text, Qt.ElideRight, int(w)))


@QmlElement
class ColorWheel(QQuickPaintedItem):
    """A hue ring around a saturation/value square. `picked` fires only for the user's own moves, so
    setting `color` from outside never echoes back."""

    colorChanged = Signal()
    picked = Signal(str)

    RING = 22.0
    GAP = 10.0

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAntialiasing(True)
        self._h, self._s, self._v = 210.0, 1.0, 1.0
        self._ring_cache: Optional[QImage] = None
        self._ring_key = None
        self._drag = 0          # 1 ring, 2 square

    def _hex(self) -> str:
        return C.to_hex(C.from_hsv(self._h, self._s, self._v))

    def _set_color(self, text):
        c = C.from_hex(str(text))
        if c is None or C.to_hex(c) == self._hex():
            return
        h, s, v = C.to_hsv(c)
        if v > 0 and s > 0:
            self._h = h
        if v > 0:
            self._s = s
        self._v = v
        self.colorChanged.emit()
        self.update()

    color = Property(str, _hex, _set_color, notify=colorChanged)
    hue = Property(float, lambda s: s._h, notify=colorChanged)

    # ------------------------------------------------------------ geometry
    def _geom(self):
        size = min(self.width(), self.height())
        cx, cy = self.width() / 2.0, self.height() / 2.0
        outer = size / 2.0 - 2.0
        inner = outer - self.RING
        half = (inner - self.GAP) / math.sqrt(2.0)
        return cx, cy, outer, inner, half

    @Slot(float, float, result=bool)
    def press(self, x, y):
        cx, cy, outer, inner, half = self._geom()
        d = math.hypot(x - cx, y - cy)
        if inner - 4 <= d <= outer + 6:
            self._drag = 1
        elif abs(x - cx) <= half + 6 and abs(y - cy) <= half + 6:
            self._drag = 2
        else:
            self._drag = 0
            return False
        self.move(x, y)
        return True

    @Slot(float, float)
    def move(self, x, y):
        cx, cy, outer, inner, half = self._geom()
        if self._drag == 1:
            self._h = math.degrees(math.atan2(-(y - cy), x - cx)) % 360.0
        elif self._drag == 2:
            self._s = min(1.0, max(0.0, (x - (cx - half)) / (2 * half)))
            self._v = min(1.0, max(0.0, 1.0 - (y - (cy - half)) / (2 * half)))
        else:
            return
        self._emit()

    @Slot()
    def release(self):
        self._drag = 0

    @Slot(float, float)
    def nudge(self, dh, dv):
        """Arrow keys: left/right turn the hue, up/down change the brightness."""
        self._h = (self._h + dh) % 360.0
        self._v = min(1.0, max(0.0, self._v + dv / 100.0))
        self._emit()

    def _emit(self):
        self.colorChanged.emit()
        self.update()
        self.picked.emit(self._hex())

    # ------------------------------------------------------------ painting
    def _ring(self, outer, inner) -> QImage:
        dpr = self.window().devicePixelRatio() if self.window() else 1.0
        key = (round(outer, 1), round(inner, 1), dpr)
        if self._ring_cache is not None and self._ring_key == key:
            return self._ring_cache
        side = int(math.ceil(outer * 2 * dpr)) + 2
        img = QImage(side, side, QImage.Format_ARGB32_Premultiplied)
        img.setDevicePixelRatio(dpr)
        img.fill(Qt.transparent)
        p = QPainter(img)
        p.setRenderHint(QPainter.Antialiasing, True)
        c = side / dpr / 2.0
        g = QConicalGradient(QPointF(c, c), 0.0)
        for i in range(13):
            g.setColorAt(i / 12.0, _qc(C.from_hsv(i * 30.0, 1, 1)))
        p.setPen(Qt.NoPen)
        p.setBrush(QBrush(g))
        p.drawEllipse(QPointF(c, c), outer, outer)
        p.setCompositionMode(QPainter.CompositionMode_Clear)
        p.drawEllipse(QPointF(c, c), inner, inner)
        p.end()
        self._ring_cache, self._ring_key = img, key
        return img

    def paint(self, p: QPainter):
        cx, cy, outer, inner, half = self._geom()
        if outer <= 0 or half <= 0:
            return
        p.setRenderHint(QPainter.Antialiasing, True)
        ring = self._ring(outer, inner)
        dpr = ring.devicePixelRatio()
        p.drawImage(QPointF(cx - ring.width() / dpr / 2.0, cy - ring.height() / dpr / 2.0), ring)

        # Saturation/value square: hue, white to the left, black to the bottom.
        sq = QRectF(cx - half, cy - half, 2 * half, 2 * half)
        p.setPen(Qt.NoPen)
        p.setBrush(_qc(C.from_hsv(self._h, 1, 1)))
        p.drawRoundedRect(sq, 9, 9)
        gw = QLinearGradient(sq.topLeft(), sq.topRight())
        gw.setColorAt(0, QColor(255, 255, 255, 255))
        gw.setColorAt(1, QColor(255, 255, 255, 0))
        p.setBrush(QBrush(gw))
        p.drawRoundedRect(sq, 9, 9)
        gb = QLinearGradient(sq.topLeft(), sq.bottomLeft())
        gb.setColorAt(0, QColor(0, 0, 0, 0))
        gb.setColorAt(1, QColor(0, 0, 0, 255))
        p.setBrush(QBrush(gb))
        p.drawRoundedRect(sq, 9, 9)
        p.setBrush(Qt.NoBrush)
        p.setPen(QPen(QColor(255, 255, 255, 40), 1))
        p.drawRoundedRect(sq.adjusted(0.5, 0.5, -0.5, -0.5), 9, 9)

        # Markers: white ring with a soft dark edge, filled with the colour they point at.
        mid = (outer + inner) / 2.0
        a = math.radians(self._h)
        self._marker(p, QPointF(cx + mid * math.cos(a), cy - mid * math.sin(a)), C.from_hsv(self._h, 1, 1), 9.5)
        self._marker(p, QPointF(sq.left() + self._s * sq.width(), sq.top() + (1 - self._v) * sq.height()),
                     C.from_hsv(self._h, self._s, self._v), 8.5)

    @staticmethod
    def _marker(p, pt, c, r):
        p.setPen(Qt.NoPen)
        p.setBrush(QColor(0, 0, 0, 70))
        p.drawEllipse(QPointF(pt.x(), pt.y() + 1), r + 1.5, r + 1.5)
        p.setBrush(QColor(255, 255, 255))
        p.drawEllipse(pt, r, r)
        p.setBrush(_qc(c))
        p.drawEllipse(pt, r - 3, r - 3)


@QmlElement
class ColorStrip(QQuickPaintedItem):
    """A rounded bar showing a run of colours as a smooth gradient (sidebar strips, scene swatches)."""

    colorsChanged = Signal()
    radiusChanged = Signal()
    ringChanged = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAntialiasing(True)
        self._colors: List[str] = []
        self._radius = 2.0
        self._ring = QColor(0, 0, 0, 0)

    def _set_colors(self, colors):
        colors = [str(c) for c in (colors or [])]
        if colors != self._colors:
            self._colors = colors
            self.colorsChanged.emit()
            self.update()

    def _set_radius(self, r):
        if float(r) != self._radius:
            self._radius = float(r)
            self.radiusChanged.emit()
            self.update()

    def _set_ring(self, c):
        c = QColor(c)
        if c != self._ring:
            self._ring = c
            self.ringChanged.emit()
            self.update()

    colors = Property("QVariantList", lambda s: s._colors, _set_colors, notify=colorsChanged)
    radius = Property(float, lambda s: s._radius, _set_radius, notify=radiusChanged)
    ring = Property(QColor, lambda s: s._ring, _set_ring, notify=ringChanged)

    def paint(self, p: QPainter):
        w, h = self.width(), self.height()
        if w <= 0 or h <= 0:
            return
        p.setRenderHint(QPainter.Antialiasing, True)
        cs = [QColor(c) for c in self._colors] or [QColor(0, 0, 0)]
        rect = QRectF(0, 0, w, h)
        r = min(self._radius, h / 2, w / 2)
        if len(cs) == 1:
            p.setBrush(cs[0])
        else:
            # At most ~48 stops: a 120-LED header still reads as a smooth band.
            step = max(1, len(cs) // 48)
            picked = cs[::step]
            g = QLinearGradient(rect.topLeft(), rect.topRight())
            for i, c in enumerate(picked):
                g.setColorAt(i / (len(picked) - 1) if len(picked) > 1 else 0, c)
            p.setBrush(QBrush(g))
        p.setPen(Qt.NoPen)
        p.drawRoundedRect(rect, r, r)
        if self._ring.alpha() > 0:
            p.setBrush(Qt.NoBrush)
            p.setPen(QPen(self._ring, 1))
            p.drawRoundedRect(rect.adjusted(0.5, 0.5, -0.5, -0.5), r, r)
