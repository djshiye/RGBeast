"""Per-user files in ~/.config/rgbeast/: scenes.json (same format the GTK app wrote) and settings.json
(window size, last device, animate preview; these used to be GSettings keys)."""
import json
import os
import tempfile
from typing import Dict, List, Optional

from . import color as C
from . import model as M


def config_dir() -> str:
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, "rgbeast")


def _read_json(path: str):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def _write_json(path: str, data) -> bool:
    """Atomic: a crash never leaves half a file behind."""
    try:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        fd, tmp = tempfile.mkstemp(prefix=".tmp-", dir=os.path.dirname(path))
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.write("\n")
        os.replace(tmp, path)
        return True
    except OSError:
        try:
            os.unlink(tmp)
        except (OSError, UnboundLocalError):
            pass
        return False


class SceneStore:
    """Scenes: a named snapshot of every device's state."""

    def __init__(self, path: Optional[str] = None):
        self.path = path or os.path.join(config_dir(), "scenes.json")
        self.scenes: List[dict] = []    # {"name": str, "states": {device id: state dict}}
        data = _read_json(self.path)
        if isinstance(data, list):
            for s in data:
                if isinstance(s, dict) and isinstance(s.get("name"), str) and isinstance(s.get("states"), dict):
                    states = {k: M.state_from_json(v) for k, v in s["states"].items() if isinstance(k, str)}
                    self.scenes.append({"name": s["name"], "states": states})

    def save(self) -> bool:
        data = [{"name": s["name"], "states": {k: M.state_to_json(v) for k, v in sorted(s["states"].items())}}
                for s in self.scenes]
        return _write_json(self.path, data)

    def get(self, name: str) -> Optional[dict]:
        return next((s for s in self.scenes if s["name"] == name), None)

    def add(self, name: str, states: Dict[str, dict]) -> bool:
        self.scenes = [s for s in self.scenes if s["name"] != name]
        self.scenes.append({"name": name, "states": states})
        return self.save()

    def remove(self, name: str) -> bool:
        self.scenes = [s for s in self.scenes if s["name"] != name]
        return self.save()

    def rename(self, old: str, new: str) -> bool:
        new = new.strip()
        if not new or old == new:
            return False
        target = self.get(old)
        if target is None:
            return False
        self.scenes = [s for s in self.scenes if s["name"] != new or s is target]
        target["name"] = new
        return self.save()

    @staticmethod
    def colors(scene: dict) -> List[C.Rgb]:
        """Representative colours for the sidebar swatch."""
        out = [M.primary_color(s) for _, s in sorted(scene["states"].items())]
        return out or [C.BLACK]


class Settings:
    DEFAULTS = {"window-width": 1120, "window-height": 800, "window-maximized": False,
                "last-device": "", "animate-preview": True}

    def __init__(self, path: Optional[str] = None):
        self.path = path or os.path.join(config_dir(), "settings.json")
        data = _read_json(self.path)
        self.values = dict(self.DEFAULTS)
        if isinstance(data, dict):
            for k, default in self.DEFAULTS.items():
                v = data.get(k)
                if isinstance(v, type(default)) and not (isinstance(v, bool) and not isinstance(default, bool)):
                    self.values[k] = v

    def get(self, key):
        return self.values[key]

    def set(self, key, value) -> bool:
        if self.values.get(key) == value:
            return False
        self.values[key] = value
        _write_json(self.path, self.values)
        return True
