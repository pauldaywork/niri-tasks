#!/usr/bin/env bash
# End-to-end test of the task panel: task state -> pixels on screen.
#
#   bash tests/e2e-panel.sh
#
# Not a cargo test, and cannot be. The panel is a layer surface whose whole
# behaviour is what the compositor puts on screen: whether cards are there, how
# far they poke out, how many there are. The window reports the size it asked
# for, not the size it got, and a layer surface that was never mapped reports
# nothing wrong while showing nothing at all — both have happened before.
#
# So this measures pixels, on a screen of its own. It starts a nested niri — a
# window on yours, with its own runtime dir, sockets and one 1600x1000 output
# of flat colour — and runs its own daemon in it against a sandboxed TASKDATA.
# Nothing on your desktop can reach those frames: not the wallpaper, not a
# terminal redrawing, not a window rule or a notification or the pointer. Your
# own niri-tasks daemon keeps running throughout and never sees these tasks.
#
# Because the screen is flat and fixed, the numbers are exact: the peek starts
# at column 1566, the keyboard's cards span 420-1180, and a frame that should
# not have changed is compared pixel for pixel. A parked nested niri draws only
# when asked, so its first frame after a change can be stale; every frame is
# shot until two in a row match.
#
# The nested window is parked, floating and unfocused, on the last workspace of
# the monitor you start it from. Keep off that workspace while it runs: going
# there focuses the window, and keys typed then would reach the panel. If that
# happens the run fails and says so, rather than measuring frames it can no
# longer trust.
#
# What it cannot check is the hover: nothing can move the pointer, so the slide
# out, the slide back and clicks passing beside the peek are checked by hand
# (README, "Testing"). The keyboard it can: `task panel` moves the panel to the
# middle of the screen, and with wtype, Down and Escape are pressed in the
# nested niri, never on your desktop.
set -uo pipefail

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }
[ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display to open the nested niri on" >&2; exit 1; }
python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

NIRITASKS="${NIRITASKS:-niritasks}"

# The nested niri's one output, in pixels: the size its window is parked at.
OUT_W=1600
OUT_H=1000
# Mirror PEEK_PX, RING_PX and SURFACE_WIDTH in src/panel/surface.rs, and
# CARD_WIDTH_PX in src/panel/style.rs.
PEEK=30
RING=4
CARD=760
SURFACE=784
# Where the tucked peek starts: the peek, and the card's outline ring drawn
# outside it.
PEEK_X=$((OUT_W - PEEK - RING))
# Where the keyboard's cards start, in the middle of the screen.
CARD_X=$(((OUT_W - CARD) / 2))
# The centred surface, shadow and all.
SURFACE_LEFT=$(((OUT_W - SURFACE) / 2))
SURFACE_RIGHT=$((SURFACE_LEFT + SURFACE))

SB="$(mktemp -d)"
# Short on purpose: the nested niri's sockets and the daemon's go in here, and
# a Unix socket path longer than ~108 characters cannot be bound.
RT="$(mktemp -d /tmp/nte2e.XXXXXX)"
mkdir -p "$SB/data" "$SB/shots"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

# The nested niri connects to this one by its absolute path, since its own
# XDG_RUNTIME_DIR is somewhere else.
case "$WAYLAND_DISPLAY" in
    /*) PARENT_WAYLAND="$WAYLAND_DISPLAY" ;;
    *) PARENT_WAYLAND="${XDG_RUNTIME_DIR:?}/$WAYLAND_DISPLAY" ;;
esac

# The real daemon is never touched; this is how the end of the run proves it.
SERVICE_PID=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)

NESTED=""; WIN=""; WATCH=""; DAEMON=""
cleanup() {
    [ -n "$WATCH" ] && kill "$WATCH" 2>/dev/null && wait "$WATCH" 2>/dev/null
    [ -n "$DAEMON" ] && kill "$DAEMON" 2>/dev/null && wait "$DAEMON" 2>/dev/null
    # Its window closes with it, and niri drops the workspace it leaves empty.
    [ -n "$NESTED" ] && kill "$NESTED" 2>/dev/null && wait "$NESTED" 2>/dev/null
    rm -rf "$RT"
    if [ -n "${NIRITASKS_E2E_KEEP:-}" ]; then
        echo "frames kept in $SB/shots"
    else
        rm -rf "$SB"
    fi
}
trap cleanup EXIT
trap 'exit 1' INT TERM

pass=0; fail=0; skipped=0
ok()   { echo "  PASS  $*"; pass=$((pass+1)); }
bad()  { echo "  FAIL  $*"; fail=$((fail+1)); }
skip() { echo "  SKIP  $*"; skipped=$((skipped+1)); }
die()  { echo "$*" >&2; exit 1; }
summary() { echo; echo "passed: $pass   failed: $fail   skipped: $skipped"; }

# ─── a niri of its own ───────────────────────────────────────────────────────
# Animations off, so niri itself never draws a frame in between; the panel's
# slide is its own and still runs. A flat background, so a frame with nothing
# on it is the same every time. One named workspace, which is the tag the
# panel's tasks are filed under. The startup command is how the test learns
# the nested niri's own sockets.
cat > "$SB/niri.kdl" <<EOF
hotkey-overlay { skip-at-startup; }
animations { off; }
xwayland-satellite { off; }
output "winit" { scale 1; }
layout { background-color "#406080"; }
workspace "e2e"
spawn-sh-at-startup "env > $SB/nested.env.tmp && mv $SB/nested.env.tmp $SB/nested.env"
EOF

# The parent's windows that are already nested niris, so the new one can be
# told apart from them.
niri_windows() {
    niri msg -j windows | python3 -c '
import json, sys
print(" ".join(str(w["id"]) for w in json.load(sys.stdin) if w["app_id"] == "niri"))'
}
before=$(niri_windows) || die "cannot list this niri's windows"

env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
    niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
NESTED=$!

for _ in $(seq 1 50); do
    [ -s "$SB/nested.env" ] && break
    kill -0 "$NESTED" 2>/dev/null || break
    sleep 0.2
done
[ -s "$SB/nested.env" ] || die "the nested niri did not start:
$(tail -n 20 "$SB/niri.log")"
N_SOCKET=$(sed -n 's/^NIRI_SOCKET=//p' "$SB/nested.env")
N_WAYLAND=$(sed -n 's/^WAYLAND_DISPLAY=//p' "$SB/nested.env")
[ -n "$N_SOCKET" ] && [ -n "$N_WAYLAND" ] || die "the nested niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"
NENV=(env XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$N_WAYLAND" NIRI_SOCKET="$N_SOCKET")

for _ in $(seq 1 25); do
    WIN=$(python3 -c '
import sys
new = set(sys.argv[2].split()) - set(sys.argv[1].split())
print(new.pop() if len(new) == 1 else "")' "$before" "$(niri_windows)")
    [ -n "$WIN" ] && break
    sleep 0.2
done
[ -n "$WIN" ] || die "the nested niri's window never appeared on this niri"
echo "nested niri: window $WIN, sockets in $RT"

# ─── park it out of the way ──────────────────────────────────────────────────
# Floating, so its size is exactly what is set; on the last workspace of its
# monitor, which niri always keeps empty; and without the focus: --focus false
# leaves it on the workspace the window opened on, back on the window that had
# it before.
niri msg action move-window-to-floating --id "$WIN" >/dev/null ||
    die "niri would not float the nested niri's window (move-window-to-floating)"
niri msg action set-window-width --id "$WIN" "$OUT_W" >/dev/null ||
    die "niri would not set the nested niri's window width (set-window-width)"
niri msg action set-window-height --id "$WIN" "$OUT_H" >/dev/null ||
    die "niri would not set the nested niri's window height (set-window-height)"
spare=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next(w for w in get("windows") if w["id"] == int(sys.argv[1]))
spaces = get("workspaces")
output = next(s["output"] for s in spaces if s["id"] == win["workspace_id"])
print(max(s["idx"] for s in spaces if s["output"] == output))' "$WIN") ||
    die "cannot find a spare workspace for the nested niri"
niri msg action move-window-to-workspace --window-id "$WIN" --focus false "$spare" >/dev/null ||
    die "niri would not park the nested niri's window on workspace $spare (move-window-to-workspace)"

# Why the nested window can no longer be trusted, or nothing while it can.
# Nothing is only ever a good answer: when this niri cannot be asked, that is
# a reason of its own, never an empty one that reads as "all is well".
UNASKABLE="cannot ask this niri where the nested window is"
focus_reason() {
    python3 - "$WIN" 2>/dev/null <<'PY' || echo "$UNASKABLE"
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next((w for w in get("windows") if w["id"] == int(sys.argv[1])), None)
if win is None:
    print("the nested niri's window is gone")
elif win["is_focused"]:
    print("the nested niri's window has the focus")
else:
    space = next((s for s in get("workspaces") if s["id"] == win["workspace_id"]), None)
    if space is not None and space["is_active"]:
        print(f"the nested niri's workspace ({space['output']}, {space['idx']}) is on screen")
PY
}

for _ in $(seq 1 25); do
    [ -z "$(focus_reason)" ] && break
    sleep 0.2
done
why=$(focus_reason)
[ -z "$why" ] || die "could not park the nested niri: $why"

for _ in $(seq 1 25); do
    size=$("${NENV[@]}" niri msg -j outputs 2>/dev/null | python3 -c '
import json, sys
o = next(iter(json.load(sys.stdin).values()))["logical"]
print(o["width"], o["height"])' 2>/dev/null)
    [ "$size" = "$OUT_W $OUT_H" ] && break
    sleep 0.2
done
[ "$size" = "$OUT_W $OUT_H" ] || die "the nested niri's output is ${size:-unknown}, not ${OUT_W}x${OUT_H}"
ok "the nested niri is parked unfocused on a spare workspace, ${OUT_W}x${OUT_H}"

# From here on, the window must stay unfocused and off screen. This watches
# for the whole run; guard ends the run the moment it has seen otherwise.
# One failed question is a hiccup; three in a row is a niri that cannot be
# trusted to answer.
watch_focus() {
    local why blind=0
    while sleep 0.25; do
        why=$(focus_reason)
        if [ "$why" = "$UNASKABLE" ]; then
            blind=$((blind+1))
            [ "$blind" -ge 3 ] || continue
        else
            blind=0
        fi
        [ -n "$why" ] && { echo "$why" > "$SB/tampered"; return; }
    done
}
watch_focus &
WATCH=$!
guard() {
    [ -s "$SB/tampered" ] || return 0
    bad "$(cat "$SB/tampered") — anything typed there would reach the panel,
      so its frames can no longer be trusted. Keep off that workspace while
      this runs"
    summary
    exit 1
}

# ─── its own daemon ──────────────────────────────────────────────────────────
add() {  # description
    task rc.verbose=nothing rc.confirmation=no add "+$TAG" -- "$1" >/dev/null 2>&1
}
settle() { sleep 2; }  # the daemon ticks every 700ms

TAG=$("${NENV[@]}" "$NIRITASKS" tag 2>/dev/null)
if [ "$TAG" = e2e ]; then
    ok "the nested niri's workspace files its tasks under e2e"
else
    bad "the nested niri's workspace reads as tag '${TAG}', expected e2e"
    summary; exit 1
fi

"${NENV[@]}" "$NIRITASKS" daemon >"$SB/daemon.err" 2>&1 &
DAEMON=$!
sleep 3
kill -0 "$DAEMON" 2>/dev/null || die "the daemon exited at once:
$(tail -n 20 "$SB/daemon.err")"

# ─── the real daemon, untouched ──────────────────────────────────────────────
guard
now=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)
if [ "$now" = "$SERVICE_PID" ]; then
    ok "niri-tasks.service was left alone (main pid ${now})"
else
    bad "niri-tasks.service changed under the run: main pid ${SERVICE_PID} before, ${now} after"
fi

summary
[ "$fail" -eq 0 ]
