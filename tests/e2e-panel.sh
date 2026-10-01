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

# Whether a frame loads: niri names the file before it has finished writing it.
loads() {
    python3 -c 'import sys; from PIL import Image; Image.open(sys.argv[1]).load()' "$1" 2>/dev/null
}

# Whether two frames are the same, pixel for pixel.
same() {  # label label
    python3 - "$SB/shots/$1.png" "$SB/shots/$2.png" <<'PY'
import sys
from PIL import Image, ImageChops
a, b = (Image.open(p).convert("RGB") for p in sys.argv[1:3])
sys.exit(0 if a.size == b.size and ImageChops.difference(a, b).getbbox() is None else 1)
PY
}

# Shoot the nested screen, and keep the frame once two in a row match. Parked
# off screen, the nested niri draws only when asked, so the first frame after
# a change can still show the last one, or the panel half way through sliding.
shot() {  # label
    local label="$1" n
    guard
    rm -f "$SB/shots/.prev.png"
    for n in $(seq 1 10); do
        rm -f "$SB/shots/.this.png"
        "${NENV[@]}" niri msg action screenshot-screen --show-pointer false \
            --path "$SB/shots/.this.png" >/dev/null 2>&1
        for _ in $(seq 1 25); do loads "$SB/shots/.this.png" && break; sleep 0.1; done
        loads "$SB/shots/.this.png" || { bad "the nested niri wrote no screenshot for $label"; return 1; }
        if [ "$n" -gt 1 ] && same .prev .this; then
            mv "$SB/shots/.this.png" "$SB/shots/$label.png"
            rm -f "$SB/shots/.prev.png"
            guard
            return 0
        fi
        mv "$SB/shots/.this.png" "$SB/shots/.prev.png"
        sleep 0.3
    done
    bad "the nested screen never held still for $label: ten frames, no two alike"
    return 1
}

frame_size() {  # label
    python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' "$SB/shots/$1.png"
}

# Where a frame differs from another (the baseline unless given), as
# "<x0> <x1> <y0> <y1>": the first column that changed and one past the last,
# the same for rows; "0 0 0 0" when nothing did.
#
# A line counts only when more than MIN_RUN of its pixels changed by more than
# SENSITIVITY levels. The cards' shadows fade into the background over many
# pixels, and these two numbers are what put the panel's edge at the same
# column every run; the exact expectations below were measured with them.
measure() {  # label [against]
    python3 - "$SB/shots/${2:-baseline}.png" "$SB/shots/$1.png" <<'PY'
import sys
from PIL import Image, ImageChops

SENSITIVITY = 6   # levels of difference that count as changed at all
MIN_RUN = 20      # changed pixels in a line before it is panel

base = Image.open(sys.argv[1]).convert("RGB")
frame = Image.open(sys.argv[2]).convert("RGB")
mask = (ImageChops.difference(base, frame)
        .convert("L")
        .point(lambda p: 255 if p > SENSITIVITY else 0))

def lines(img):
    w, h = img.size
    data = img.tobytes()
    return [y for y in range(h) if data[y * w:(y + 1) * w].count(255) >= MIN_RUN]

rows = lines(mask)
cols = lines(mask.transpose(Image.Transpose.TRANSPOSE))
if cols and rows:
    print(cols[0], cols[-1] + 1, rows[0], rows[-1] + 1)
else:
    print(0, 0, 0, 0)
PY
}

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

# ─── baseline: the nested screen with nothing on it ──────────────────────────
shot baseline || exit 1
read -r base_w base_h < <(frame_size baseline)
[ "$base_w $base_h" = "$OUT_W $OUT_H" ] ||
    die "the nested screen shoots at ${base_w}x${base_h}, not ${OUT_W}x${OUT_H} — every number below assumes it"

# ─── one task: a peek on the right edge ──────────────────────────────────────
add "ship it"
settle
shot one || exit 1
read -r x0 x1 y0 y1 < <(measure one)
one_h=$((y1 - y0))
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "one task pokes out a ${PEEK}px peek and its ${RING}px ring (columns ${x0}-${x1})"
else
    bad "one task drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W} —
      0-0 means nothing drew; an earlier start means it is not tucked away.
      NIRITASKS_E2E_KEEP=1 keeps the frames to tell which"
fi

# ─── three tasks: taller, same peek ──────────────────────────────────────────
add "write the glossary"
add "a much longer description that has to ellipsise rather than wrap onto a second line"
settle
shot three || exit 1
read -r x0 x1 y0 y1 < <(measure three)
three_h=$((y1 - y0))
if [ "$one_h" -gt 0 ] && [ "$three_h" -gt $((one_h * 2)) ]; then
    ok "three tasks stack three cards (${three_h}px tall vs ${one_h}px for one)"
else
    bad "three tasks drew ${three_h}px of panel against ${one_h}px for one —
      the cards are not stacking, or a long one wrapped"
fi
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "and the peek stays the same width (columns ${x0}-${x1})"
else
    bad "the peek moved with the task count: columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
fi

# ─── a task on another tag stays off this panel ──────────────────────────────
task rc.verbose=nothing rc.confirmation=no add +niritasks_e2e_elsewhere -- "not here" >/dev/null 2>&1
settle
shot elsewhere || exit 1
if same three elsewhere; then
    ok "a task on another workspace's tag changes nothing on screen"
else
    read -r x0 x1 y0 y1 < <(measure elsewhere three)
    bad "a task on another tag changed columns ${x0}-${x1}, rows ${y0}-${y1}"
fi

# ─── the keyboard: every card wrapped, with its buttons, mid-screen ──────────
# `task panel` is the keybind's command: it asks this sandbox's daemon to hand
# its panel the keyboard, and the panel leaves the right edge for the middle of
# the screen. Everything it draws is then inside the centred surface.
"${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
settle
shot keyboard || exit 1
read -r x0 x1 _ _ < <(measure keyboard)
if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
    ok "the keyboard takes the panel off the right edge to the middle (columns ${x0}-${x1})"
else
    bad "with the keyboard the panel covers columns ${x0}-${x1}, expected within
      ${SURFACE_LEFT}-${SURFACE_RIGHT} — 0-0 means nothing drew; reaching ${OUT_W} means it
      did not leave the edge"
fi

# Down moves the darker fill from the first card to the second, and nothing
# else in the frame changes, so what differs between the two frames is exactly
# two cards: their columns, and with their action rows, well over twice the
# height of one card's one-line peek.
if command -v wtype >/dev/null; then
    "${NENV[@]}" wtype -k Down
    sleep 1
    shot keyboard_down || exit 1
    read -r x0 x1 y0 y1 < <(measure keyboard_down keyboard)
    key_h=$((y1 - y0))
    if [ "$x0" -eq "$CARD_X" ] && [ "$x1" -eq $((CARD_X + CARD)) ]; then
        ok "and shows its cards in the middle of the screen (columns ${x0}-${x1})"
    else
        bad "the keyboard's cards cover columns ${x0}-${x1}, expected
      ${CARD_X}-$((CARD_X + CARD)) — 0-0 means Down did not move the focus"
    fi
    if [ "$key_h" -ge $((one_h * 2 + 40)) ]; then
        ok "and each card grows its buttons (${key_h}px for two cards vs ${one_h}px for one tucked away)"
    else
        bad "two of the keyboard's cards are ${key_h}px tall against ${one_h}px for
      one tucked away — the action rows are missing"
    fi

    "${NENV[@]}" wtype -k Escape
    settle
    shot released || exit 1
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi
else
    skip "the panel's cards in the middle of the screen, their buttons, and Escape back (needs wtype)"
fi

# ─── nothing pending shows nothing ───────────────────────────────────────────
# rc.bulk=0: completing more than two tasks at once otherwise stops to ask,
# and with no terminal to answer, completes none of them. A panel with no
# tasks also gives the keyboard back, when wtype was not there to press Escape.
task rc.verbose=nothing rc.confirmation=no rc.bulk=0 "+$TAG" done </dev/null >/dev/null 2>&1
settle
shot empty || exit 1
if same baseline empty; then
    ok "the panel goes away when the workspace has no tasks"
else
    read -r x0 x1 y0 y1 < <(measure empty)
    bad "something is still drawn with no tasks (columns ${x0}-${x1}, rows ${y0}-${y1})"
fi

# ─── the map-once trap: a daemon that starts with nothing to show ────────────
# A layer surface that has never been mapped does not respond to a later
# present(), so this once stayed invisible for an entire session however many
# tasks were added afterwards.
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
sleep 1
"${NENV[@]}" "$NIRITASKS" daemon >"$SB/daemon2.err" 2>&1 &
DAEMON=$!
sleep 3
add "after a cold start"
settle
shot cold_start || exit 1
read -r x0 x1 _ _ < <(measure cold_start)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "a daemon started with nothing to show still shows the next task (columns ${x0}-${x1})"
else
    bad "after a cold start with no tasks the panel drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
fi

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
