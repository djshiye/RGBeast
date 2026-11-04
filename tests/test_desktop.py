"""Reading GNOME's appearance settings (no gsettings needed)."""
import unittest

from rgbeast_gui import desktop as D


class DesktopTests(unittest.TestCase):
    def test_dark_and_accent(self):
        self.assertTrue(D.prefers_dark("prefer-dark"))
        self.assertFalse(D.prefers_dark("default"))
        self.assertFalse(D.prefers_dark(None))
        self.assertEqual(D.accent("'purple'"), "#9141ac")
        self.assertEqual(D.accent("nonsense"), "#3584e4")
        self.assertEqual(D.accent(None), "#3584e4")

    def test_animations(self):
        self.assertFalse(D.animations("false"))
        self.assertTrue(D.animations("true"))
        self.assertTrue(D.animations(None))

    def test_button_layout(self):
        self.assertEqual(D.button_layout("appmenu:minimize,maximize,close"), ([], ["minimize", "maximize", "close"]))
        self.assertEqual(D.button_layout("'close,minimize:appmenu'"), (["close", "minimize"], []))
        self.assertEqual(D.button_layout(None), ([], ["close"]))
        # A layout without close still gets one.
        self.assertEqual(D.button_layout("menu:spacer,bogus,maximize,maximize"), ([], ["maximize", "close"]))

    def test_monitor_line(self):
        self.assertEqual(D.parse_monitor_line("color-scheme: 'prefer-dark'"), ("color-scheme", "prefer-dark"))
        self.assertIsNone(D.parse_monitor_line("garbage"))


if __name__ == "__main__":
    unittest.main()
