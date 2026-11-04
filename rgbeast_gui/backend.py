"""QObject backends exposed to QML: the desktop appearance, Liquid Glass tokens, and `Lighting`, which
owns the D-Bus client, the device list, the editor and the scenes."""
import copy
import json
import os
from typing import Dict, List, Optional

from PySide6.QtCore import Property, QObject, QProcess, QTimer, Signal, Slot

from . import color as C
from . import desktop
from . import model as M
from .client import DENIED, FAILED, UNAVAILABLE, Client, friendly
from .editor import Editor
from .store import SceneStore, Settings

APPLY_DEBOUNCE_MS = 70
RESIZE_DEBOUNCE_MS = 400
RELOAD_DEBOUNCE_MS = 300
LIGHTS_OFF_SWATCH = "#241F31"


class Appearance(QObject):
    """GNOME's colour scheme, accent colour, animation switch and window button layout, followed live
    through `gsettings monitor`. `--theme` pins light or dark (screenshots)."""

    changed = Signal()

    def __init__(self, override: Optional[bool] = None, parent=None):
        super().__init__(parent)
        self._override = override
        self._dark = override if override is not None else (
            desktop.prefers_dark(desktop.get(desktop.INTERFACE, "color-scheme")) if desktop.available() else True)
        self._accent = desktop.accent(desktop.get(desktop.INTERFACE, "accent-color"))
        self._animate = desktop.animations(desktop.get(desktop.INTERFACE, "enable-animations"))
        self._left, self._right = desktop.button_layout(desktop.get(desktop.WM, "button-layout"))
        self._procs: List[QProcess] = []

    @Slot()
    def watch(self):
        if not desktop.available() or self._procs:
            return
        for schema in (desktop.INTERFACE, desktop.WM):
            p = QProcess(self)
            p.readyReadStandardOutput.connect(lambda p=p: self._on_output(p))
            p.start("gsettings", ["monitor", schema])
            self._procs.append(p)

    def _on_output(self, proc):
        text = bytes(proc.readAllStandardOutput()).decode(errors="replace")
        for line in text.splitlines():
            self.apply(line)

    def apply(self, line: str):
        kv = desktop.parse_monitor_line(line)
        if kv is None:
            return
        key, value = kv
        before = (self._dark, self._accent, self._animate, self._left, self._right)
        if key == "color-scheme" and self._override is None:
            self._dark = desktop.prefers_dark(value)
        elif key == "accent-color":
            self._accent = desktop.accent(value)
        elif key == "enable-animations":
            self._animate = desktop.animations(value)
        elif key == "button-layout":
            self._left, self._right = desktop.button_layout(value)
        if before != (self._dark, self._accent, self._animate, self._left, self._right):
            self.changed.emit()

    def shutdown(self):
        for p in self._procs:
            p.kill()
            p.waitForFinished(500)

    dark = Property(bool, lambda s: s._dark, notify=changed)
    accent = Property(str, lambda s: s._accent, notify=changed)
    reduceMotion = Property(bool, lambda s: not s._animate, notify=changed)
    buttonsLeft = Property("QVariantList", lambda s: s._left, notify=changed)
    buttonsRight = Property("QVariantList", lambda s: s._right, notify=changed)


class LiquidGlass(QObject):
    """Colours from the Liquid Glass theme (github.com/djshiye/fedora-liquid-glass), when it is in use.

    Liquid Glass writes its tokens to ~/.config/liquid-glass/tokens-{dark,light}.json on every apply.
    While those files exist the app takes its surfaces, text and accent from them, like Wheelguard; the
    folder is watched, so a change shows up immediately.
    """

    changed = Signal()

    # Theme.qml role -> Liquid Glass token
    ROLES = {
        "bg": "window", "surface": "card", "surface2": "active", "surface3": "hover", "sunken": "view",
        "hairline": "edge", "hairlineStrong": "highlight",
        "text": "fg", "text2": "fg_dim", "text3": "fg_faint",
        "accent": "accent", "accentText": "accent_text",
        "danger": "destructive", "warning": "warning", "ok": "success",
    }

    def __init__(self, directory: Optional[str] = None, parent=None):
        super().__init__(parent)
        base = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
        self._dir = directory or os.path.join(base, "liquid-glass")
        self._tokens: dict = {}
        self._watcher = None
        self._debounce = QTimer(self, singleShot=True, interval=150, timeout=self.reload)
        self.reload()

    @Slot()
    def watch(self):
        from PySide6.QtCore import QFileSystemWatcher
        if self._watcher is None:
            self._watcher = QFileSystemWatcher(self)
            self._watcher.directoryChanged.connect(lambda _p: self._debounce.start())
            self._watcher.fileChanged.connect(lambda _p: self._debounce.start())
        for p in (os.path.dirname(self._dir), self._dir, *self._files()):
            if os.path.exists(p) and p not in self._watcher.directories() + self._watcher.files():
                self._watcher.addPath(p)

    def _files(self):
        return [os.path.join(self._dir, f"tokens-{s}.json") for s in ("dark", "light")]

    @Slot()
    def reload(self):
        tokens = {}
        for scheme, path in zip(("dark", "light"), self._files()):
            try:
                with open(path, encoding="utf-8") as f:
                    data = json.load(f)
            except (OSError, ValueError):
                continue
            if isinstance(data, dict) and data.get("version") == 1:
                tokens[scheme] = data
        if tokens != self._tokens:
            self._tokens = tokens
            self.changed.emit()
        if self._watcher is not None:
            self.watch()            # files that were replaced need re-adding

    @Slot(bool, str, str, result=str)
    def color(self, dark: bool, role: str, fallback: str) -> str:
        """The token colour for a Theme.qml role, or the fallback ("#RRGGBBAA" becomes Qt's "#AARRGGBB")."""
        value = self._tokens.get("dark" if dark else "light", {}).get(self.ROLES.get(role, ""))
        if not (isinstance(value, str) and value.startswith("#") and len(value) in (7, 9)):
            return fallback
        return value if len(value) == 7 else "#" + value[7:9] + value[1:7]

    @Slot(bool, str, int, result=int)
    def number(self, dark: bool, name: str, fallback: int) -> int:
        value = self._tokens.get("dark" if dark else "light", {}).get(name)
        return int(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else fallback

    active = Property(bool, lambda s: bool(s._tokens), notify=changed)


def _hexes(colors) -> List[str]:
    return [C.to_hex(c) for c in colors]


class Lighting(QObject):
    """Everything the window shows, and every change it makes through the daemon."""

    statusChanged = Signal()
    devicesChanged = Signal()
    currentChanged = Signal()
    editorChanged = Signal()
    previewChanged = Signal()
    scenesChanged = Signal()
    prefsChanged = Signal()
    applied = Signal()
    toast = Signal(str)

    def __init__(self, client: Client, settings: Settings, scenes: SceneStore, parent=None):
        super().__init__(parent)
        self.client = client
        self.settings = settings
        self.scenes = scenes
        self.editor = Editor()
        self.group_info = M.all_devices_info()
        self.group_state = M.static_state((0x00, 0x80, 0xFF))
        self.infos: Dict[str, dict] = {}        # id -> DeviceInfo, in daemon order
        self.states: Dict[str, dict] = {}
        self.current = ""
        self.debug_device = ""
        self._status = "loading"
        self._status_text = "Connecting…"
        self._simulated = False
        self._version = ""
        self._restore = True
        self._log: List[str] = []
        self._loading = False
        self._seq: Dict[str, int] = {}          # device id -> sequence of the last set_state sent
        self._inflight: Dict[str, int] = {}

        self._apply_timer = QTimer(self, singleShot=True, interval=APPLY_DEBOUNCE_MS, timeout=self._apply_now)
        self._reload_timer = QTimer(self, singleShot=True, interval=RELOAD_DEBOUNCE_MS, timeout=self.reload)
        self._resize_timers: Dict[str, QTimer] = {}
        self._resize_pending: Dict[str, tuple] = {}

        client.stateChanged.connect(self._on_state_signal)
        client.devicesChanged.connect(lambda: self._reload_timer.start())
        client.serviceAppeared.connect(self._on_service_appeared)
        client.serviceVanished.connect(self._on_service_vanished)

    # ================================================================ connection and loading
    @Slot()
    def start(self):
        self._set_status("loading", "Connecting…")
        self.reload()

    @Slot()
    def retry(self):
        self.start()

    def _on_service_appeared(self):
        if self._status in ("disconnected", "loading"):
            self.start()

    def _on_service_vanished(self):
        self._disconnected()

    def _disconnected(self):
        self._set_status("disconnected", "Lighting service not running")

    @Slot()
    def reload(self):
        if self._loading:
            self._reload_timer.start()
            return
        self._loading = True
        self.client.list_devices(self._on_list, self._on_load_error)
        self.client.get_property("Simulated", self._on_simulated, lambda *_: None)
        self.client.get_property("Version", lambda v: self._set_version(str(v or "")), lambda *_: None)

    def _on_simulated(self, v):
        self._simulated = bool(v)
        self._refresh_status_text()

    def _set_version(self, v):
        if v != self._version:
            self._version = v
            self.prefsChanged.emit()

    def _on_load_error(self, kind, message):
        self._loading = False
        if kind == UNAVAILABLE:
            self._disconnected()
        else:
            self._set_status(self._status if self._status != "loading" else "disconnected",
                             "Lighting service not answering")
            self.toast.emit(friendly(kind, message))

    def _on_list(self, infos: List[dict]):
        if not infos:
            self._loading = False
            self.infos, self.states = {}, {}
            self._after_load()
            return
        pending = {"n": len(infos)}
        states: Dict[str, dict] = {}

        def got(dev_id, state_json):
            states[dev_id] = M.state_from_json(state_json) if state_json is not None else M.default_state()
            pending["n"] -= 1
            if pending["n"] == 0:
                self._loading = False
                self.infos = {i["id"]: i for i in infos}
                self.states = {i["id"]: states.get(i["id"], M.default_state()) for i in infos}
                self._after_load()

        for info in infos:
            dev_id = info["id"]
            self.client.get_state(dev_id, lambda s, d=dev_id: got(d, s), lambda *_a, d=dev_id: got(d, None))

    def _after_load(self):
        n = len(self.infos)
        self._refresh_status_text()
        self.devicesChanged.emit()
        self.prefsChanged.emit()
        if n == 0:
            self._set_status("empty", self._status_text)
            return
        was_ready = self._status == "ready"
        self._set_status("ready", self._status_text)
        if was_ready and (self.current == M.ALL_ID or self.current in self.infos):
            # A rescan or hot-plug: keep the page, refresh what it shows.
            if self.current == M.ALL_ID:
                self.previewChanged.emit()
            else:
                self.editor.set_info(self.infos[self.current])
                if not self._editing(self.current):
                    self.editor.adopt(self.states[self.current])
                self._editor_changed()
            return
        last = self.debug_device or self.settings.get("last-device")
        if last in self.infos or last == M.ALL_ID:
            target = last
        else:
            target = M.ALL_ID if n > 1 else next(iter(self.infos))
        self._show(target)

    def _refresh_status_text(self):
        if self._status == "disconnected":
            return
        n = len(self.infos)
        text = f"{n} device{'' if n == 1 else 's'}"
        if self._simulated:
            text += " · simulated"
        self._status_text = text
        self.statusChanged.emit()

    def _set_status(self, status, text):
        if (status, text) != (self._status, self._status_text):
            self._status, self._status_text = status, text
            self.statusChanged.emit()

    def _report(self, kind, message):
        if kind == UNAVAILABLE:
            self._disconnected()
        self.toast.emit(friendly(kind, message))

    # ================================================================ selection
    @Slot(str)
    def selectDevice(self, dev_id: str):
        if dev_id != M.ALL_ID and dev_id not in self.infos:
            return
        if self._apply_timer.isActive():
            self._apply_timer.stop()
            self._apply_now()           # never lose the last edit
        self._show(dev_id)

    def _show(self, dev_id):
        self.current = dev_id
        if dev_id == M.ALL_ID:
            self.editor.load(self.group_info, self.group_state, is_group=True)
        else:
            self.editor.load(self.infos[dev_id], self.states[dev_id])
        self.settings.set("last-device", dev_id)
        self.currentChanged.emit()
        self.devicesChanged.emit()
        self._editor_changed()

    def _editing(self, dev_id) -> bool:
        return dev_id == self.current and (self._apply_timer.isActive() or self._inflight.get(dev_id, 0) > 0)

    # ================================================================ edits from the page
    def _edited(self, changed: bool):
        if not changed:
            return
        self._editor_changed()
        self._apply_timer.start()

    def _editor_changed(self):
        self.editorChanged.emit()
        self.previewChanged.emit()

    @Slot(str)
    def selectMode(self, mode_id):
        self._edited(self.editor.select_mode(mode_id))

    @Slot(int)
    def selectSlot(self, i):
        if self.editor.select_slot(i):
            self.editorChanged.emit()

    @Slot(str, result=bool)
    def pickColor(self, text):
        """A colour from the wheel, a swatch or the hex field. False for text that is not a colour."""
        c = C.from_hex(text)
        if c is None:
            return False
        self._edited(self.editor.pick_color(c))
        return True

    @Slot()
    def fillAll(self):
        self._edited(self.editor.fill_all())

    @Slot(int, int)
    def paintLed(self, zone, led):
        self._edited(self.editor.paint_led(zone, led))

    @Slot()
    def addSlot(self):
        self._edited(self.editor.add_slot())

    @Slot()
    def removeSlot(self):
        self._edited(self.editor.remove_slot())

    @Slot(int)
    def setBrightness(self, v):
        self._edited(self.editor.set_brightness(v))

    @Slot(int)
    def setSpeed(self, v):
        self._edited(self.editor.set_speed(v))

    @Slot(int)
    def setDirection(self, i):
        self._edited(self.editor.set_direction(i))

    @Slot(bool)
    def setRandom(self, on):
        self._edited(self.editor.set_random(on))

    # ================================================================ talking to the daemon
    def _apply_now(self):
        state = copy.deepcopy(self.editor.state)
        if self.editor.is_group:
            self.group_state = state
            self._apply_to_all(state)
        elif self.current in self.infos:
            self._apply_state(self.current, state, adopt=True, feedback=True)

    def _apply_state(self, dev_id, state, adopt, feedback):
        seq = self._seq.get(dev_id, 0) + 1
        self._seq[dev_id] = seq
        self._inflight[dev_id] = self._inflight.get(dev_id, 0) + 1

        def ok(applied_json):
            self._inflight[dev_id] = max(0, self._inflight.get(dev_id, 1) - 1)
            applied = M.state_from_json(applied_json)
            self.states[dev_id] = applied
            self.devicesChanged.emit()
            # Adopt only the answer to the newest request, and only while nothing newer is waiting,
            # so a drag never jumps back to an older value.
            if adopt and dev_id == self.current and seq == self._seq.get(dev_id) and not self._editing(dev_id):
                if applied != self.editor.state:
                    self.editor.adopt(applied)
                    self._editor_changed()
            if self.current == M.ALL_ID:
                self.previewChanged.emit()
            if feedback:
                self.applied.emit()

        def err(kind, message):
            self._inflight[dev_id] = max(0, self._inflight.get(dev_id, 1) - 1)
            self._report(kind, message)

        self.client.set_state(dev_id, M.state_to_json(state), ok, err)

    def _apply_to_all(self, group):
        for dev_id, info in self.infos.items():
            mapped = M.map_group_state(info, group)
            if mapped is not None:
                self._apply_state(dev_id, mapped, adopt=False, feedback=False)
        self.applied.emit()

    def _on_state_signal(self, dev_id, state_json):
        if dev_id not in self.infos:
            return
        try:
            state = M.state_from_json(json.loads(state_json))
        except ValueError:
            return
        self.states[dev_id] = state
        self.devicesChanged.emit()
        if dev_id == self.current and not self._editing(dev_id) and state != self.editor.state:
            self.editor.adopt(state)
            self._editor_changed()
        elif self.current == M.ALL_ID:
            self.previewChanged.emit()

    @Slot(str, str, int)
    def resizeZone(self, dev_id, zone_id, leds):
        """LED count of an addressable header, sent once the spinner rests for 400 ms."""
        key = f"{dev_id}\n{zone_id}"
        self._resize_pending[key] = (dev_id, zone_id, int(leds))
        t = self._resize_timers.get(key)
        if t is None:
            t = QTimer(self, singleShot=True, interval=RESIZE_DEBOUNCE_MS)
            t.timeout.connect(lambda k=key: self._resize_now(*self._resize_pending.pop(k)))
            self._resize_timers[key] = t
        t.start()

    def _resize_now(self, dev_id, zone_id, leds):
        def ok(info):
            if dev_id not in self.infos:
                return
            self.infos[dev_id] = info
            if dev_id == self.current:
                self.editor.set_info(info)
            self.devicesChanged.emit()
            self.prefsChanged.emit()
            self.client.get_state(dev_id, lambda s: self._on_state_signal(dev_id, json.dumps(s)), lambda *_: None)
            self._editor_changed()

        self.client.set_zone_leds(dev_id, zone_id, leds, ok, self._report)

    @Slot()
    def storeOnDevice(self):
        if self.current not in self.infos:
            return
        self.client.save_to_device(self.current, lambda _v: self.toast.emit("Stored as the power-on lighting"),
                                   self._report)

    @Slot()
    def rescan(self):
        if self._status == "disconnected":
            self.start()
            return
        self._set_status(self._status, "Scanning…")

        def done(_v=None):
            self.reload()

        def err(kind, message):
            self._report(kind, message)
            self.reload()

        self.client.rescan(done, err)

    @Slot()
    def lightsOff(self):
        self.group_state = M.off_state()
        if self.current == M.ALL_ID:
            self.editor.load(self.group_info, self.group_state, is_group=True)
            self._editor_changed()
        self._apply_to_all(self.group_state)

    # ================================================================ scenes
    @Slot(str)
    def applyScene(self, name):
        scene = self.scenes.get(name)
        if scene is None:
            return
        for dev_id in self.infos:
            if dev_id in scene["states"]:
                self._apply_state(dev_id, scene["states"][dev_id], adopt=True, feedback=False)
        self.applied.emit()
        self.toast.emit(f"Scene “{name}” applied")

    @Slot(str, result=bool)
    def saveScene(self, name):
        name = name.strip()
        if not name:
            return False
        self.scenes.add(name, {d: copy.deepcopy(s) for d, s in self.states.items()})
        self.scenesChanged.emit()
        self.toast.emit(f"Scene “{name}” saved")
        return True

    @Slot(str, str, result=bool)
    def renameScene(self, old, new):
        if self.scenes.rename(old, new):
            self.scenesChanged.emit()
            return True
        return False

    @Slot(str)
    def deleteScene(self, name):
        self.scenes.remove(name)
        self.scenesChanged.emit()

    @Slot(str, result=bool)
    def sceneExists(self, name):
        return self.scenes.get(name.strip()) is not None

    # ================================================================ preferences
    @Slot()
    def loadPreferences(self):
        self.client.get_property("Version", lambda v: self._set_version(str(v or "")), lambda *_: None)
        self.client.get_property("RestoreOnResume", self._on_restore, lambda *_: None)
        self.client.discovery_log(self._on_log, lambda *_: None)

    def _on_restore(self, v):
        self._restore = bool(v)
        self.prefsChanged.emit()

    def _on_log(self, lines):
        self._log = list(lines)
        self.prefsChanged.emit()

    @Slot(bool)
    def setRestoreOnResume(self, on):
        prev = self._restore
        self._restore = bool(on)
        self.prefsChanged.emit()

        def err(kind, message):
            self._restore = prev
            self.prefsChanged.emit()
            self._report(kind, message)

        self.client.set_restore_on_resume(on, lambda _v: None, err)

    @Slot(bool)
    def setAnimate(self, on):
        if self.settings.set("animate-preview", bool(on)):
            self.prefsChanged.emit()

    # ================================================================ properties
    def _device_rows(self):
        rows = [{"id": M.ALL_ID, "name": "All Devices", "subtitle": "Set everything at once", "icon": "grid",
                 "colors": [], "selected": self.current == M.ALL_ID}]
        for dev_id, info in self.infos.items():
            s = self.states.get(dev_id, M.default_state())
            rows.append({"id": dev_id, "name": info.get("name", dev_id), "subtitle": M.subtitle(info, s),
                         "icon": M.device_icon(info), "colors": _hexes(M.strip_colors(info, s)),
                         "selected": self.current == dev_id})
        return rows

    def _scene_rows(self):
        return [{"name": s["name"], "colors": _hexes(SceneStore.colors(s))} for s in self.scenes.scenes]

    def _current_info(self):
        if self.current == M.ALL_ID:
            return self.group_info
        return self.infos.get(self.current, {})

    def _title(self):
        return self._current_info().get("name", "") if self.current else ""

    def _subtitle(self):
        if self.current == M.ALL_ID:
            n = len(self.infos)
            return f"{n} device{'' if n == 1 else 's'}, closest effect on each"
        info = self._current_info()
        parts = [p for p in (info.get("vendor"), info.get("location")) if p]
        return " · ".join(parts)

    def _editor_view(self):
        v = self.editor.view()
        zones = []
        for z in self.editor.info.get("zones", []):
            zones.append({"id": z["id"], "name": z.get("name", z["id"]), "leds": int(z.get("leds", 0)),
                          "min": int(z.get("leds_min", 0)), "max": int(z.get("leds_max", 0)),
                          "sizable": M.is_sizable(z)})
        v["zones"] = zones
        v["showZones"] = len(zones) > 1 or any(z["sizable"] for z in zones)
        return v

    def _preview(self):
        if self.editor.is_group:
            layout = M.group_preview([(self.infos[d], self.states[d]) for d in self.infos])
        else:
            layout = self.editor.preview()
        return [{"name": z["name"], "shape": z["shape"], "colors": _hexes(z["colors"]), "rowBreak": z["row_break"]}
                for z in layout]

    def _effect(self):
        st = self.editor.state
        if self.editor.is_group:
            # The group preview shows each device's own state.
            return {"mode": "static", "speed": 50, "brightness": 100, "colors": [], "reverse": False, "random": False}
        return {"mode": st["mode"], "speed": st["speed"], "brightness": st["brightness"],
                "colors": _hexes(st["colors"]), "reverse": st["direction"] in ("reverse", "down"),
                "random": st["random"]}

    def _header_zones(self):
        out = []
        for dev_id, info in self.infos.items():
            for z in info.get("zones", []):
                if M.is_sizable(z):
                    out.append({"device": dev_id, "zone": z["id"], "title": f"{info.get('name', '')} · {z.get('name', '')}",
                                "leds": int(z.get("leds", 0)), "min": int(z.get("leds_min", 0)),
                                "max": int(z.get("leds_max", 0))})
        return out

    status = Property(str, lambda s: s._status, notify=statusChanged)
    statusText = Property(str, lambda s: s._status_text, notify=statusChanged)
    simulated = Property(bool, lambda s: s._simulated, notify=statusChanged)
    devices = Property("QVariantList", _device_rows, notify=devicesChanged)
    scenesList = Property("QVariantList", _scene_rows, notify=scenesChanged)
    currentId = Property(str, lambda s: s.current, notify=currentChanged)
    isGroup = Property(bool, lambda s: s.current == M.ALL_ID, notify=currentChanged)
    title = Property(str, _title, notify=currentChanged)
    subtitle = Property(str, _subtitle, notify=devicesChanged)
    canSave = Property(bool, lambda s: bool(s._current_info().get("can_save")), notify=currentChanged)
    page = Property("QVariantMap", _editor_view, notify=editorChanged)
    preview = Property("QVariantList", _preview, notify=previewChanged)
    effect = Property("QVariantMap", _effect, notify=previewChanged)
    animate = Property(bool, lambda s: bool(s.settings.get("animate-preview")), notify=prefsChanged)
    version = Property(str, lambda s: (f"{s._version} (simulated devices)" if s._simulated else s._version) or "…",
                       notify=prefsChanged)
    restoreOnResume = Property(bool, lambda s: s._restore, notify=prefsChanged)
    discoveryLog = Property("QVariantList", lambda s: s._log, notify=prefsChanged)
    headerZones = Property("QVariantList", _header_zones, notify=prefsChanged)
    lightsOffColor = Property(str, lambda s: LIGHTS_OFF_SWATCH, constant=True)
    palette = Property("QVariantList", lambda s: _hexes(M.PALETTE), constant=True)
