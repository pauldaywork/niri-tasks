#!/usr/bin/env bash
# What the test image must share with the machine for the suites to see the
# same thing there as here: the drawing stack's versions, the programs the
# suites run, the fonts, and the GTK settings. One "name value" line each.
# tests/container.sh runs it on the machine, inside.sh runs it in the
# container, and a difference is printed at the top of the run: the image
# was built from an older machine, and NIRITASKS_CONTAINER_REBUILD=1 brings
# it up to date. Mesa is not here: the machine draws with its GPU.
set -u

for p in niri ghostty libgtk-4-1 libgtk4-layer-shell0 libpango-1.0-0 libcairo2 \
         libharfbuzz0b libfreetype6 fontconfig-config yaru-theme-gtk \
         fonts-noto-core fonts-noto-color-emoji fonts-noto-mono fonts-noto-cjk \
         fonts-dejavu-core fonts-dejavu-extra fonts-dejavu-mono fonts-droid-fallback \
         fonts-liberation fonts-liberation2 fonts-liberation-sans-narrow fonts-ubuntu fonts-urw-base35; do
    echo "$p $(dpkg-query -W -f '${Version}' "$p" 2>/dev/null || echo missing)"
done
for c in task herdr wt claude rustc; do
    echo "$c $("$c" --version 2>/dev/null | head -n 1 || echo missing)"
done
echo "iosevka-term-extended $(fc-match -f '%{fontversion}' 'Iosevka Term Extended' 2>/dev/null || echo missing)"
echo "action-glyph-font $(fc-match -f '%{family[0]} %{fontversion}' 'Iosevka Term Extended:charset=f141' 2>/dev/null || echo missing)"
for k in color-scheme gtk-theme icon-theme cursor-theme font-name monospace-font-name \
         font-antialiasing font-hinting font-rgba-order text-scaling-factor; do
    echo "gsettings.$k $(gsettings get org.gnome.desktop.interface "$k" 2>/dev/null || echo missing)"
done
