"""The editor for one device, without any Qt: the state being edited and every edit the page offers
(effect, colour slots, wheel/swatch/hex colour, painting single LEDs, brightness, speed, direction,
random). Ported from the GTK crate's device_page.rs so the behaviour is the same.

`view()` describes what the page should show; the Qt layer turns it into properties.
"""
import copy
from typing import List, Optional

from . import color as C
from . import model as M


class Editor:
    def __init__(self):
        self.info: dict = M.all_devices_info()
        self.state: dict = M.default_state()
        self.is_group = False
        self.selected_slot = 0
        self.painted = False

    # ------------------------------------------------------------ loading
    def load(self, info: dict, state: dict, is_group: bool = False):
        self.info = info
        self.state = copy.deepcopy(state)
        self.is_group = is_group
        self.selected_slot = 0
        self.painted = False
        self._clamp_slot()

    def adopt(self, state: dict):
        """The daemon confirmed (and normalised) a state."""
        self.state = copy.deepcopy(state)
        self._clamp_slot()

    def set_info(self, info: dict):
        self.info = info

    def mode(self) -> Optional[dict]:
        return M.mode_info(self.info, self.state["mode"])

    def _clamp_slot(self):
        n = len(self.state["colors"])
        self.selected_slot = max(0, min(self.selected_slot, n - 1)) if n else 0

    # ------------------------------------------------------------ view
    def current_color(self) -> C.Rgb:
        cs = self.state["colors"]
        if self.selected_slot < len(cs):
            return cs[self.selected_slot]
        if cs:
            return cs[0]
        return M.primary_color(self.state)

    def view(self) -> dict:
        m = self.mode() or {}
        cm = m.get("color_mode", "none")
        uses_colors = bool(m) and cm != "none"
        per_led = cm == "per-led"
        multi = cm == "mode-colors" and m.get("colors_max", 0) > 1
        dirs = list(m.get("directions", []))
        cmin, cmax = (m.get("colors_min", 1), m.get("colors_max", 1)) if m else (1, 1)
        n = len(self.state["colors"])
        background_last = cmin >= 2 and cmax == 11
        slots = []
        if multi:
            for i, c in enumerate(self.state["colors"]):
                label = "Background colour" if background_last and i == n - 1 else f"Colour {i + 1}"
                slots.append({"color": C.to_hex(c), "label": label})
        dir_index = 1 if len(dirs) >= 2 and self.state["direction"] == dirs[1] else 0
        return {
            "modes": [{"id": x["id"], "name": x["name"]} for x in self.info.get("modes", [])],
            "mode": self.state["mode"],
            "usesColors": uses_colors,
            "perLed": per_led,
            "paintBar": per_led and not self.is_group,
            "multi": multi,
            "slots": slots,
            "selectedSlot": self.selected_slot,
            "canAddSlot": multi and n < cmax,
            "canRemoveSlot": multi and n > max(cmin, 1),
            "color": C.to_hex(self.current_color()),
            "hasBrightness": bool(m.get("has_brightness")),
            "brightness": self.state["brightness"],
            "hasSpeed": bool(m.get("has_speed")),
            "speed": self.state["speed"],
            "directions": [M.direction_label(d) for d in dirs[:2]] if len(dirs) >= 2 else [],
            "directionIndex": dir_index,
            "hasRandom": bool(m.get("has_random")),
            "random": self.state["random"],
            "hasMotion": bool(m.get("has_speed")) or len(dirs) >= 2 or bool(m.get("has_random")),
        }

    def preview(self, group_layout: Optional[List[dict]] = None) -> List[dict]:
        if self.is_group:
            return group_layout or []
        return M.device_preview(self.info, self.state)

    # ------------------------------------------------------------ edits (each returns True if the state changed)
    def select_mode(self, mode_id: str) -> bool:
        m = M.mode_info(self.info, mode_id)
        if m is None or mode_id == self.state["mode"]:
            return False
        st = self.state
        current = self.current_color()
        st["mode"] = mode_id
        cm = m.get("color_mode")
        if cm == "none":
            st["colors"] = []
        elif cm == "mode-colors":
            if not st["colors"]:
                st["colors"].append(current)
            del st["colors"][max(1, m.get("colors_max", 1)):]
            while len(st["colors"]) < m.get("colors_min", 0):
                st["colors"].append(M.PALETTE[len(st["colors"]) % len(M.PALETTE)])
        elif cm == "per-led":
            if not st["colors"]:
                st["colors"].append(current)
            del st["colors"][1:]
            if not st["zones"]:
                c = st["colors"][0]
                st["zones"] = [{"id": z["id"], "colors": [c] * int(z["leds"])} for z in self.info.get("zones", [])]
        dirs = m.get("directions", [])
        if dirs and st["direction"] not in dirs:
            st["direction"] = dirs[0]
        self.painted = False
        self.selected_slot = 0
        return True

    def select_slot(self, i: int) -> bool:
        if 0 <= i < len(self.state["colors"]) and i != self.selected_slot:
            self.selected_slot = i
            return True
        return False

    def pick_color(self, c: C.Rgb) -> bool:
        st = self.state
        slot = self.selected_slot
        if not st["colors"]:
            st["colors"].append(c)
        elif slot < len(st["colors"]):
            if st["colors"][slot] == c and not ((self.mode() or {}).get("color_mode") == "per-led" and not self.painted):
                return False
            st["colors"][slot] = c
        else:
            st["colors"][0] = c
        if (self.mode() or {}).get("color_mode") == "per-led" and not self.painted:
            st["zones"] = [{"id": z["id"], "colors": [c] * int(z["leds"])} for z in self.info.get("zones", [])]
        return True

    def fill_all(self) -> bool:
        self.painted = False
        return self.pick_color(self.current_color())

    def paint_led(self, zone: int, led: int) -> bool:
        if self.is_group:
            return False
        c = self.current_color()
        st = self.state
        zones = self.info.get("zones", [])
        if len(st["zones"]) != len(zones):
            base = M.primary_color(st)
            st["zones"] = [{"id": z["id"], "colors": [base] * int(z["leds"])} for z in zones]
        if not (0 <= zone < len(st["zones"])):
            return False
        z = st["zones"][zone]
        n = int(zones[zone]["leds"]) if zone < len(zones) else 0
        if len(z["colors"]) < n:
            last = z["colors"][-1] if z["colors"] else c
            z["colors"].extend([last] * (n - len(z["colors"])))
        if not (0 <= led < len(z["colors"])):
            return False
        z["colors"][led] = c
        self.painted = True
        return True

    def add_slot(self) -> bool:
        cmax = (self.mode() or {}).get("colors_max", 1)
        cs = self.state["colors"]
        if len(cs) >= cmax:
            return False
        cs.append(M.PALETTE[len(cs) % len(M.PALETTE)])
        self.selected_slot = len(cs) - 1
        return True

    def remove_slot(self) -> bool:
        cmin = max(1, (self.mode() or {}).get("colors_min", 1))
        cs = self.state["colors"]
        if len(cs) <= cmin:
            return False
        i = min(self.selected_slot, len(cs) - 1)
        del cs[i]
        self.selected_slot = max(0, i - 1)
        return True

    def set_brightness(self, v: int) -> bool:
        v = max(0, min(100, int(v)))
        if v == self.state["brightness"]:
            return False
        self.state["brightness"] = v
        return True

    def set_speed(self, v: int) -> bool:
        v = max(0, min(100, int(v)))
        if v == self.state["speed"]:
            return False
        self.state["speed"] = v
        return True

    def set_direction(self, index: int) -> bool:
        dirs = (self.mode() or {}).get("directions", [])
        if not (0 <= index < len(dirs)) or self.state["direction"] == dirs[index]:
            return False
        self.state["direction"] = dirs[index]
        return True

    def set_random(self, on: bool) -> bool:
        if bool(on) == self.state["random"]:
            return False
        self.state["random"] = bool(on)
        return True
