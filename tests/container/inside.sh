#!/usr/bin/env bash
# What runs inside the test container: the far side of tests/container.sh,
# run from the snapshot of the checkout in /work.
#
#   bash tests/container/inside.sh                          # tests/all.sh
#   bash tests/container/inside.sh bash tests/e2e-box.sh    # one suite
#   bash tests/container/inside.sh --no-build <command>     # the desktop only
#
# It stands up what tests/all.sh expects to find on the machine, then gets
# out of the way: the links install.sh makes, a headless sway, an outer niri
# as its one window with two named workspaces, and the session variables
# niri's session sets. tests/all.sh then runs as it does on the desktop, and
# its e2e suites nest their own niri inside this one. Nothing here is the
# machine's: no socket, file or display of the host reaches the container,
# and it has no network.
#
# It builds the snapshot first and points NIRITASKS at that build: there is
# no installed niritasks in here to test by mistake. When the command fails,
# the run's logs and the e2e sandboxes it kept (frames included) go into
# /tmp/artifacts.tar for tests/container.sh to copy out.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

die() { echo "container: $*" >&2; exit 1; }

build=1
[ "${1:-}" = --no-build ] && { build=""; shift; }

export XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
# What niri's session sets on the machine.
export XDG_CURRENT_DESKTOP=niri XDG_SESSION_DESKTOP=niri XDG_SESSION_TYPE=wayland
# Mesa's llvmpipe for every GL client — the outer niri, the nested niris,
# alacritty. GTK draws with Vulkan, as on the machine, on Mesa's lavapipe.
export LIBGL_ALWAYS_SOFTWARE=1
# Every crate is in the image's registry; with no network, say so at once
# rather than time out.
export CARGO_NET_OFFLINE=true
# The e2e suites keep their sandboxes, frames and logs, instead of deleting
# them: in here they go with the container, unless a failure has them copied
# out.
export NIRITASKS_E2E_KEEP=1

# Whether the image still matches the machine. A difference is worth knowing
# before reading the results, but is not a failure.
if [ -n "${NIRITASKS_HOST_PARITY:-}" ]; then
    if drift=$(diff <(printf '%s\n' "$NIRITASKS_HOST_PARITY") <(bash tests/container/parity.sh)); then
        echo "parity: the image matches this machine"
    else
        echo "parity: the image differs from this machine (< machine, > image):"
        printf '%s\n' "$drift" | grep '^[<>]' | sed 's/^/  /'
        echo "  rebuild it with: NIRITASKS_CONTAINER_REBUILD=1 bash tests/container.sh"
    fi
fi

# The container has no accessibility bus, and every GTK process otherwise
# warns about it on stderr; a11y does not change drawing.
export GTK_A11Y=none

# GTK 4 on Wayland takes its settings from the desktop portal (as on the
# machine) and does not read GSettings without one, so the container gives
# GTK the same values through settings.ini, translated as GTK translates
# them. They come from the image's gsettings, so the dconf keyfile stays the
# one source parity.sh compares with the machine.
gs() { gsettings get org.gnome.desktop.interface "$1" 2>/dev/null | tr -d "'"; }
aa=$(gs font-antialiasing)
case "$aa" in
    none) antialias=0 rgba=none ;;
    rgba) antialias=1 rgba=$(gs font-rgba-order) ;;
    *)    antialias=1 rgba=none ;;
esac
case "$(gs font-hinting)" in
    none)   hinting=0 hintstyle=hintnone ;;
    medium) hinting=1 hintstyle=hintmedium ;;
    full)   hinting=1 hintstyle=hintfull ;;
    *)      hinting=1 hintstyle=hintslight ;;
esac
case "$(gs color-scheme)" in
    prefer-dark)  scheme=dark ;;
    prefer-light) scheme=light ;;
    *)            scheme=default ;;
esac
dpi=$(awk -v f="$(gs text-scaling-factor)" 'BEGIN { printf "%d", 96 * 1024 * f + 0.5 }')
mkdir -p "$HOME/.config/gtk-4.0"
cat > "$HOME/.config/gtk-4.0/settings.ini" <<INI
[Settings]
gtk-theme-name=$(gs gtk-theme)
gtk-icon-theme-name=$(gs icon-theme)
gtk-cursor-theme-name=$(gs cursor-theme)
gtk-font-name=$(gs font-name)
gtk-interface-color-scheme=$scheme
gtk-xft-antialias=$antialias
gtk-xft-rgba=$rgba
gtk-xft-hinting=$hinting
gtk-xft-hintstyle=$hintstyle
gtk-xft-dpi=$dpi
INI

# What install.sh links on the machine, from this checkout, to the same
# places. e2e-tag.sh's refine checks need the refine mod: refine::launch
# refuses to start without it. The rest make this home look like the
# machine's. install.sh is the authority: a file it links that is not listed
# here stops the run, so the two cannot drift apart quietly.
LINKS=(
    "niri/niri-tasks.kdl                             .config/niri/niri-tasks.kdl"
    ".claude/skills/workspace-tasks/SKILL.md         .claude/skills/workspace-tasks/SKILL.md"
    ".claude/skills/refine-task/SKILL.md             .claude/skills/refine-task/SKILL.md"
    ".claude/skills/refine-task/report-catalogue.md  .claude/skills/refine-task/report-catalogue.md"
    ".claude/skills/report-task/SKILL.md             .claude/skills/report-task/SKILL.md"
    ".claude/skills/report-branch/SKILL.md           .claude/skills/report-branch/SKILL.md"
    ".claude/skills/report-branch/branch-catalogue.md .claude/skills/report-branch/branch-catalogue.md"
    ".claude/skills/finish-worktree/SKILL.md         .claude/skills/finish-worktree/SKILL.md"
    ".claude/skills/reject-worktree/SKILL.md         .claude/skills/reject-worktree/SKILL.md"
    ".claude/skills/private-github-repo/SKILL.md     .claude/skills/private-github-repo/SKILL.md"
    "claude/refine-mod                               .local/share/niri-tasks/refine-mod"
    "claude/rules/niri-tasks.md                      .claude/rules/niri-tasks.md"
)
for entry in "${LINKS[@]}"; do
    read -r src dst <<< "$entry"
    [ -e "$src" ] || die "install.sh's $src is not in this checkout"
    mkdir -p "$HOME/$(dirname "$dst")"
    ln -sfn "$PWD/$src" "$HOME/$dst"
done
# Only lines that start with `link`: `herdr plugin link "$REPO/…"` is a
# herdr registration, not a file link.
unlisted=""
while read -r src; do
    printf '%s\n' "${LINKS[@]}" | awk '{ print $1 }' | grep -qxF "$src" || unlisted="$unlisted $src"
done < <(grep -oE '^link "\$REPO/[^"]*"' install.sh | sed 's/^link "\$REPO\///; s/"$//')
[ -z "$unlisted" ] || die "install.sh links what inside.sh does not:$unlisted — add them to LINKS"

WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 \
    sway -c tests/container/sway.conf >/tmp/sway.log 2>&1 &
sway_display=""
for _ in $(seq 1 50); do
    sway_display=$(cd "$XDG_RUNTIME_DIR" && ls -d wayland-[0-9] 2>/dev/null | head -n 1)
    [ -n "$sway_display" ] && [ -S "$XDG_RUNTIME_DIR/$sway_display" ] && break
    sleep 0.2
done
[ -n "$sway_display" ] || die "sway did not start:
$(tail -n 20 /tmp/sway.log)"

WAYLAND_DISPLAY="$sway_display" niri -c tests/container/niri.kdl >/tmp/niri.log 2>&1 &
for _ in $(seq 1 50); do
    [ -s "$XDG_RUNTIME_DIR/outer.env" ] && break
    sleep 0.2
done
[ -s "$XDG_RUNTIME_DIR/outer.env" ] || die "the outer niri did not start:
$(tail -n 20 /tmp/niri.log)"
NIRI_SOCKET=$(sed -n 's/^NIRI_SOCKET=//p' "$XDG_RUNTIME_DIR/outer.env")
WAYLAND_DISPLAY=$(sed -n 's/^WAYLAND_DISPLAY=//p' "$XDG_RUNTIME_DIR/outer.env")
export NIRI_SOCKET WAYLAND_DISPLAY
[ -n "$NIRI_SOCKET" ] && [ -n "$WAYLAND_DISPLAY" ] ||
    die "the outer niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"
niri msg workspaces || die "the outer niri does not answer"

if [ -n "$build" ]; then
    echo "building $(pwd)"
    cargo build --locked || die "cargo build failed"
    export NIRITASKS="$PWD/target/debug/niritasks"
fi

[ $# -gt 0 ] || set -- bash tests/all.sh
"$@"
status=$?
if [ "$status" -ne 0 ]; then
    # Regular files only: the sandboxes also hold sockets, which do not copy.
    (cd /tmp && find sway.log niri.log tmp.* -type f 2>/dev/null) > /tmp/artifacts.list
    tar -C /tmp -cf /tmp/artifacts.tar -T /tmp/artifacts.list 2>/dev/null
fi
exit "$status"
