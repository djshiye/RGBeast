"""D-Bus error names to the three kinds the app reports, and their wording."""
import unittest

try:
    from rgbeast_gui.client import DENIED, FAILED, UNAVAILABLE, classify, friendly
except ImportError:
    raise unittest.SkipTest("PySide6 is not installed")


class ErrorTests(unittest.TestCase):
    def test_classify(self):
        self.assertEqual(classify("org.freedesktop.DBus.Error.ServiceUnknown"), UNAVAILABLE)
        self.assertEqual(classify("org.freedesktop.DBus.Error.NoReply.Disconnected"), UNAVAILABLE)
        self.assertEqual(classify("org.freedesktop.DBus.Error.AccessDenied"), DENIED)
        self.assertEqual(classify("org.freedesktop.DBus.Error.AuthFailed"), DENIED)
        self.assertEqual(classify("org.freedesktop.DBus.Error.InvalidArgs"), FAILED)
        self.assertEqual(classify(""), FAILED)

    def test_friendly(self):
        self.assertEqual(friendly(UNAVAILABLE, "x"), "The lighting service is not running")
        self.assertEqual(friendly(DENIED), "Not allowed to change the lighting from this session")
        self.assertEqual(friendly(FAILED, " no device sim:x "), "Could not apply: no device sim:x")
        self.assertEqual(friendly(FAILED), "Could not apply the change")


if __name__ == "__main__":
    unittest.main()
