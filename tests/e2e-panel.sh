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
# So this measures pixels, on a screen of its own. It starts a nested niri
# (tests/lib/nested-niri.sh, which e2e-box.sh shares) — a window on yours, with
# its own runtime dir, sockets and one 1600x1000 output of flat colour — and
# runs its own daemon in it against a sandboxed TASKDATA. Nothing on your
# desktop can reach those frames: not the wallpaper, not a terminal redrawing,
# not a window rule or a notification or the pointer. Your own niri-tasks
# daemon keeps running throughout and never sees these tasks.
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
# middle of the screen, and with wtype, Down (which moves the action row), the
# filter tabs' keys and Escape are pressed in the nested niri, never on your
# desktop.
set -uo pipefail

python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

. "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"

# The nested niri's one output (tests/lib/nested-niri.sh), in pixels.
OUT_W=$NESTED_W
OUT_H=$NESTED_H
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

mkdir -p "$SB/shots"

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
# A line counts only when at least MIN_RUN of its pixels changed by more than
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

nested_start

TAG=$("${NENV[@]}" "$NIRITASKS" tag 2>/dev/null)
if [ "$TAG" = e2e ]; then
    ok "the nested niri's workspace files its tasks under e2e"
else
    bad "the nested niri's workspace reads as tag '${TAG}', expected e2e"
    summary; exit 1
fi

nested_daemon_start "$SB/daemon.err"

# ─── baseline: the nested screen with nothing on it ──────────────────────────
shot baseline || { summary; exit 1; }
read -r base_w base_h < <(frame_size baseline)
[ "$base_w $base_h" = "$OUT_W $OUT_H" ] ||
    die "the nested screen shoots at ${base_w}x${base_h}, not ${OUT_W}x${OUT_H} — every number below assumes it"

# ─── one task: a peek on the right edge ──────────────────────────────────────
add "ship it"
settle
shot one || { summary; exit 1; }
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
shot three || { summary; exit 1; }
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
shot elsewhere || { summary; exit 1; }
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
shot keyboard || { summary; exit 1; }
read -r x0 x1 y0 y1 < <(measure keyboard)
keyboard_h=$((y1 - y0))
if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
    ok "the keyboard takes the panel off the right edge to the middle (columns ${x0}-${x1})"
else
    bad "with the keyboard the panel covers columns ${x0}-${x1}, expected within
      ${SURFACE_LEFT}-${SURFACE_RIGHT} — 0-0 means nothing drew; reaching ${OUT_W} means it
      did not leave the edge"
fi

# Down moves the darker fill from the first card to the second, and the action
# row with it: only the focused card has one. So what differs between the two
# frames is those two cards' columns, and the panel is as tall as it was.
if command -v wtype >/dev/null; then
    # None of these tasks is started, planned or waiting, so only All and To
    # refine have a tab. 3 is Planned's key, and its tab is hidden: nothing.
    "${NENV[@]}" wtype 3
    sleep 1
    shot tab_hidden || { summary; exit 1; }
    if same keyboard tab_hidden; then
        ok "3 does nothing while no task is planned, its tab hidden"
    else
        bad "3 changed the panel with no planned task; Planned's tab should be hidden"
    fi

    # 4 is To refine: the same three cards, so only the tab bar changes, the
    # picked tab's fill moving off All.
    "${NENV[@]}" wtype 4
    sleep 1
    shot tab_refine || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure tab_refine keyboard)
    if [ "$y1" -gt 0 ] && [ "$((y1 - y0))" -lt 60 ]; then
        ok "4 picks To refine, moving only the tab bar's fill (rows ${y0}-${y1})"
    else
        bad "4 changed rows ${y0}-${y1} of the keyboard's panel — 0-0 means it did
      nothing; more than the tab bar means the cards changed too"
    fi

    # ] from All skips the hidden Active and Planned to To refine, and stops
    # there, Waiting being hidden too; [ goes back to All.
    "${NENV[@]}" wtype 1
    "${NENV[@]}" wtype -k bracketright
    sleep 1
    shot tab_right || { summary; exit 1; }
    if same tab_refine tab_right; then
        ok "] from All skips the hidden tabs to To refine"
    else
        bad "] from All did not land on To refine, the next tab shown"
    fi
    "${NENV[@]}" wtype -k bracketright
    sleep 1
    shot tab_end || { summary; exit 1; }
    if same tab_refine tab_end; then
        ok "] on the last tab shown stays there"
    else
        bad "] on To refine moved; it is the last tab shown and should stop"
    fi
    "${NENV[@]}" wtype -k bracketleft
    sleep 1
    shot tab_left || { summary; exit 1; }
    if same keyboard tab_left; then
        ok "[ steps back to All, as the panel opened"
    else
        bad "[ from To refine is not All as the panel opened"
    fi

    "${NENV[@]}" wtype -k Down
    sleep 1
    shot keyboard_down || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure keyboard_down keyboard)
    # The two cards change height as the row moves, and each one's ring with
    # it, so the columns that differ take in the rings either side.
    if [ "$x0" -eq $((CARD_X - RING)) ] && [ "$x1" -eq $((CARD_X + CARD + RING)) ]; then
        ok "and shows its cards in the middle of the screen (columns ${x0}-${x1}, rings included)"
    else
        bad "the keyboard's cards cover columns ${x0}-${x1}, expected
      $((CARD_X - RING))-$((CARD_X + CARD + RING)) — 0-0 means Down did not move the focus"
    fi
    read -r x0 x1 y0 y1 < <(measure keyboard_down)
    if [ "$((y1 - y0))" -eq "$keyboard_h" ]; then
        ok "and Down moves the buttons to the next card, the panel as tall as before (${keyboard_h}px)"
    else
        bad "after Down the panel is $((y1 - y0))px tall against ${keyboard_h}px before —
      the action row should move to the focused card, not be added or lost"
    fi

    # Escape from a tab other than All: the hover panel has no tabs, so it
    # comes back exactly as before, and the next keyboard opens on All.
    "${NENV[@]}" wtype 4
    sleep 1
    "${NENV[@]}" wtype -k Escape
    settle
    shot released || { summary; exit 1; }
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi

    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot reopened || { summary; exit 1; }
    if same keyboard reopened; then
        ok "taking the keyboard again opens on All, whatever tab it was left on"
    else
        bad "the keyboard reopened on something other than All with the first card focused"
    fi

    # Park one as waiting while the panel is open: All loses it and the
    # Waiting tab comes up, and 5 shows it there alone.
    task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:ship it" \
        modify wait:someday </dev/null >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 5
    sleep 1
    shot tab_waiting || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure tab_waiting)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ] &&
        [ "$((y1 - y0))" -lt "$keyboard_h" ]; then
        ok "5 shows the waiting task alone, shorter (${keyboard_h}px to $((y1 - y0))px)"
    else
        bad "on the Waiting tab the panel covers columns ${x0}-${x1}, rows ${y0}-${y1},
      against ${keyboard_h}px for three cards — the tab is missing or shows more"
    fi
    # The Waiting tab's one card has its row, as the focused card on All did;
    # All's two other cards have none. So All is taller by exactly two
    # one-line cards and their gaps, as three tucked cards are than one.
    waiting_h=$((y1 - y0))
    extra=$((keyboard_h - waiting_h - (three_h - one_h)))
    if [ "$extra" -ge -2 ] && [ "$extra" -le 2 ]; then
        ok "only the focused card shows its buttons (All ${keyboard_h}px, Waiting ${waiting_h}px)"
    else
        bad "All's three cards are ${keyboard_h}px against ${waiting_h}px for the Waiting
      tab's one, ${extra}px off two one-line cards ($((three_h - one_h))px) — the unfocused
      cards still show their action rows"
    fi

    # Off the keyboard, the waiting task is off the hover panel too: two
    # cards tucked away, where there were three.
    "${NENV[@]}" wtype -k Escape
    settle
    shot parked || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure parked)
    if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ] && [ "$((y1 - y0))" -lt "$three_h" ]; then
        ok "a waiting task leaves the tucked panel (${three_h}px to $((y1 - y0))px)"
    else
        bad "with one task waiting the tucked panel covers columns ${x0}-${x1}, rows
      ${y0}-${y1}, against ${three_h}px for three — the waiting task is still on it"
    fi
else
    skip "the panel's cards in the middle of the screen, their buttons, the filter tabs, and Escape back (needs wtype)"
fi

# ─── nothing pending shows nothing ───────────────────────────────────────────
# rc.bulk=0: completing more than two tasks at once otherwise stops to ask,
# and with no terminal to answer, completes none of them. A panel with no
# tasks also gives the keyboard back, when wtype was not there to press Escape.
task rc.verbose=nothing rc.confirmation=no rc.bulk=0 "+$TAG" done </dev/null >/dev/null 2>&1
settle
shot empty || { summary; exit 1; }
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
nested_daemon_stop
sleep 1
nested_daemon_start "$SB/daemon2.err"
add "after a cold start"
settle
shot cold_start || { summary; exit 1; }
read -r x0 x1 _ _ < <(measure cold_start)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "a daemon started with nothing to show still shows the next task (columns ${x0}-${x1})"
else
    bad "after a cold start with no tasks the panel drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
fi

# ─── the real daemon, untouched ──────────────────────────────────────────────
nested_service_untouched

summary
[ "$fail" -eq 0 ]
