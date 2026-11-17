"""The preview layout and click hit-testing (no Qt)."""
import unittest

from rgbeast_gui import effects as E
from rgbeast_gui import model as M

RED = (255, 0, 0)


def zone(name, shape, n):
    return {"name": name, "shape": shape, "colors": [RED] * n, "row_break": False}


class LayoutTests(unittest.TestCase):
    def test_every_led_gets_a_dot(self):
        zones = [zone("Board", M.STRIP, 3), zone("Fans", M.FANS, 36), zone("RAM", M.STICK, 12)]
        f = E.layout(zones, 900)
        self.assertEqual(len(f.dots), 3 + 36 + 12)
        self.assertEqual({(d.zone, d.led) for d in f.dots if d.zone == 1}, {(1, i) for i in range(36)})
        self.assertGreaterEqual(f.height, 140)

    def test_dots_stay_inside_the_width(self):
        f = E.layout([zone("Strip", M.STRIP, 120)], 560)
        for d in f.dots:
            self.assertGreaterEqual(d.x - d.r, 0)
            self.assertLessEqual(d.x + d.r, 560)

    def test_keyboard_keys_are_caps_and_clickable(self):
        keys = [{"label": "Esc", "x": 0, "y": 0, "w": 4, "h": 4}, {"label": "Space", "x": 4, "y": 4, "w": 25, "h": 4}]
        z = dict(zone("Keys", M.KEYBOARD, 2), keys=keys)
        f = E.layout([z], 700)
        self.assertEqual(len(f.dots), 2)
        esc, space = f.dots
        self.assertIsNotNone(esc.rect)
        self.assertGreater(space.rect[2], esc.rect[2] * 5)
        for d in f.dots:
            self.assertLessEqual(d.rect[0] + d.rect[2], 700)
        self.assertEqual(E.hit(f, space.rect[0] + 3, space.rect[1] + 3), (0, 1))
        self.assertLess(esc.t, space.t)

    def test_keyboard_waves_move(self):
        keys = [{"label": "A", "x": i * 4, "y": 0, "w": 4, "h": 4} for i in range(8)]
        z = dict(zone("Keys", M.KEYBOARD, 8), keys=keys)
        f = E.layout([z], 700)
        eff = {"mode": "wave-1", "speed": 50, "brightness": 100, "colors": [RED], "random": False}
        a = [E.color_at([z], d, 0.0, eff) for d in f.dots]
        b = [E.color_at([z], d, 0.7, eff) for d in f.dots]
        self.assertNotEqual(a, b)

    def test_empty_zones_are_skipped(self):
        f = E.layout([zone("Empty", M.STRIP, 0)], 600)
        self.assertEqual(f.dots, [])
        self.assertGreaterEqual(f.height, 140)

    def test_hit(self):
        f = E.layout([zone("Strip", M.STRIP, 10)], 800)
        d = f.dots[4]
        self.assertEqual(E.hit(f, d.x + 1, d.y - 1), (0, 4))
        self.assertIsNone(E.hit(f, -500, -500))

    def test_static_colour_and_brightness(self):
        f = E.layout([zone("Strip", M.STRIP, 2)], 600)
        eff = {"mode": "static", "speed": 50, "brightness": 50, "colors": [], "reverse": False, "random": False}
        self.assertEqual(E.color_at([zone("Strip", M.STRIP, 2)], f.dots[0], 0.0, eff), (127, 0, 0))

    def test_hash_is_stable_and_in_range(self):
        self.assertEqual(E.hash01(3, 7), E.hash01(3, 7))
        for a in range(20):
            self.assertTrue(0.0 <= E.hash01(a, a * 3) < 1.0)


if __name__ == "__main__":
    unittest.main()
