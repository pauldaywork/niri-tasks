"""What GTK sees: the settings that decide how a window is drawn, and the
renderer it draws with. Run on the machine and in the test container
(tests/container/inside.sh) and diff the two; they should be the same.
tests/container/parity.sh checks the inputs at every run; this checks what
GTK makes of them, which needs a display.

    python3 tests/container/gtk-settings.py
"""
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk  # noqa: E402

SETTINGS = [
    "gtk-theme-name", "gtk-interface-color-scheme", "gtk-application-prefer-dark-theme",
    "gtk-font-name", "gtk-xft-antialias", "gtk-xft-hinting", "gtk-xft-hintstyle",
    "gtk-xft-rgba", "gtk-xft-dpi", "gtk-hint-font-metrics", "gtk-font-rendering",
    "gtk-icon-theme-name", "gtk-cursor-theme-name", "gtk-enable-animations",
]


def main():
    settings = Gtk.Settings.get_default()
    for name in SETTINGS:
        print(name, settings.get_property(name))
    window = Gtk.Window()
    window.present()
    loop = GLib.MainLoop()

    def report():
        print("renderer", type(window.get_renderer()).__name__)
        loop.quit()
        return False

    GLib.timeout_add(500, report)
    loop.run()


main()
