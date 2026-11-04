"""RGBeast: the Qt Quick window, its backends, single-instance handling and a screenshot/self-test harness."""
import argparse
import os
import signal
import sys
from pathlib import Path

from . import APP_ID, __version__

HERE = Path(__file__).resolve().parent
QML_DIR = HERE / "qml"
ICON_NAME = APP_ID + ".svg"
APP_PATH = "/io/github/djshiye/RGBeast"


def find_icon() -> Path:
    """The SVG is in data/icons/ in the source tree, and in the hicolor theme next to
    <prefix>/share/rgbeast/rgbeast_gui once installed."""
    for p in (HERE.parent / "data/icons/hicolor/scalable/apps" / ICON_NAME,
              HERE.parent.parent / "icons/hicolor/scalable/apps" / ICON_NAME,
              Path("/usr/share/icons/hicolor/scalable/apps") / ICON_NAME):
        if p.is_file():
            return p
    return HERE.parent / "data/icons/hicolor/scalable/apps" / ICON_NAME


def parse_args(argv):
    ap = argparse.ArgumentParser(prog="rgbeast", description="RGB lighting control (talks to rgbeastd over D-Bus)")
    ap.add_argument("--session", action="store_true",
                    help="use the session bus (development against `rgbeastd --session --simulate`); "
                         "RGBEAST_BUS=session does the same")
    ap.add_argument("--version", action="version", version=f"rgbeast {__version__}")
    dev = ap.add_argument_group("development")
    dev.add_argument("--screenshot", metavar="PNG", help="render the window to PNG after loading, then quit")
    dev.add_argument("--self-test", action="store_true",
                     help="load the window, wait for the devices, exit non-zero on any QML warning")
    dev.add_argument("--theme", choices=("dark", "light"), help="force a style instead of following GNOME")
    dev.add_argument("--size", help="window size WxH (including the 14 px shadow margin)")
    dev.add_argument("--device", help="open on this device id (or 'all')")
    dev.add_argument("--sheet", choices=("prefs", "about", "shortcuts", "menu", "toast"),
                     help="open a sheet, the main menu or a toast (screenshots)")
    dev.add_argument("--delay-ms", type=int, default=1800)
    return ap.parse_args(argv)


class Messages:
    """Collects Qt warnings so --self-test can fail on them, and still prints everything."""

    def __init__(self):
        self.warnings = []

    def __call__(self, mode, context, message):
        from PySide6.QtCore import QtMsgType
        if mode in (QtMsgType.QtWarningMsg, QtMsgType.QtCriticalMsg, QtMsgType.QtFatalMsg):
            where = f"{context.file}:{context.line}: " if context.file else ""
            self.warnings.append(where + message)
            print(f"qml: warning: {where}{message}" if (context.category or "").startswith("qml")
                  or ".qml" in (context.file or "") else f"warning: {message}", file=sys.stderr, flush=True)
        elif mode == QtMsgType.QtInfoMsg or mode == QtMsgType.QtDebugMsg:
            print(message, file=sys.stderr, flush=True)


def claim_instance(app_bus, activate_cb) -> bool:
    """Own io.github.djshiye.RGBeast on the session bus. A second launch asks the first to show its
    window and exits. Without a session bus (headless), every launch runs on its own."""
    from PySide6.QtCore import ClassInfo, QObject, Slot
    from PySide6.QtDBus import QDBusConnection, QDBusMessage

    if not app_bus.isConnected():
        return True

    @ClassInfo({"D-Bus Interface": "io.github.djshiye.RGBeast.App"})
    class AppService(QObject):
        @Slot()
        def Activate(self):
            activate_cb()

    if app_bus.registerService(APP_ID):
        claim_instance.service = AppService()
        app_bus.registerObject(APP_PATH, claim_instance.service, QDBusConnection.ExportAllSlots)
        return True
    msg = QDBusMessage.createMethodCall(APP_ID, APP_PATH, "io.github.djshiye.RGBeast.App", "Activate")
    app_bus.call(msg)
    return False


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else argv)
    harness = bool(args.screenshot or args.self_test)

    # Qt's QML disk cache trusts source mtimes, but RPM clamps every file to the build day, so two
    # releases built the same day would share stale bytecode. Key it by version.
    cache_root = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    os.environ.setdefault("QML_DISK_CACHE_PATH", str(cache_root / "rgbeast" / f"qmlcache-{__version__}"))

    if args.screenshot:
        # grabWindow() can deadlock against the threaded render loop while a live blur is on screen.
        os.environ.setdefault("QSG_RENDER_LOOP", "basic")

    try:
        from PySide6.QtCore import QTimer, QUrl, qInstallMessageHandler
        from PySide6.QtDBus import QDBusConnection
        from PySide6.QtGui import QFont, QFontDatabase, QGuiApplication, QIcon
        from PySide6.QtQml import QQmlApplicationEngine
        from PySide6.QtQuick import QQuickWindow  # noqa: F401  (registers the window type for grabWindow)
        from PySide6.QtQuickControls2 import QQuickStyle
    except ImportError as e:
        print(f"rgbeast needs PySide6 ({e}): sudo dnf install python3-pyside6", file=sys.stderr)
        return 1

    messages = Messages()
    qInstallMessageHandler(messages)

    QGuiApplication.setApplicationName("RGBeast")
    QGuiApplication.setApplicationDisplayName("RGBeast")
    QGuiApplication.setOrganizationName("djshiye")
    QGuiApplication.setDesktopFileName(APP_ID)
    app = QGuiApplication(sys.argv[:1])
    QQuickStyle.setStyle("Basic")

    families = set(QFontDatabase.families())
    for name in ("Adwaita Sans", "Inter", "Cantarell"):
        if name in families:
            f = QFont(name)
            f.setPixelSize(13)
            app.setFont(f)
            break

    win_holder = {}

    def activate():
        w = win_holder.get("win")
        if w is not None:
            w.show()
            w.raise_()
            w.requestActivate()

    if not harness and not claim_instance(QDBusConnection.sessionBus(), activate):
        return 0

    from . import widgets  # noqa: F401  (registers LedPreview, ColorWheel, ColorStrip)
    from .backend import Appearance, Lighting, LiquidGlass
    from .client import Client, use_session_bus
    from .store import SceneStore, Settings

    icon_path = find_icon()
    icon = QIcon(str(icon_path))
    if icon.isNull():
        icon = QIcon.fromTheme(APP_ID)
    app.setWindowIcon(icon)

    settings = Settings()
    engine = QQmlApplicationEngine()
    # Backends are children of the engine so they outlive the QML objects bound to them at exit.
    appearance = Appearance(None if args.theme is None else args.theme == "dark", engine)
    glass = LiquidGlass(parent=engine)
    client = Client(session=use_session_bus(args.session), parent=engine)
    lighting = Lighting(client, settings, SceneStore(), engine)
    if harness:
        # Screenshots and self-tests never change the user's remembered device or window size.
        settings.set = lambda *_a, **_k: False
    if args.device:
        lighting.debug_device = args.device

    engine.addImportPath(str(QML_DIR))
    ctx = engine.rootContext()
    ctx.setContextProperty("lighting", lighting)
    ctx.setContextProperty("appearance", appearance)
    ctx.setContextProperty("liquidGlass", glass)
    ctx.setContextProperty("iconUrl", QUrl.fromLocalFile(str(icon_path)))
    ctx.setContextProperty("appVersion", __version__)
    engine.load(QUrl.fromLocalFile(str(QML_DIR / "Main.qml")))
    if not engine.rootObjects():
        return 1
    win = engine.rootObjects()[0]
    win_holder["win"] = win

    if args.size:
        w, h = (int(v) for v in args.size.lower().split("x"))
    else:
        w, h = settings.get("window-width"), settings.get("window-height")
    win.setWidth(max(w, win.minimumWidth()))
    win.setHeight(max(h, win.minimumHeight()))
    if args.device:
        win.setProperty("showContent", True)

    def remember():
        if win.visibility() != win.Visibility.Maximized:
            settings.set("window-width", int(win.width()))
            settings.set("window-height", int(win.height()))
        settings.set("window-maximized", win.visibility() == win.Visibility.Maximized)

    app.aboutToQuit.connect(remember)
    app.aboutToQuit.connect(appearance.shutdown)

    appearance.watch()
    glass.watch()
    lighting.start()
    if settings.get("window-maximized") and not harness:
        win.showMaximized()
    else:
        win.show()

    signal.signal(signal.SIGINT, signal.SIG_DFL)

    if args.sheet:
        names = {"prefs": "prefsSheet", "about": "aboutSheet", "shortcuts": "shortcutsSheet"}
        if args.sheet in names:
            show = lambda: _open_sheet(win, names[args.sheet])  # noqa: E731
        else:
            show = lambda: win.setProperty("demo", args.sheet)  # noqa: E731
        QTimer.singleShot(max(200, args.delay_ms - 700), show)

    if args.screenshot:
        def grab():
            img = win.grabWindow()
            ok = img.save(args.screenshot)
            print(f"screenshot {'saved' if ok else 'FAILED'}: {args.screenshot} {img.width()}x{img.height()}",
                  flush=True)
            app.exit(0 if ok else 2)
        QTimer.singleShot(args.delay_ms, grab)
    elif args.self_test:
        def finish():
            status = lighting.property("status")
            print(f"self-test: status={status} devices={len(lighting.infos)} warnings={len(messages.warnings)}",
                  flush=True)
            app.exit(1 if messages.warnings else 0)
        QTimer.singleShot(args.delay_ms, finish)

    return app.exec()


def _open_sheet(win, name):
    from PySide6.QtCore import QObject
    sheet = win.findChild(QObject, name)
    if sheet is not None:
        sheet.open()
