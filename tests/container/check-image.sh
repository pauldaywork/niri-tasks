#!/usr/bin/env bash
# Whether the test image is as designed: what the suites use on the
# machine, at the machine's versions, in the machine's folders. README's
# "Running the tests in a container" says why each piece is there. Run
# inside the image, from the host:
#
#   docker run --rm -i "$(bash tests/container/image.sh)" bash -s < tests/container/check-image.sh
#
# One line per item, ok or MISSING, and the exit status is how many are
# missing. A piece that is absent here would otherwise show up much later,
# as a suite skipped or a frame a few pixels off.
set -u

missing=0
ok()   { echo "  ok       $*"; }
miss() { echo "  MISSING  $*"; missing=$((missing+1)); }
check() {  # label, command...
    local label="$1"; shift
    if "$@" >/dev/null 2>&1; then ok "$label"; else miss "$label"; fi
}

echo "programs"
for p in niri sway alacritty wtype python3 git flock setsid timeout realpath \
         sqlite3 tar fzf xdg-open xdg-settings notify-send dbus-run-session \
         cargo rustc cc pkg-config task fc-match gsettings locale \
         herdr wt claude ghostty; do
    check "$p" command -v "$p"
done

echo "versions"
check "niri 26.04"          sh -c 'niri --version | grep -q "^niri 26\.04"'
check "task 3.5.0"          sh -c '[ "$(task --version)" = 3.5.0 ]'
check "rustc 1.95.0"        sh -c 'rustc --version | grep -q "^rustc 1\.95\.0 "'
check "no apt taskwarrior"  sh -c '! dpkg -s taskwarrior'
check "herdr 0.9.3"         sh -c '[ "$(herdr --version)" = "herdr 0.9.3" ]'
check "wt v0.79.0"          sh -c 'wt --version | grep -q "^wt v0\.79\.0"'
check "claude 2.1.312"      sh -c 'claude --version | grep -q "^2\.1\.312 "'
check "ghostty 1.3.1"       sh -c 'ghostty --version | grep -q "^Ghostty 1\.3\.1"'

echo "where they are, as on the machine"
for p in herdr wt claude task; do
    check "$p in ~/.local/bin" sh -c "[ \"\$(command -v $p)\" = \"\$HOME/.local/bin/$p\" ]"
done
check "task -> ~/.local/opt/taskwarrior-3.5.0" sh -c '[ "$(readlink -f "$HOME/.local/bin/task")" = "$HOME/.local/opt/taskwarrior-3.5.0/bin/task" ]'
check "ghostty in /usr/bin"   sh -c '[ "$(command -v ghostty)" = /usr/bin/ghostty ]'
check "no herdr on the e2e spawn PATH" sh -c '! PATH=/usr/bin:/bin command -v herdr'

echo "left out on purpose"
for p in code gh plannotator-tui; do
    check "no $p" sh -c "! command -v $p"
done
check "no desktop portal for GTK to wait on" \
    test ! -e /usr/share/dbus-1/services/org.freedesktop.portal.Desktop.service

echo "build libraries"
for m in gtk4 gtk4-layer-shell-0 wayland-client; do
    check "pkg-config $m" pkg-config --exists "$m"
done

echo "python"
check "PIL"     python3 -c 'import PIL'
check "Gtk 4.0" python3 -c 'import gi; gi.require_version("Gtk", "4.0"); from gi.repository import Gtk'

echo "fonts"
check "Iosevka Term Extended" sh -c 'fc-match -f "%{file}" "Iosevka Term Extended" | grep -q "/IosevkaTerm-Extended.ttf$"'
check "54 Iosevka Term files" sh -c '[ "$(ls ~/.local/share/fonts/iosevka-term/*.ttf | wc -l)" = 54 ]'
check "action glyphs are JetBrainsMonoNL Nerd Font" sh -c 'fc-match -f "%{file}" "Iosevka Term Extended:charset=f141" | grep -q "/JetBrainsMonoNLNerdFont-Regular.ttf$"'
check "96 JetBrainsMono Nerd Font files" sh -c '[ "$(ls ~/.local/share/fonts/JetBrainsMono*.ttf | wc -l)" = 96 ]'
check "e2e-panel's fonts link finds them" sh -c 'd=$(mktemp -d) && ln -s ~/.local/share/fonts "$d/fonts" && XDG_DATA_HOME="$d" fc-match -f "%{file}" "Iosevka Term Extended" | grep -q "/IosevkaTerm-Extended.ttf$"; r=$?; rm -rf "$d"; exit $r'
check "sans is Noto Sans"     sh -c 'fc-match -f "%{file}" sans-serif | grep -q "/NotoSans-Regular.ttf$"'
check "emoji is Noto"         sh -c 'fc-match -f "%{file}" emoji | grep -q "/NotoColorEmoji.ttf$"'

echo "GTK settings"
want() {  # key, value as gsettings prints it
    check "$1 = $2" sh -c "[ \"\$(gsettings get org.gnome.desktop.interface $1)\" = \"$2\" ]"
}
want color-scheme        "'prefer-dark'"
want gtk-theme           "'Yaru-purple-dark'"
want icon-theme          "'Yaru-purple-dark'"
want cursor-theme        "'Adwaita'"
want font-name           "'Adwaita Sans 11'"
want monospace-font-name "'Ubuntu Sans Mono 11'"
want font-antialiasing   "'grayscale'"
want font-hinting        "'slight'"
want font-rgba-order     "'rgb'"
want text-scaling-factor "1.0"
check "Yaru-purple-dark gtk-4.0 theme" test -f /usr/share/themes/Yaru-purple-dark/gtk-4.0/gtk.css

echo "rendering"
check "lavapipe (Vulkan)" sh -c 'ls /usr/share/vulkan/icd.d/ | grep -q lvp'
check "llvmpipe (GL)"     sh -c 'ls /usr/lib/x86_64-linux-gnu/dri/ | grep -q swrast'

echo "user and session"
check "uid 1000"               sh -c '[ "$(id -u)" = 1000 ]'
check "HOME /home/tester"      sh -c '[ "$HOME" = /home/tester ]'
check "~/.taskrc"              grep -qx 'data.location=$HOME/.task' "$HOME/.taskrc"
check "~/Projects/alpha, beta" sh -c '[ -d "$HOME/Projects/alpha" ] && [ -d "$HOME/Projects/beta" ]'
check "LANG en_US.UTF-8"       sh -c '[ "$LANG" = en_US.UTF-8 ] && locale -a | grep -qix en_US.utf8'
check "tzdata"                 test -f /usr/share/zoneinfo/Australia/Melbourne
check "/work writable"         test -w /work
check "/work/target writable"  test -w /work/target

echo "prebuilt dependencies"
check "gtk4 compiled in /work/target"  sh -c 'ls /work/target/debug/deps | grep -q "^libgtk4-"'
check "clap compiled in /work/target"  sh -c 'ls /work/target/debug/deps | grep -q "^libclap-"'
check "no niri-tasks artifacts"        sh -c '! ls /work/target/debug/.fingerprint | grep -q "^niri-tasks-"'
check "crates in the registry"         sh -c 'ls "$CARGO_HOME"/registry/cache/*/ | grep -q "^gtk4-"'

echo
echo "$missing missing"
exit "$missing"
