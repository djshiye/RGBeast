"""What the GNOME desktop asks for, read through the `gsettings` CLI (no GLib bindings needed):
light or dark, the accent colour, reduced motion and the window button layout."""
import shutil
import subprocess
from typing import List, Optional, Tuple

INTERFACE = "org.gnome.desktop.interface"
WM = "org.gnome.desktop.wm.preferences"

# libadwaita's accent colours (accent-color key, GNOME 47+).
ACCENTS = {
    "blue": "#3584e4", "teal": "#2190a4", "green": "#3a944a", "yellow": "#c88800", "orange": "#ed5b00",
    "red": "#e62d42", "pink": "#d56199", "purple": "#9141ac", "slate": "#6f8396",
}
DEFAULT_ACCENT = ACCENTS["blue"]
DEFAULT_LAYOUT = "appmenu:close"        # Fedora Workstation's default
BUTTONS = ("minimize", "maximize", "close")


def available() -> bool:
    return shutil.which("gsettings") is not None


def get(schema: str, key: str) -> Optional[str]:
    if not available():
        return None
    try:
        r = subprocess.run(["gsettings", "get", schema, key], capture_output=True, text=True, timeout=3)
    except (OSError, subprocess.SubprocessError):
        return None
    return unquote(r.stdout) if r.returncode == 0 else None


def unquote(text: str) -> str:
    s = (text or "").strip()
    if len(s) >= 2 and s[0] == s[-1] and s[0] in "'\"":
        return s[1:-1]
    return s


def prefers_dark(value: Optional[str]) -> bool:
    return value is not None and "prefer-dark" in value


def accent(value: Optional[str]) -> str:
    return ACCENTS.get(unquote(value or ""), DEFAULT_ACCENT)


def animations(value: Optional[str]) -> bool:
    return unquote(value or "true") != "false"


def button_layout(value: Optional[str]) -> Tuple[List[str], List[str]]:
    """"appmenu:minimize,maximize,close" -> ([], ["minimize", "maximize", "close"])."""
    s = unquote(value if value else DEFAULT_LAYOUT)
    left, _, right = s.partition(":")

    def side(part):
        out = []
        for b in part.split(","):
            b = b.strip()
            if b in BUTTONS and b not in out:
                out.append(b)
        return out

    lft, rgt = side(left), side(right)
    if "close" not in lft and "close" not in rgt:
        rgt.append("close")     # never leave a window without a way to close it
    return lft, rgt


def parse_monitor_line(line: str) -> Optional[Tuple[str, str]]:
    """A `gsettings monitor` line ("color-scheme: 'prefer-dark'") as (key, value)."""
    key, sep, value = line.partition(":")
    if not sep:
        return None
    return key.strip(), unquote(value)
