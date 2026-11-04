"""scenes.json and settings.json: round trips, the GTK app's format, bad files."""
import json
import os
import tempfile
import unittest

from rgbeast_gui import model as M
from rgbeast_gui.store import SceneStore, Settings

# What the GTK app wrote (serde of Vec<Scene>).
GTK_SCENES = [{"name": "Movie", "states": {
    "sim:fury": {"mode": "static", "colors": [{"r": 255, "g": 0, "b": 0}], "brightness": 40, "speed": 50,
                 "direction": "", "random": False, "zones": []}}}]


class SceneTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "rgbeast", "scenes.json")

    def tearDown(self):
        self.dir.cleanup()

    def test_reads_the_gtk_format(self):
        os.makedirs(os.path.dirname(self.path))
        with open(self.path, "w") as f:
            json.dump(GTK_SCENES, f)
        s = SceneStore(self.path)
        movie = s.get("Movie")
        self.assertEqual(movie["states"]["sim:fury"]["brightness"], 40)
        self.assertEqual(SceneStore.colors(movie), [(255, 0, 0)])

    def test_roundtrip_and_overwrite(self):
        s = SceneStore(self.path)
        st = M.static_state((1, 2, 3))
        self.assertTrue(s.add("A", {"d": st}))
        self.assertTrue(s.add("A", {"d": M.off_state()}))
        again = SceneStore(self.path)
        self.assertEqual([x["name"] for x in again.scenes], ["A"])
        self.assertEqual(again.get("A")["states"]["d"], M.off_state())

    def test_rename(self):
        s = SceneStore(self.path)
        s.add("A", {})
        s.add("B", {})
        self.assertFalse(s.rename("A", "  "))
        self.assertFalse(s.rename("missing", "C"))
        self.assertTrue(s.rename("A", " B "))       # replaces the old B
        self.assertEqual([x["name"] for x in s.scenes], ["B"])

    def test_remove(self):
        s = SceneStore(self.path)
        s.add("A", {})
        s.remove("A")
        self.assertEqual(SceneStore(self.path).scenes, [])

    def test_bad_file_is_ignored(self):
        os.makedirs(os.path.dirname(self.path))
        with open(self.path, "w") as f:
            f.write("{not json")
        self.assertEqual(SceneStore(self.path).scenes, [])
        with open(self.path, "w") as f:
            json.dump([{"name": 3}, "x", {"name": "ok", "states": {}}], f)
        self.assertEqual([x["name"] for x in SceneStore(self.path).scenes], ["ok"])

    def test_empty_scene_colour(self):
        self.assertEqual(SceneStore.colors({"name": "x", "states": {}}), [(0, 0, 0)])


class SettingsTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "settings.json")

    def tearDown(self):
        self.dir.cleanup()

    def test_defaults_and_roundtrip(self):
        s = Settings(self.path)
        self.assertEqual(s.get("window-width"), 1120)
        self.assertTrue(s.set("last-device", "sim:fury"))
        self.assertFalse(s.set("last-device", "sim:fury"))
        self.assertEqual(Settings(self.path).get("last-device"), "sim:fury")

    def test_wrong_types_fall_back(self):
        with open(self.path, "w") as f:
            json.dump({"window-width": "wide", "window-height": True, "animate-preview": 0, "last-device": 5}, f)
        s = Settings(self.path)
        self.assertEqual((s.get("window-width"), s.get("window-height")), (1120, 800))
        self.assertIs(s.get("animate-preview"), True)
        self.assertEqual(s.get("last-device"), "")


if __name__ == "__main__":
    unittest.main()
