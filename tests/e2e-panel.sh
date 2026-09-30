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
# So this measures pixels. Every frame is compared against a baseline taken with
# no tasks, and what is measured is the region on the right edge that differs:
# that region is the peek. It runs its own daemon against a sandboxed TASKDATA
# and never touches the real task database.
#
# What it cannot check is the hover: nothing on this machine can move the
# pointer, so the slide out, the slide back, and clicks passing beside the peek
# are checked by hand (README, "Testing").
#
# It needs the right edge of the screen to hold still, so it checks that first
# and says so rather than producing a flaky answer. Do not switch workspaces
# while it runs: the panel follows the workspace, and so does the tag its tasks
# are filed under. And not over a fullscreen window, which covers the panel.
#
# Park the pointer away from the right edge first. A pointer resting against
# the edge is inside the panel once it grows tall enough, and the panel slides
# out for it exactly as it should — which reads here as a peek 300px wide.
set -uo pipefail

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }
python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

NIRITASKS="${NIRITASKS:-niritasks}"

# Mirrors PEEK_PX in src/panel/surface.rs.
PEEK=30

SB="$(mktemp -d)"
mkdir -p "$SB/data" "$SB/shots"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

DAEMON=""
WAS_ACTIVE=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
cleanup() {
    [ -n "$DAEMON" ] && kill "$DAEMON" 2>/dev/null
    [ "$WAS_ACTIVE" = active ] && systemctl --user start niri-tasks.service 2>/dev/null
    if [ -n "${NIRITASKS_E2E_KEEP:-}" ]; then
        echo "frames kept in $SB/shots"
    else
        rm -rf "$SB"
    fi
}
trap cleanup EXIT INT TERM

pass=0; fail=0
ok()  { echo "  PASS  $*"; pass=$((pass+1)); }
bad() { echo "  FAIL  $*"; fail=$((fail+1)); }

# The panel on the focused monitor shows that monitor's workspace, which is the
# focused workspace — so its tasks have to carry the focused workspace's tag.
TAG=$("$NIRITASKS" tag 2>/dev/null)
[ -n "$TAG" ] || { echo "this workspace has no name, so the panel has nothing to show" >&2; exit 1; }

# Where niri drops screenshots. There is no option to write one somewhere else,
# so the file it makes is moved into the sandbox and nothing already in that
# directory is read or removed.
SHOTDIR=$(python3 - <<'PY'
import os, pathlib
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
    # niri names the file before it finishes writing it.
    sleep 0.5
    mv "$SHOTDIR/$new" "$SB/shots/$label.png"
}

# Compare a frame against the baseline and print "<width> <height>" of the
# panel: the span of columns, and of rows, that changed over most of a run.
#
# Counting changed pixels does not work: the cards are translucent, so much of
# them differs from the wallpaper by only a few levels, while a window
# redrawing under the strip differs by a lot. What separates them is shape. A
# card is a solid block, so every column through the peek changes down most of
# a card's height and every row through it changes across most of the peek,
# while noise changes a handful of pixels. Columns and rows over MIN_RUN are
# panel and nothing else.
measure() {
    python3 - "$SB/shots/baseline.png" "$SB/shots/$1.png" <<'PY'
import sys
from PIL import Image, ImageChops

SENSITIVITY = 6   # levels of difference that count as changed at all
MIN_RUN = 20      # changed pixels in a line before it is panel rather than noise

base = Image.open(sys.argv[1]).convert("RGB")
frame = Image.open(sys.argv[2]).convert("RGB")
w, h = base.size
# The right edge, well wider than the peek so an overlong one is seen, and a
# band across the middle, where a vertically centred panel sits.
#
# The band is kept narrow on purpose. Every screenshot this takes makes niri
# post a "Screenshot captured" notification, and they stack down from the top
# right — into the very strip being measured, by the third shot, if it runs
# much above the middle. Three cards fit in this band with room to spare.
box = (w - 300, int(h * 0.36), w, int(h * 0.64))
mask = (ImageChops.difference(base.crop(box), frame.crop(box))
        .convert("L")
        .point(lambda p: 255 if p > SENSITIVITY else 0))
px = mask.load()
cw, ch = mask.size
cols = [x for x in range(cw) if sum(1 for y in range(ch) if px[x, y]) >= MIN_RUN]
rows = [y for y in range(ch) if sum(1 for x in range(cw) if px[x, y]) >= MIN_RUN]
width = (cols[-1] - cols[0] + 1) if cols else 0
height = (rows[-1] - rows[0] + 1) if rows else 0
print(width, height)
PY
}

add() {  # description
    task rc.verbose=nothing rc.confirmation=no add "+$TAG" -- "$1" >/dev/null 2>&1
}
settle() { sleep 2; }  # the daemon ticks every 700ms

# Our own daemon, against the sandbox. The real one has to go first or two
# panels would draw on top of each other.
systemctl --user stop niri-tasks.service 2>/dev/null
sleep 1
"$NIRITASKS" daemon >"$SB/daemon.err" 2>&1 &
DAEMON=$!
sleep 3

echo "workspace tag: $TAG   screenshots via $SHOTDIR"

# ─── baseline, and whether it can be trusted ─────────────────────────────────
shot baseline || exit 1
shot stillness || exit 1
read -r noise _ < <(measure stillness)
if [ "$noise" -eq 0 ]; then
    ok "the right edge holds still between two identical frames"
else
    bad "the right edge is not static ($noise columns changed between two
      identical frames) — move or close whatever is animating there, or this
      measures that instead of the panel"
    echo; echo "passed: $pass   failed: $fail"; exit 1
fi

# ─── one task: a peek on the right edge ──────────────────────────────────────
add "ship it"
settle
shot one || exit 1
read -r one_w one_h < <(measure one)
if [ "$one_w" -ge $((PEEK - 20)) ] && [ "$one_w" -le $((PEEK + 30)) ]; then
    ok "one task pokes out a peek (${one_w}px wide, ~${PEEK}px expected)"
else
    bad "the panel is ${one_w}px wide on the right edge, expected ~${PEEK}px —
      0 means nothing drew; much more means it is not tucked away.
      NIRITASKS_E2E_KEEP=1 keeps the frames to tell which"
fi

# ─── three tasks: taller, same peek ──────────────────────────────────────────
add "write the glossary"
add "a much longer description that has to ellipsise rather than wrap onto a second line"
settle
shot three || exit 1
read -r three_w three_h < <(measure three)
if [ "$one_h" -gt 0 ] && [ "$three_h" -gt $((one_h * 2)) ]; then
    ok "three tasks stack three cards (${three_h}px tall vs ${one_h}px for one)"
else
    bad "three tasks drew ${three_h}px of panel against ${one_h}px for one —
      the cards are not stacking, or a long one wrapped"
fi
delta=$(( three_w > one_w ? three_w - one_w : one_w - three_w ))
[ "$delta" -le 6 ] && ok "and the peek stays the same width (${three_w}px)" \
    || bad "the peek changed width with the task count: ${three_w}px vs ${one_w}px —
      if it is ~300px the panel slid out, which is what it does when the pointer
      is resting against the right edge; move the pointer away and run again"

# ─── a task on another tag stays off this panel ──────────────────────────────
task rc.verbose=nothing rc.confirmation=no add +niritasks_e2e_elsewhere -- "not here" >/dev/null 2>&1
settle
shot elsewhere || exit 1
read -r _ other_h < <(measure elsewhere)
delta=$(( other_h > three_h ? other_h - three_h : three_h - other_h ))
[ "$delta" -le 6 ] && ok "a task on another workspace's tag does not appear" \
    || bad "the panel changed height (${other_h}px vs ${three_h}px) for a task on another tag"

# ─── nothing pending shows nothing ───────────────────────────────────────────
# rc.bulk=0: completing more than two tasks at once otherwise stops to ask,
# and with no terminal to answer, completes none of them.
task rc.verbose=nothing rc.confirmation=no rc.bulk=0 "+$TAG" done </dev/null >/dev/null 2>&1
settle
shot empty || exit 1
read -r empty_w _ < <(measure empty)
[ "$empty_w" -eq 0 ] && ok "the panel goes away when the workspace has no tasks" \
    || bad "something is still drawn with no tasks (${empty_w}px wide)"

# ─── the map-once trap: a daemon that starts with nothing to show ────────────
# A layer surface that has never been mapped does not respond to a later
# present(), so this once stayed invisible for an entire session however many
# tasks were added afterwards.
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
sleep 1
"$NIRITASKS" daemon >"$SB/daemon2.err" 2>&1 &
DAEMON=$!
sleep 3
add "after a cold start"
settle
shot cold_start || exit 1
read -r cold_w _ < <(measure cold_start)
if [ "$cold_w" -gt 0 ]; then
    ok "a daemon started with nothing to show still shows the next task (${cold_w}px)"
else
    bad "nothing appeared after a cold start with no tasks"
fi

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
