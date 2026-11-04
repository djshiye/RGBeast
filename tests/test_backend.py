"""Lighting (the QML-facing backend) against a fake client: loading, the 70 ms debounce, adopting only
the newest daemon reply, the All Devices mapping, scenes and errors."""
import copy
import os
import tempfile
import unittest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

try:
    from PySide6.QtCore import QCoreApplication, QObject, Signal
    from PySide6.QtTest import QTest
except ImportError:         # the pure-Python tests still run without Qt
    raise unittest.SkipTest("PySide6 is not installed")

from rgbeast_gui import model as M  # noqa: E402
from rgbeast_gui.backend import Lighting  # noqa: E402
from rgbeast_gui.client import DENIED, UNAVAILABLE  # noqa: E402
from rgbeast_gui.store import SceneStore, Settings  # noqa: E402

from .test_model import info  # noqa: E402

APP = QCoreApplication.instance() or QCoreApplication([])


def board():
    return {"id": "sim:aura", "name": "ASUS Aura", "vendor": "ASUS", "kind": "motherboard", "location": "USB",
            "driver": "aura", "version": "", "can_save": True,
            "zones": [{"id": "mainboard", "name": "Mainboard", "leds": 3, "leds_min": 3, "leds_max": 3},
                      {"id": "argb1", "name": "ARGB 1", "leds": 0, "leds_min": 0, "leds_max": 120}],
            "modes": [M.new_mode("static", "Static", "mode-colors", 1, 1, brightness=True),
                      M.new_mode("off", "Off"),
                      M.new_mode("rainbow", "Rainbow", speed=True, brightness=True)]}


def fury():
    i = info()
    i["id"] = "sim:fury"
    return i


class FakeClient(QObject):
    stateChanged = Signal(str, str)
    devicesChanged = Signal()
    serviceAppeared = Signal()
    serviceVanished = Signal()

    def __init__(self, infos):
        super().__init__()
        self.infos = infos
        self.states = {i["id"]: M.state_to_json(M.static_state((255, 255, 255))) for i in infos}
        self.calls = []
        self.held = []          # set_state replies we have not answered yet
        self.hold = False
        self.fail = None        # (kind, message) for the next write

    def list_devices(self, ok, err=None):
        if self.fail and self.fail[0] == UNAVAILABLE:
            err(*self.fail)
            return
        ok(copy.deepcopy(self.infos))

    def get_state(self, dev_id, ok, err=None):
        ok(copy.deepcopy(self.states[dev_id]))

    def set_state(self, dev_id, state, ok, err=None):
        self.calls.append(("set_state", dev_id, state))
        if self.fail:
            err(*self.fail)
            return
        self.states[dev_id] = state
        if self.hold:
            self.held.append(lambda: ok(copy.deepcopy(state)))
        else:
            ok(copy.deepcopy(state))

    def set_zone_leds(self, dev_id, zone, leds, ok, err=None):
        self.calls.append(("set_zone_leds", dev_id, zone, leds))
        i = next(x for x in self.infos if x["id"] == dev_id)
        next(z for z in i["zones"] if z["id"] == zone)["leds"] = leds
        ok(copy.deepcopy(i))

    def save_to_device(self, dev_id, ok, err=None):
        self.calls.append(("save", dev_id))
        ok(None)

    def rescan(self, ok, err=None):
        ok(None)

    def discovery_log(self, ok, err=None):
        ok(["found sim:aura"])

    def get_property(self, name, ok, err=None):
        ok({"Simulated": True, "Version": "1.1.0", "RestoreOnResume": True}[name])

    def set_restore_on_resume(self, on, ok, err=None):
        if self.fail:
            err(*self.fail)
        else:
            ok(None)


class LightingTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.client = FakeClient([board(), fury()])
        self.settings = Settings(os.path.join(self.dir.name, "settings.json"))
        self.scenes = SceneStore(os.path.join(self.dir.name, "scenes.json"))
        self.l = Lighting(self.client, self.settings, self.scenes)
        self.toasts = []
        self.l.toast.connect(self.toasts.append)
        self.l.start()

    def tearDown(self):
        self.dir.cleanup()

    def sets(self):
        return [c for c in self.client.calls if c[0] == "set_state"]

    def test_loads_and_opens_all_devices(self):
        self.assertEqual(self.l.status, "ready")
        self.assertEqual(self.l.statusText, "2 devices · simulated")
        self.assertEqual(self.l.currentId, M.ALL_ID)
        self.assertEqual([r["id"] for r in self.l.devices], [M.ALL_ID, "sim:aura", "sim:fury"])

    def test_remembers_last_device(self):
        self.l.selectDevice("sim:fury")
        self.assertEqual(self.settings.get("last-device"), "sim:fury")
        other = Lighting(self.client, self.settings, self.scenes)
        other.start()
        self.assertEqual(other.currentId, "sim:fury")

    def test_edits_are_debounced(self):
        self.l.selectDevice("sim:aura")
        for v in (10, 20, 30, 40):
            self.l.setBrightness(v)
        self.assertEqual(self.sets(), [])
        QTest.qWait(150)
        self.assertEqual(len(self.sets()), 1)
        self.assertEqual(self.sets()[0][2]["brightness"], 40)

    def test_switching_device_flushes_the_pending_edit(self):
        self.l.selectDevice("sim:aura")
        self.l.pickColor("#00FF00")
        self.l.selectDevice("sim:fury")
        self.assertEqual(len(self.sets()), 1)
        self.assertEqual(self.sets()[0][1], "sim:aura")

    def test_only_the_newest_reply_is_adopted(self):
        self.l.selectDevice("sim:aura")
        self.client.hold = True
        self.l.setBrightness(30)
        QTest.qWait(120)
        self.l.setBrightness(60)
        QTest.qWait(120)
        self.assertEqual(len(self.client.held), 2)
        self.client.held[1]()       # newest answers first
        self.client.held[0]()       # then the stale one
        self.assertEqual(self.l.page["brightness"], 60)

    def test_external_change_is_adopted(self):
        self.l.selectDevice("sim:aura")
        self.client.stateChanged.emit("sim:aura", '{"mode": "rainbow", "speed": 80}')
        self.assertEqual(self.l.page["mode"], "rainbow")
        self.assertEqual(self.l.page["speed"], 80)

    def test_invalid_hex(self):
        self.l.selectDevice("sim:aura")
        self.assertFalse(self.l.pickColor("#12"))
        self.assertTrue(self.l.pickColor("#123456"))

    def test_all_devices_maps_to_each(self):
        self.l.selectDevice(M.ALL_ID)
        self.l.selectMode("rainbow")
        QTest.qWait(150)
        modes = {c[1]: c[2]["mode"] for c in self.sets()}
        self.assertEqual(modes, {"sim:aura": "rainbow", "sim:fury": "rainbow"})

    def test_lights_off(self):
        self.l.lightsOff()
        by_dev = {c[1]: c[2] for c in self.sets()}
        self.assertEqual(by_dev["sim:aura"]["mode"], "off")
        self.assertEqual((by_dev["sim:fury"]["mode"], by_dev["sim:fury"]["brightness"]), ("static", 0))

    def test_scenes(self):
        self.assertTrue(self.l.saveScene("  Movie "))
        self.assertTrue(self.l.sceneExists("Movie"))
        self.assertEqual(self.l.scenesList[0]["name"], "Movie")
        self.client.calls.clear()
        self.l.applyScene("Movie")
        self.assertEqual({c[1] for c in self.sets()}, {"sim:aura", "sim:fury"})
        self.assertTrue(self.l.renameScene("Movie", "Night"))
        self.l.deleteScene("Night")
        self.assertEqual(self.l.scenesList, [])
        self.assertFalse(self.l.saveScene("   "))

    def test_zone_resize_is_debounced(self):
        self.l.selectDevice("sim:aura")
        for n in (5, 10, 24):
            self.l.resizeZone("sim:aura", "argb1", n)
        QTest.qWait(500)
        resizes = [c for c in self.client.calls if c[0] == "set_zone_leds"]
        self.assertEqual(resizes, [("set_zone_leds", "sim:aura", "argb1", 24)])
        argb = next(z for z in self.l.page["zones"] if z["id"] == "argb1")
        self.assertEqual(argb["leds"], 24)

    def test_denied_write_toasts(self):
        self.l.selectDevice("sim:aura")
        self.client.fail = (DENIED, "")
        self.l.setBrightness(5)
        QTest.qWait(150)
        self.assertIn("Not allowed to change the lighting from this session", self.toasts)
        self.assertEqual(self.l.status, "ready")

    def test_unavailable_disconnects_and_retry_recovers(self):
        self.client.fail = (UNAVAILABLE, "")
        self.l.selectDevice("sim:aura")
        self.l.setBrightness(5)
        QTest.qWait(150)
        self.assertEqual(self.l.status, "disconnected")
        self.client.fail = None
        self.l.retry()
        self.assertEqual(self.l.status, "ready")

    def test_restore_on_resume_reverts_on_error(self):
        self.client.fail = (DENIED, "")
        self.l.setRestoreOnResume(False)
        self.assertTrue(self.l.restoreOnResume)

    def test_store_on_device(self):
        self.l.selectDevice("sim:aura")
        self.l.storeOnDevice()
        self.assertIn(("save", "sim:aura"), self.client.calls)


class EmptyTests(unittest.TestCase):
    def test_no_devices(self):
        with tempfile.TemporaryDirectory() as d:
            l = Lighting(FakeClient([]), Settings(os.path.join(d, "s.json")), SceneStore(os.path.join(d, "x.json")))
            l.start()
            self.assertEqual(l.status, "empty")


if __name__ == "__main__":
    unittest.main()
