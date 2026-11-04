"""Pure-Python tests for the colour maths, the device model and the "All Devices" mapping (no Qt)."""
import unittest

from rgbeast_gui import color as C
from rgbeast_gui import model as M


def fury_modes():
    return [M.new_mode("direct", "Direct", "per-led", brightness=True),
            M.new_mode("static", "Static", "mode-colors", 1, 1, brightness=True),
            M.new_mode("breath", "Breath", "mode-colors", 1, 10, speed=True, brightness=True),
            M.new_mode("rainbow", "Rainbow", speed=True, brightness=True, directions=("up", "down"))]


def info(kind="dram", modes=None, zones=None):
    return {"id": "x", "name": "Kingston Fury Beast DDR5 RGB ×2", "vendor": "Kingston", "kind": kind,
            "location": "SMBus", "driver": "fury", "version": "", "can_save": False,
            "zones": zones if zones is not None else [{"id": "slot2", "name": "Slot 2", "leds": 12, "leds_min": 12, "leds_max": 12}],
            "modes": modes if modes is not None else fury_modes()}


class ColorTests(unittest.TestCase):
    def test_hex_roundtrip(self):
        c = (0x12, 0xAB, 0xFF)
        self.assertEqual(C.from_hex(C.to_hex(c)), c)
        self.assertEqual(C.from_hex("#00ff00"), (0, 255, 0))
        self.assertEqual(C.from_hex("  00FF00 "), (0, 255, 0))
        for bad in ("zz0000", "#fff", "", "#1234567", None):
            self.assertIsNone(C.from_hex(bad))

    def test_hsv_roundtrip(self):
        for h, s, v in ((0, 1, 1), (120, 1, 1), (240, 0.5, 0.8), (300, 0.2, 0.3)):
            c = C.from_hsv(h, s, v)
            h2, s2, v2 = C.to_hsv(c)
            self.assertEqual(C.from_hsv(h2, s2, v2), c)

    def test_scaled_matches_rust_integer_maths(self):
        self.assertEqual(C.scaled((255, 128, 1), 50), (127, 64, 0))
        self.assertEqual(C.scaled((10, 10, 10), 250), (10, 10, 10))
        self.assertEqual(C.scaled((10, 10, 10), -5), (0, 0, 0))

    def test_json(self):
        self.assertEqual(C.from_json({"r": 1, "g": 2, "b": 3}), (1, 2, 3))
        self.assertEqual(C.from_json({"r": 999, "g": -1, "b": 3}), (255, 0, 3))
        self.assertEqual(C.from_json("nope"), C.BLACK)


class StateTests(unittest.TestCase):
    def test_serde_defaults(self):
        s = M.state_from_json({"mode": "rainbow"})
        self.assertEqual((s["brightness"], s["speed"], s["colors"], s["zones"]), (100, 50, [], []))
        self.assertEqual(M.state_from_json(None), M.default_state())

    def test_roundtrip(self):
        js = {"mode": "direct", "colors": [{"r": 1, "g": 2, "b": 3}], "brightness": 40, "speed": 10,
              "direction": "up", "random": True, "zones": [{"id": "a", "colors": [{"r": 9, "g": 9, "b": 9}]}]}
        self.assertEqual(M.state_to_json(M.state_from_json(js)), js)

    def test_zone_colors_pad_and_truncate(self):
        s = M.state_from_json({"mode": "direct", "zones": [{"id": "a", "colors": [{"r": 1, "g": 0, "b": 0},
                                                                                   {"r": 2, "g": 0, "b": 0}]}]})
        self.assertEqual(M.zone_colors(s, "a", 4), [(1, 0, 0), (2, 0, 0), (2, 0, 0), (2, 0, 0)])
        self.assertEqual(M.zone_colors(s, "a", 1), [(1, 0, 0)])
        self.assertEqual(M.zone_colors(s, "missing", 2), [(1, 0, 0), (1, 0, 0)])   # primary colour


class PreviewTests(unittest.TestCase):
    def test_mode_colours_spread_over_leds(self):
        s = M.default_state()
        s["mode"], s["colors"] = "breath", [(255, 0, 0), (0, 0, 255)]
        (_, colors), = M.preview_colors(info(), s)
        self.assertEqual(colors[:6], [(255, 0, 0)] * 6)
        self.assertEqual(colors[6:], [(0, 0, 255)] * 6)

    def test_off_is_black_and_effects_are_a_rainbow(self):
        s = M.off_state()
        (_, colors), = M.preview_colors(info(modes=fury_modes() + [M.new_mode("off", "Off")]), s)
        self.assertEqual(set(colors), {C.BLACK})
        s = M.state_from_json({"mode": "rainbow"})
        (_, colors), = M.preview_colors(info(), s)
        self.assertEqual(colors[0], (255, 0, 0))
        self.assertEqual(len(set(colors)), 12)

    def test_shapes(self):
        board = info(kind="motherboard")
        self.assertEqual(M.zone_shape(board, {"id": "argb1", "leds": 36}), M.FANS)
        self.assertEqual(M.zone_shape(board, {"id": "argb1", "leds": 30}), M.STRIP)
        self.assertEqual(M.zone_shape(board, {"id": "argb1", "leds": 0}), M.STRIP)
        self.assertEqual(M.zone_shape(info(), {"id": "slot2", "leds": 12}), M.STICK)

    def test_subtitle_and_icons(self):
        self.assertEqual(M.subtitle(info(), M.state_from_json({"mode": "rainbow"})), "Rainbow · 12 LEDs")
        self.assertEqual(M.device_icon(info()), "memory")
        self.assertEqual(M.device_icon(M.all_devices_info()), "grid")
        self.assertEqual(M.device_icon(info(kind="weird")), "question")

    def test_group_preview_labels(self):
        board = info(kind="motherboard", zones=[
            {"id": "mainboard", "name": "Mainboard", "leds": 3, "leds_min": 3, "leds_max": 3},
            {"id": "argb1", "name": "Addressable Header 1", "leds": 24, "leds_min": 0, "leds_max": 120},
            {"id": "argb3", "name": "Addressable Header 3", "leds": 0, "leds_min": 0, "leds_max": 120}])
        ram = info(zones=[{"id": "slot2", "name": "Slot 2", "leds": 12, "leds_min": 12, "leds_max": 12},
                          {"id": "slot4", "name": "Slot 4", "leds": 12, "leds_min": 12, "leds_max": 12}])
        st = M.static_state((1, 2, 3))
        out = M.group_preview([(board, st), (ram, st)])
        self.assertEqual([z["name"] for z in out], ["Board", "Board · Header 1", "Slot 2", "Slot 4"])
        self.assertEqual([z["row_break"] for z in out], [True, False, True, False])
        self.assertEqual(out[1]["shape"], M.FANS)


class GroupMappingTests(unittest.TestCase):
    """The same cases as the GTK crate's model.rs test."""

    def test_prefers_native_modes(self):
        fury = info(modes=fury_modes())
        g = M.default_state()
        g["mode"] = "breathing"
        self.assertEqual(M.map_group_state(fury, g)["mode"], "breath")
        off = M.map_group_state(fury, M.off_state())
        self.assertEqual((off["mode"], off["brightness"], off["colors"]), ("static", 0, [C.BLACK]))
        ene = info(modes=[M.new_mode("off", "Off"), M.new_mode("static", "Static", "mode-colors", 1, 1)])
        self.assertEqual(M.map_group_state(ene, M.off_state())["mode"], "off")
        g["mode"] = "spectrum-cycle"
        self.assertEqual(M.map_group_state(fury, g)["mode"], "rainbow")

    def test_falls_back_to_static_colour_or_nothing(self):
        sapphire = info(modes=[M.new_mode("static", "Static", "mode-colors", 1, 1)])
        g = M.default_state()
        g["mode"] = "breathing"
        self.assertEqual(M.map_group_state(sapphire, g)["mode"], "static")
        g["colors"] = []
        self.assertIsNone(M.map_group_state(sapphire, g))
        self.assertIsNone(M.map_group_state(info(modes=[]), M.off_state()))

    def test_group_zones_are_dropped(self):
        g = M.static_state((9, 9, 9))
        g["zones"] = [{"id": "slot2", "colors": [(1, 1, 1)]}]
        self.assertEqual(M.map_group_state(info(), g)["zones"], [])
        self.assertEqual(g["zones"], [{"id": "slot2", "colors": [(1, 1, 1)]}])   # the group's own state is untouched



class VersionTests(unittest.TestCase):
    def test_app_version_matches_the_workspace(self):
        import os
        import re
        from rgbeast_gui import __version__
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        with open(os.path.join(root, "Cargo.toml"), encoding="utf-8") as f:
            cargo = re.search(r'^version = "([^"]+)"', f.read(), re.M).group(1)
        self.assertEqual(__version__, cargo)


if __name__ == "__main__":
    unittest.main()
