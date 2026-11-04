"""The editor's behaviour, ported from the GTK device page: modes, colour slots, painting (no Qt)."""
import unittest

from rgbeast_gui import model as M
from rgbeast_gui.editor import Editor

from .test_model import fury_modes, info

ZONES = [{"id": "a", "name": "A", "leds": 3, "leds_min": 3, "leds_max": 3},
         {"id": "b", "name": "B", "leds": 2, "leds_min": 0, "leds_max": 10}]


def editor(state=None, modes=None, zones=ZONES, group=False):
    e = Editor()
    e.load(info(modes=modes or fury_modes() + [M.new_mode("multi", "Multi", "mode-colors", 2, 4),
                                                M.new_mode("bg", "Background", "mode-colors", 2, 11)],
                zones=zones), state or M.static_state((10, 20, 30)), is_group=group)
    return e


class EditorTests(unittest.TestCase):
    def test_view_for_static(self):
        v = editor().view()
        self.assertTrue(v["usesColors"])
        self.assertFalse(v["multi"])
        self.assertFalse(v["hasMotion"])
        self.assertTrue(v["hasBrightness"])
        self.assertEqual(v["color"], "#0A141E")
        self.assertEqual([m["id"] for m in v["modes"]][:2], ["direct", "static"])

    def test_switch_to_multi_fills_minimum_from_palette(self):
        e = editor()
        self.assertTrue(e.select_mode("multi"))
        self.assertEqual(e.state["colors"], [(10, 20, 30), M.PALETTE[1]])
        v = e.view()
        self.assertTrue(v["multi"] and v["canAddSlot"] and not v["canRemoveSlot"])
        self.assertEqual([s["label"] for s in v["slots"]], ["Colour 1", "Colour 2"])
        self.assertFalse(e.select_mode("multi"))      # same mode: no change

    def test_background_colour_label(self):
        e = editor()
        e.select_mode("bg")
        self.assertEqual(e.view()["slots"][-1]["label"], "Background colour")

    def test_slots_add_remove_and_pick(self):
        e = editor()
        e.select_mode("multi")
        self.assertTrue(e.add_slot())
        self.assertEqual(e.selected_slot, 2)
        self.assertTrue(e.pick_color((1, 1, 1)))
        self.assertEqual(e.state["colors"][2], (1, 1, 1))
        self.assertTrue(e.add_slot())
        self.assertFalse(e.add_slot())                # max 4
        self.assertTrue(e.remove_slot())
        self.assertTrue(e.remove_slot())
        self.assertFalse(e.remove_slot())             # min 2
        self.assertEqual(len(e.state["colors"]), 2)

    def test_colourless_mode_clears_colours_and_sets_direction(self):
        e = editor()
        e.select_mode("rainbow")
        self.assertEqual(e.state["colors"], [])
        self.assertEqual(e.state["direction"], "up")
        v = e.view()
        self.assertEqual(v["directions"], ["Up", "Down"])
        self.assertTrue(v["hasMotion"])
        self.assertTrue(e.set_direction(1))
        self.assertEqual(e.state["direction"], "down")
        self.assertFalse(e.set_direction(5))

    def test_per_led_paint_and_fill(self):
        e = editor()
        e.select_mode("direct")
        self.assertEqual(e.state["zones"], [{"id": "a", "colors": [(10, 20, 30)] * 3},
                                            {"id": "b", "colors": [(10, 20, 30)] * 2}])
        e.pick_color((255, 0, 0))
        self.assertEqual(e.state["zones"][0]["colors"], [(255, 0, 0)] * 3)   # unpainted: the colour fills
        self.assertTrue(e.paint_led(1, 1))
        e.pick_color((0, 255, 0))
        self.assertTrue(e.paint_led(0, 0))
        self.assertEqual(e.state["zones"][0]["colors"], [(0, 255, 0), (255, 0, 0), (255, 0, 0)])
        self.assertEqual(e.state["zones"][1]["colors"], [(255, 0, 0), (255, 0, 0)])
        self.assertFalse(e.paint_led(9, 0))
        self.assertTrue(e.fill_all())
        self.assertEqual(e.state["zones"][1]["colors"], [(0, 255, 0)] * 2)

    def test_group_cannot_paint(self):
        e = editor(group=True)
        e.select_mode("direct")
        self.assertFalse(e.paint_led(0, 0))
        self.assertFalse(e.view()["paintBar"])

    def test_sliders_clamp(self):
        e = editor()
        self.assertTrue(e.set_brightness(50))
        self.assertTrue(e.set_brightness(250))
        self.assertEqual(e.state["brightness"], 100)
        self.assertFalse(e.set_brightness(100))
        self.assertTrue(e.set_speed(-3))
        self.assertEqual(e.state["speed"], 0)
        self.assertFalse(e.set_speed(0))
        self.assertTrue(e.set_random(True))
        self.assertFalse(e.set_random(True))

    def test_adopt_keeps_slot_in_range(self):
        e = editor()
        e.select_mode("multi")
        e.add_slot()
        e.adopt(M.static_state((5, 5, 5)))
        self.assertEqual(e.selected_slot, 0)
        self.assertEqual(e.view()["color"], "#050505")


if __name__ == "__main__":
    unittest.main()
