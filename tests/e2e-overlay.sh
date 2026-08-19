#!/usr/bin/env bash
# End-to-end test of the active-task overlay: task state -> pixels on screen.
#
#   bash tests/e2e-overlay.sh
#
# Not a cargo test, and cannot be. The overlay is a layer-shell surface whose
# whole behaviour is what the compositor puts on screen: whether a pill is
# there, and how wide it is. There is no in-process answer to either — the
# window reports the size it asked for, not the size it got, which is precisely
# the thing that has been wrong twice.
#
# Both times it was wrong in a way every unit test agreed was fine:
#
#   * The pill was pinned to 80 characters whatever the task said, because
#     width-chars was set alongside max-width-chars and is a minimum as well as
#     a maximum. "ship it" sat in a band of empty pill.
#   * Then, once it hugged its text, it kept the *previous* task's width — a
#     layer surface negotiates its size when it maps and is not asked again — so
#     a long description ellipsised down to "mak…" inside a pill cut for a short
#     one.
#
# So this measures pixels. Every frame is compared against a baseline taken with
# no task active, and what is measured is the width of the region that differs:
# that region is the pill. It runs its own daemon against a sandboxed TASKDATA
# and never touches the real task database.
#
# It needs the area under the pill to hold still, so it checks that first and
# says so rather than producing a flaky answer. Do not switch workspaces while
# it runs: the overlay follows the focused workspace, and so does the tag its
# tasks are filed under.
set -uo pipefail

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }
python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

WT="${WT:-wt}"

# How far above the bottom edge the pill sits. Everything below measures a strip
# of screen, and a strip in the wrong place finds nothing however well the
# overlay is working — so this is resolved once, here, and used for both halves.
#
# Exported, which is the part that matters: the daemon under test is the one
# this script starts a few lines down, so it reads this same value out of this
# same environment. The band and the pill therefore move together, whatever the
# margin is, and neither depends on what the installed systemd unit happens to
# set. The default mirrors DEFAULT_BOTTOM_MARGIN in src/overlay.rs.
MARGIN="${WT_OVERLAY_MARGIN:-10}"
export WT_OVERLAY_MARGIN="$MARGIN"

SB="$(mktemp -d)"
mkdir -p "$SB/data" "$SB/shots"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

DAEMON=""
WAS_ACTIVE=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
cleanup() {
    [ -n "$DAEMON" ] && kill "$DAEMON" 2>/dev/null
    [ "$WAS_ACTIVE" = active ] && systemctl --user start niri-tasks.service 2>/dev/null
    if [ -n "${WT_E2E_KEEP:-}" ]; then
        echo "frames kept in $SB/shots"
    else
        rm -rf "$SB"
    fi
}
trap cleanup EXIT INT TERM

pass=0; fail=0
ok()  { echo "  PASS  $*"; pass=$((pass+1)); }
bad() { echo "  FAIL  $*"; fail=$((fail+1)); }

# The overlay shows the focused workspace's active task, so its tasks have to
# carry the focused workspace's tag.
TAG=$("$WT" tag 2>/dev/null)
[ -n "$TAG" ] || { echo "this workspace has no name, so the overlay has nothing to show" >&2; exit 1; }

# Where niri drops screenshots. There is no option to write one somewhere else,
# so the file it makes is moved into the sandbox and nothing already in that
# directory is read or removed.
SHOTDIR=$(python3 - <<'PY'
import os, re, pathlib
cfg = pathlib.Path.home() / ".config/niri/config.kdl"
path = "~/Pictures/Screenshots/x.png"
if cfg.is_file():
    for line in cfg.read_text().splitlines():
        line = line.strip()
        if line.startswith("screenshot-path") and '"' in line:
            path = line.split('"')[1]
            break
print(os.path.dirname(os.path.expanduser(path)))
PY
)
[ -d "$SHOTDIR" ] || { echo "screenshot directory $SHOTDIR does not exist" >&2; exit 1; }

# Take a screenshot and claim only the file it produced.
shot() {
    local label="$1" before after new
    before=$(mktemp); after=$(mktemp)
    ls -1 "$SHOTDIR" > "$before" 2>/dev/null
    niri msg action screenshot-screen >/dev/null 2>&1
    for _ in $(seq 1 30); do
        ls -1 "$SHOTDIR" > "$after" 2>/dev/null
        new=$(comm -13 "$before" "$after" | head -1)
        [ -n "$new" ] && break
        sleep 0.2
    done
    rm -f "$before" "$after"
    [ -n "$new" ] || { bad "no screenshot appeared in $SHOTDIR"; return 1; }
    mv "$SHOTDIR/$new" "$SB/shots/$label.png"
}

# Compare a frame against the baseline and print "<pill columns> <pill width>".
#
# Counting changed pixels does not work: the pill is translucent, so most of it
# differs from the wallpaper by only a few levels, while a window redrawing
# somewhere else under the strip differs by a lot. Both a faint pill and a
# bright unrelated change land in the same number.
#
# What separates them is shape. The pill is a solid band about 33px tall, so
# every column inside it changes down most of its height, while noise — a glyph
# repainting, an artefact — changes a handful of pixels in a column at most.
# Counting columns that change over at least MIN_RUN of their height finds the
# pill and nothing else, and the distance between the first and last of them is
# its width.
measure() {
    python3 - "$SB/shots/baseline.png" "$SB/shots/$1.png" "$MARGIN" <<'PY'
import sys
from PIL import Image, ImageChops

SENSITIVITY = 6   # levels of difference that count as changed at all
MIN_RUN = 20      # changed pixels in a column before it is pill rather than noise

base = Image.open(sys.argv[1]).convert("RGB")
frame = Image.open(sys.argv[2]).convert("RGB")
margin = int(sys.argv[3])
w, h = base.size
# The strip the pill lives in, kept clear of the bar's workspace pills on the
# left and its tray and clock on the right.
#
# Vertically it is placed relative to the pill's own bottom edge, which sits
# `margin` px up from the bottom of the screen — not at a fixed height, which
# would be reading the default margin of 10 off the screen and calling it a
# measurement. With a taller bar and a larger WT_OVERLAY_MARGIN a fixed strip
# looks straight past the pill, and every check downstream reports "no pill
# appeared" — a failure that blames the overlay for the ruler being in the
# wrong place.
box = (int(w * 0.16), h - margin - 85, int(w * 0.86), h - margin - 10)
mask = (ImageChops.difference(base.crop(box), frame.crop(box))
        .convert("L")
        .point(lambda p: 255 if p > SENSITIVITY else 0))
px = mask.load()
cw, ch = mask.size
cols = [x for x in range(cw) if sum(1 for y in range(ch) if px[x, y]) >= MIN_RUN]
print(len(cols), (cols[-1] - cols[0] + 1) if cols else 0)
PY
}

set_active() {  # description substring, or nothing to clear
    task rc.verbose=nothing rc.confirmation=no "+$TAG" +ACTIVE stop >/dev/null 2>&1
    [ -n "${1:-}" ] && task rc.verbose=nothing rc.confirmation=no "/$1/" start >/dev/null 2>&1
    sleep 2   # the overlay ticks every 700ms
}

SHORT="ship it"
LONG="a much longer description that should stretch the pill out towards its ceiling"
task rc.verbose=nothing rc.confirmation=no add "+$TAG" -- "$SHORT" >/dev/null 2>&1
task rc.verbose=nothing rc.confirmation=no add "+$TAG" -- "$LONG"  >/dev/null 2>&1

# Our own daemon, against the sandbox. The real one has to go first or two
# overlays would draw on top of each other.
systemctl --user stop niri-tasks.service 2>/dev/null
sleep 1
"$WT" daemon >"$SB/daemon.err" 2>&1 &
DAEMON=$!
sleep 3

SCREEN_W=$(niri msg -j outputs | python3 -c "
import json,sys
outs = json.load(sys.stdin)
print(max(o['logical']['width'] for o in outs.values() if o.get('logical')))
")
echo "workspace tag: $TAG   screen ${SCREEN_W}px   margin ${MARGIN}px"
echo "measuring the strip $((MARGIN + 85))-$((MARGIN + 10))px above the bottom edge, via $SHOTDIR"

# ─── baseline, and whether it can be trusted ─────────────────────────────────
set_active ""
shot baseline || exit 1
shot stillness || exit 1
read -r noise _ < <(measure stillness)
if [ "$noise" -eq 0 ]; then
    ok "the area under the pill holds still between two identical frames"
else
    bad "the area under the pill is not static ($noise columns changed between
      two identical frames) — move or close whatever is animating at the bottom
      of the screen, or this measures that instead of the overlay"
    echo; echo "passed: $pass   failed: $fail"; exit 1
fi

# ─── a short task gets a short pill ──────────────────────────────────────────
set_active "$SHORT"
shot short || exit 1
read -r cols short_width < <(measure short)
if [ "$cols" -ge 10 ]; then
    ok "a pill appears for the active task (${short_width}px wide)"
else
    bad "no pill appeared for the active task ($cols pill columns found) —
      either the overlay drew nothing, or it is not in the strip being measured
      ($((MARGIN + 85))-$((MARGIN + 10))px above the bottom edge, from
      WT_OVERLAY_MARGIN=$MARGIN); WT_E2E_KEEP=1 keeps the frames to tell which"
fi

band=$((SCREEN_W / 4))
if [ "$short_width" -lt "$band" ]; then
    ok "and a two-word task gets a small one, not a band (under ${band}px)"
else
    bad "a two-word task drew a ${short_width}px pill — wider than a quarter of
      the screen, which is the width-chars bug back again"
fi

# ─── a long one grows, and ellipsises rather than running off ────────────────
set_active "$LONG"
shot long || exit 1
read -r _ long_width < <(measure long)
if [ "$long_width" -gt $((short_width + 100)) ]; then
    ok "a longer task makes a wider pill (${long_width}px vs ${short_width}px)"
else
    bad "the pill did not grow: ${long_width}px vs ${short_width}px"
fi

limit=$((SCREEN_W * 9 / 10))
if [ "$long_width" -lt "$limit" ]; then
    ok "and stops short of the screen edge (${long_width}px, under ${limit}px)"
else
    bad "the pill ran to ${long_width}px, past the ${limit}px ceiling — the max
      width is not being applied"
fi

# ─── and shrinks back, which is the half that regressed ──────────────────────
set_active "$SHORT"
shot short_again || exit 1
read -r _ back_width < <(measure short_again)
delta=$(( back_width > short_width ? back_width - short_width : short_width - back_width ))
if [ "$long_width" -le "$short_width" ]; then
    bad "cannot tell whether the pill shrinks back: it never grew, so this
      check would pass whatever the code did"
elif [ "$delta" -le 6 ]; then
    ok "and shrinks back to the short pill (${back_width}px vs ${short_width}px)"
else
    bad "the pill kept the long task's width: ${back_width}px, expected ~${short_width}px"
fi

# ─── nothing active shows nothing ────────────────────────────────────────────
set_active ""
shot hidden || exit 1
read -r cols _ < <(measure hidden)
[ "$cols" -eq 0 ] && ok "the pill goes away when no task is active" \
    || bad "something is still drawn with no active task ($cols pill columns)"

# ─── the map-once trap: a daemon that starts with nothing active ─────────────
# A layer surface that has never been mapped does not respond to a later
# present(), so this once stayed invisible for an entire session however many
# tasks were started afterwards.
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
sleep 1
"$WT" daemon >"$SB/daemon2.err" 2>&1 &
DAEMON=$!
sleep 3
set_active "$SHORT"
shot cold_start || exit 1
read -r cols cold_width < <(measure cold_start)
if [ "$cols" -ge 10 ]; then
    ok "a daemon started with nothing active still shows the next task (${cold_width}px)"
else
    bad "nothing appeared after a cold start with no active task ($cols pill columns)"
fi

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
