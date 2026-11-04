"""Asynchronous QtDBus client for io.github.djshiye.RGBeast1.Manager. The app never touches hardware:
every change goes through rgbeastd, which asks polkit."""
import json
import os
import threading
from typing import Callable, Optional

from PySide6.QtCore import Q_ARG, Q_RETURN_ARG, QMetaObject, QObject, Qt, Signal, Slot, SLOT
from PySide6.QtDBus import (QDBusConnection, QDBusInterface, QDBusMessage, QDBusPendingCallWatcher,
                            QDBusServiceWatcher, QDBusVariant)

SERVICE = "io.github.djshiye.RGBeast1"
PATH = "/io/github/djshiye/RGBeast1"
IFACE = "io.github.djshiye.RGBeast1.Manager"
PROPS = "org.freedesktop.DBus.Properties"

# Reads answer at once; changes may wait for a polkit password dialog.
READ_TIMEOUT_MS = 10_000
WRITE_TIMEOUT_MS = 120_000

UNAVAILABLE, DENIED, FAILED = "unavailable", "denied", "failed"

_UNAVAILABLE = ("ServiceUnknown", "NameHasNoOwner", "Disconnected", "NoServer", "Spawn.")
_DENIED = ("AccessDenied", "AuthFailed", "InteractiveAuthorizationRequired", "NotAuthorized")


def classify(error_name: str) -> str:
    """D-Bus error name -> unavailable | denied | failed."""
    n = error_name or ""
    if any(k in n for k in _UNAVAILABLE):
        return UNAVAILABLE
    if any(n.endswith(k) for k in _DENIED):
        return DENIED
    return FAILED


def friendly(kind: str, message: str = "") -> str:
    if kind == UNAVAILABLE:
        return "The lighting service is not running"
    if kind == DENIED:
        return "Not allowed to change the lighting from this session"
    msg = (message or "").strip()
    return f"Could not apply: {msg}" if msg else "Could not apply the change"


def use_session_bus(flag: bool = False) -> bool:
    return flag or os.environ.get("RGBEAST_BUS") == "session"


class Client(QObject):
    """Calls take `ok(value)` and `err(kind, message)` callbacks; replies arrive on the Qt event loop."""

    stateChanged = Signal(str, str)     # device id, state JSON
    devicesChanged = Signal()
    serviceAppeared = Signal()
    serviceVanished = Signal()
    _threadDone = Signal(int, str, str, str)

    def __init__(self, session: bool = False, parent=None):
        super().__init__(parent)
        self.session = session
        self.bus = QDBusConnection.sessionBus() if session else QDBusConnection.systemBus()
        self._pending = set()
        self._threads = {}
        self._seq = 0
        self._threadDone.connect(self._on_thread_done)
        self._watcher = QDBusServiceWatcher(SERVICE, self.bus, QDBusServiceWatcher.WatchForOwnerChange, self)
        self._watcher.serviceRegistered.connect(lambda _n: self.serviceAppeared.emit())
        self._watcher.serviceUnregistered.connect(lambda _n: self.serviceVanished.emit())
        if self.bus.isConnected():
            self.bus.connect(SERVICE, PATH, IFACE, "StateChanged", self, SLOT("_on_state_changed(QString,QString)"))
            self.bus.connect(SERVICE, PATH, IFACE, "DevicesChanged", self, SLOT("_on_devices_changed()"))

    @Slot(str, str)
    def _on_state_changed(self, dev_id, state):
        self.stateChanged.emit(dev_id, state)

    @Slot()
    def _on_devices_changed(self):
        self.devicesChanged.emit()

    # ------------------------------------------------------------ plumbing
    def _call(self, method: str, args: list, ok: Callable, err: Optional[Callable], iface: str = IFACE,
              timeout: int = READ_TIMEOUT_MS, parse: Optional[Callable] = None):
        if not self.bus.isConnected():
            if err:
                err(UNAVAILABLE, self.bus.lastError().message() or "no bus")
            return
        msg = QDBusMessage.createMethodCall(SERVICE, PATH, iface, method)
        msg.setArguments(args)
        msg.setInteractiveAuthorizationAllowed(True)
        watcher = QDBusPendingCallWatcher(self.bus.asyncCall(msg, timeout), self)
        self._pending.add(watcher)

        def finished(w):
            self._pending.discard(w)
            reply = w.reply()
            w.deleteLater()
            if reply.type() == QDBusMessage.MessageType.ErrorMessage:
                if err:
                    err(classify(reply.errorName()), reply.errorMessage())
                return
            values = reply.arguments()
            value = values[0] if values else None
            if parse is not None:
                try:
                    value = parse(value)
                except (ValueError, TypeError, KeyError) as e:
                    if err:
                        err(FAILED, f"unexpected answer from the service: {e}")
                    return
            ok(value)

        watcher.finished.connect(finished)

    # ------------------------------------------------------------ the API
    def list_devices(self, ok, err=None):
        self._call("ListDevices", [], ok, err, parse=_json_list)

    def get_state(self, dev_id: str, ok, err=None):
        self._call("GetState", [dev_id], ok, err, parse=_json_dict)

    def set_state(self, dev_id: str, state: dict, ok, err=None):
        self._call("SetState", [dev_id, json.dumps(state, separators=(",", ":"))], ok, err,
                   timeout=WRITE_TIMEOUT_MS, parse=_json_dict)

    def set_zone_leds(self, dev_id: str, zone: str, leds: int, ok, err=None):
        """`SetZoneLeds(ssu)`. PySide cannot put a D-Bus `u` in a message (a Python int goes out as `i`
        or `q`), but QDBusInterface converts arguments to the introspected signature. Its calls block,
        so this one runs on a short-lived thread and reports back on the main loop."""
        self._seq += 1
        key = self._seq
        self._threads[key] = (ok, err)

        def work():
            iface = QDBusInterface(SERVICE, PATH, IFACE, self.bus)
            iface.setTimeout(WRITE_TIMEOUT_MS)
            iface.setInteractiveAuthorizationAllowed(True)
            ret = ""
            if iface.isValid():
                ret = QMetaObject.invokeMethod(iface, "SetZoneLeds", Qt.DirectConnection, Q_RETURN_ARG("QString"),
                                               Q_ARG(str, dev_id), Q_ARG(str, zone), Q_ARG("uint", max(0, int(leds))))
            e = iface.lastError()
            self._threadDone.emit(key, ret or "", e.name() if e.isValid() else "", e.message() if e.isValid() else "")

        threading.Thread(target=work, name="rgbeast-dbus", daemon=True).start()

    @Slot(int, str, str, str)
    def _on_thread_done(self, key, ret, ename, emsg):
        ok, err = self._threads.pop(key, (None, None))
        if ok is None:
            return
        if ename or not ret:
            if err:
                err(classify(ename) if ename else FAILED, emsg)
            return
        try:
            ok(_json_dict(ret))
        except (ValueError, TypeError) as e:
            if err:
                err(FAILED, f"unexpected answer from the service: {e}")

    def save_to_device(self, dev_id: str, ok, err=None):
        self._call("SaveToDevice", [dev_id], ok, err, timeout=WRITE_TIMEOUT_MS)

    def rescan(self, ok, err=None):
        self._call("Rescan", [], ok, err, timeout=WRITE_TIMEOUT_MS, parse=_json_list)

    def discovery_log(self, ok, err=None):
        self._call("DiscoveryLog", [], ok, err, parse=lambda v: [str(x) for x in (v or [])])

    def get_property(self, name: str, ok, err=None):
        self._call("Get", [IFACE, name], ok, err, iface=PROPS,
                   parse=lambda v: v.variant() if hasattr(v, "variant") else v)

    def set_restore_on_resume(self, on: bool, ok, err=None):
        self._call("Set", [IFACE, "RestoreOnResume", QDBusVariant(bool(on))], ok, err, iface=PROPS,
                   timeout=WRITE_TIMEOUT_MS)


def _json_list(v):
    data = json.loads(v)
    if not isinstance(data, list):
        raise ValueError("not a list")
    return [d for d in data if isinstance(d, dict) and isinstance(d.get("id"), str)]


def _json_dict(v):
    data = json.loads(v)
    if not isinstance(data, dict):
        raise ValueError("not an object")
    return data
