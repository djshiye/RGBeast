"""The real QtDBus client against `rgbeastd --session --simulate`. Runs only when RGBEASTD names the
daemon binary and a session bus is available (CI runs it inside dbus-run-session)."""
import json
import os
import subprocess
import tempfile
import time
import unittest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

DAEMON = os.environ.get("RGBEASTD", "")


def wait(cond, ms=5000):
    from PySide6.QtTest import QTest
    end = time.monotonic() + ms / 1000
    while not cond() and time.monotonic() < end:
        QTest.qWait(20)
    return cond()


@unittest.skipUnless(DAEMON and os.environ.get("DBUS_SESSION_BUS_ADDRESS"), "set RGBEASTD and run in a session bus")
class SimulatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from PySide6.QtCore import QCoreApplication
        from rgbeast_gui.client import SERVICE, Client
        from PySide6.QtDBus import QDBusConnection
        cls.app = QCoreApplication.instance() or QCoreApplication([])
        cls.state = tempfile.TemporaryDirectory()
        cls.proc = subprocess.Popen([DAEMON, "--session", "--simulate", "--state-dir", cls.state.name],
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        bus = QDBusConnection.sessionBus()
        if not wait(lambda: bus.interface().isServiceRegistered(SERVICE).value()):
            raise RuntimeError("rgbeastd did not appear on the session bus")
        cls.client = Client(session=True)

    @classmethod
    def tearDownClass(cls):
        cls.proc.terminate()
        cls.proc.wait(5)
        cls.state.cleanup()

    def call(self, method, *args):
        box = {}
        getattr(self.client, method)(*args, lambda v=None: box.setdefault("ok", v),
                                     lambda kind, msg: box.setdefault("err", (kind, msg)))
        self.assertTrue(wait(lambda: box), f"{method} never answered")
        return box

    def test_list_get_set(self):
        devs = self.call("list_devices")["ok"]
        ids = [d["id"] for d in devs]
        self.assertIn("sim:fury", ids)
        state = {"mode": "static", "colors": [{"r": 1, "g": 2, "b": 3}], "brightness": 40, "speed": 50,
                 "direction": "", "random": False, "zones": []}
        applied = self.call("set_state", "sim:fury", state)["ok"]
        self.assertEqual(applied["brightness"], 40)
        self.assertEqual(self.call("get_state", "sim:fury")["ok"]["colors"], [{"r": 1, "g": 2, "b": 3}])

    def test_set_zone_leds_sends_a_uint(self):
        dev = next(d for d in self.call("list_devices")["ok"] if any(z["leds_max"] > z["leds_min"] for z in d["zones"]))
        zone = next(z for z in dev["zones"] if z["leds_max"] > z["leds_min"])
        n = zone["leds_min"] + 7
        info = self.call("set_zone_leds", dev["id"], zone["id"], n)["ok"]
        self.assertEqual(next(z for z in info["zones"] if z["id"] == zone["id"])["leds"], n)

    def test_errors_are_classified(self):
        r = self.call("get_state", "sim:nope")
        self.assertEqual(r["err"][0], "failed")

    def test_properties(self):
        self.assertIs(self.call("get_property", "Simulated")["ok"], True)
        self.assertTrue(self.call("get_property", "Version")["ok"])
        self.assertNotIn("err", self.call("set_restore_on_resume", False))
        self.assertIs(self.call("get_property", "RestoreOnResume")["ok"], False)

    def busctl(self, method, sig, *args):
        """Methods the app does not call (Wheelguard does), through busctl."""
        from rgbeast_gui.client import IFACE, PATH, SERVICE
        return subprocess.run(["busctl", "--user", "--json=short", "call", SERVICE, PATH, IFACE, method, sig,
                               *[str(a) for a in args]], capture_output=True, text=True, timeout=10)

    def test_untested_keyboard_stays_locked_until_turned_on(self):
        kb = next(d for d in self.call("list_devices")["ok"] if d["kind"] == "keyboard")
        self.assertTrue(kb["untested"])
        self.assertTrue(kb["locked"])
        state = {"mode": "static", "colors": [{"r": 1, "g": 2, "b": 3}], "brightness": 40, "speed": 50,
                 "direction": "", "random": False, "zones": []}
        self.assertEqual(self.call("set_state", kb["id"], state)["err"][0], "failed")
        r = self.busctl("SetKeyboardOptions", "ss", kb["id"], '{"profile":1,"layer":1,"debounce_ms":8}')
        self.assertNotEqual(r.returncode, 0, "locked keyboards take no settings either")
        # Fury is not an untested device: nothing to turn on.
        self.assertIn("err", self.call("set_unlocked", "sim:fury", True))

        info = self.call("set_unlocked", kb["id"], True)["ok"]
        self.assertFalse(info.get("locked", False))
        applied = self.call("set_state", kb["id"], state)["ok"]
        self.assertEqual(applied["switches"], {"side-lights": True})
        r = self.busctl("SetKeyboardOptions", "ss", kb["id"], '{"profile":2,"layer":3,"debounce_ms":8}')
        self.assertEqual(r.returncode, 0, r.stderr)
        r = self.busctl("GetKeyboardOptions", "s", kb["id"])
        self.assertEqual(json.loads(json.loads(r.stdout)["data"][0]), {"profile": 2, "layer": 3, "debounce_ms": 8})
        r = self.busctl("SetKeyboardOptions", "ss", kb["id"], '{"profile":2,"layer":3,"debounce_ms":5}')
        self.assertNotEqual(r.returncode, 0)

        self.assertTrue(self.call("set_unlocked", kb["id"], False)["ok"]["locked"])
        self.assertEqual(self.call("set_state", kb["id"], state)["err"][0], "failed")

    def test_discovery_log(self):
        self.assertIsInstance(self.call("discovery_log")["ok"], list)


if __name__ == "__main__":
    unittest.main()
