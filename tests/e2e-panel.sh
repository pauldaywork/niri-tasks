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
# at column 1567, the keyboard's cards span 420-1180, and a frame that should
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
# middle of the screen, and with wtype, Down (which moves the action row, and
# shows the rest on reaching "+N more"), the filter tabs' keys, Ctrl+Enter,
# Enter on a card's body (its notes), Ctrl+Shift+Delete and Enter for the
# Waiting tab's Clear all, c and m on a card, and Escape are pressed in the
# nested niri, never on your desktop.
#
# What a key does to the panel's state (which tab, which card has the focus,
# what is armed) is src/panel/state.rs's, and its unit tests check every rule
# of it. This checks that GTK draws what the state says and that what it runs
# reaches taskwarrior: one press of each kind, not every rule again.
set -uo pipefail

python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

. "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"
# The nested niri's spawns (what Ctrl+Enter runs) get a PATH with no herdr and
# no wt, so a refine or a start fails at once instead of opening anything real.
NESTED_SPAWN_PATH="$SB/bin:/usr/bin:/bin"

# The nested niri's one output (tests/lib/nested-niri.sh), in pixels.
OUT_W=$NESTED_W
OUT_H=$NESTED_H
# Mirror PEEK_PX, SURFACE_WIDTH and NOTES_MAX_PX in src/panel/layout.rs, and
# RING_PX, CARD_WIDTH_PX and PADDING_PX in src/panel/style.rs.
PEEK=30
RING=3
CARD=760
SURFACE=784
NOTES_MAX=240
PADDING=12
SHORT_NOTES_GROWTH=50
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
    # A fixed, old entry: an age counted from now could tick from 0m to 1m
    # between two frames compared for sameness. This one only moves weekly.
    task rc.verbose=nothing rc.confirmation=no add "+$TAG" entry:20260101T000000Z -- "$1" >/dev/null 2>&1
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

# Wait out the project list's blinking text cursor. The focused field blinks
# on every frame `shot` takes, so no two match and it gives up. GTK stops
# blinking, cursor shown, after gtk-cursor-blink-timeout (10 seconds, from
# the user's settings) with nothing typed. The setting itself cannot be
# turned off for the nested daemon alone: the portal's value beats a
# settings.ini, and GDK_DEBUG=no-portals changes the fonts along with it.
let_cursor_rest() { sleep 12; }

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

# Where a frame differs from another, in words for a bad line. A change of a
# few pixels in each line, such as a label's words, is too small for measure
# to place, and it prints "0 0 0 0" as if nothing changed; this says so.
whereabouts() {  # label against
    local x0 x1 y0 y1
    read -r x0 x1 y0 y1 < <(measure "$1" "$2")
    if [ "$x0 $x1 $y0 $y1" = "0 0 0 0" ]; then
        echo "a change too small for measure to place: fewer than 20 changed pixels in any line, such as a label's words"
    else
        echo "columns ${x0}-${x1}, rows ${y0}-${y1}"
    fi
}

nested_start

# The Ideas tab saves under XDG_DATA_HOME: the sandbox's, never the real
# ~/.local/share. Set before the daemon starts, so it has it.
NENV+=(XDG_DATA_HOME="$SB/share")

TAG=$("${NENV[@]}" "$NIRITASKS" tag 2>/dev/null)
if [ "$TAG" = e2e ]; then
    ok "the nested niri's workspace files its tasks under e2e"
else
    bad "the nested niri's workspace reads as tag '${TAG}', expected e2e"
    summary; exit 1
fi
IDEAS="$SB/share/niri-tasks/ideas/$TAG.md"
ideas() { cat "$IDEAS" 2>/dev/null; }

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
    # refine have a tab. 4 is To refine: the same three cards, so only the
    # tab bar changes, the picked tab's fill moving off All.
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

    # Back to All for Down.
    "${NENV[@]}" wtype 1
    sleep 1

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
    key_h=$((y1 - y0))
    read -r x0 x1 y0 y1 < <(measure keyboard_down)
    if [ "$((y1 - y0))" -eq "$keyboard_h" ]; then
        ok "and Down moves the buttons to the next card, the panel as tall as before (${keyboard_h}px)"
    else
        bad "after Down the panel is $((y1 - y0))px tall against ${keyboard_h}px before —
      the action row should move to the focused card, not be added or lost"
    fi
    # What differs spans card 1's top to card 2's bottom: two one-line cards,
    # a gap, and one action row. With no row on either it would be two cards
    # and a gap, a row's height short of this.
    if [ "$key_h" -ge $((three_h - one_h + 12)) ]; then
        ok "and the focused card shows its buttons (${key_h}px for two cards and one row, ${three_h}px for three tucked)"
    else
        bad "the two cards Down touched are ${key_h}px tall, against $((three_h - one_h))px for
      two one-line cards and their gaps — no card shows its action row"
    fi

    # Enter on the focused card's body shows its task's notes under the
    # description, and the panel grows to fit them; a second Enter hides
    # them. Every task gets a note first, so whichever card Down left the
    # focus on has one. Notes show only once pressed, so the re-render
    # the notes bring leaves the panel's size and place alone. The frames after
    # the notes are added are compared among themselves.
    for uuid in $(task "+$TAG" _uuids 2>/dev/null); do
        task rc.verbose=nothing rc.confirmation=no "$uuid" annotate -- "a note on this task" >/dev/null 2>&1
    done
    settle
    shot noted || { summary; exit 1; }
    keyed_at=$(measure keyboard_down)
    noted_at=$(measure noted)
    if [ "$noted_at" = "$keyed_at" ]; then
        ok "adding notes leaves the panel as it was"
    else
        bad "adding notes moved or resized the panel before any press: against the
      baseline it measured '${keyed_at}' before the notes and '${noted_at}' after"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot notes_shown || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure notes_shown)
    if [ "$((y1 - y0))" -gt "$keyboard_h" ]; then
        ok "Enter on the card's body shows its notes, the panel grown to fit (${keyboard_h}px to $((y1 - y0))px)"
    else
        bad "after Enter the panel is $((y1 - y0))px tall against ${keyboard_h}px before —
      the focused card's notes should show and the panel grow to fit"
    fi
    # One short note must not cost the card more than its lines: the id line
    # and the note, a padding over them, make up SHORT_NOTES_GROWTH, as
    # measured on 1efd4c6, before the notes were put in a scroller.
    if [ "$((y1 - y0 - keyboard_h))" -eq "$SHORT_NOTES_GROWTH" ]; then
        ok "one short note grows the card by exactly its lines ($((y1 - y0 - keyboard_h))px)"
    else
        bad "one short note grew the card $((y1 - y0 - keyboard_h))px, expected ${SHORT_NOTES_GROWTH}px —
      short notes should take their own height, as before the notes scrolled"
    fi
    settle
    shot notes_ticked || { summary; exit 1; }
    if same notes_shown notes_ticked; then
        ok "and the daemon's ticks keep them shown"
    else
        bad "the notes' frame changed over a tick with nothing changed"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot notes_hidden || { summary; exit 1; }
    if same noted notes_hidden; then
        ok "and a second Enter hides them, the panel back as it was before the press"
    else
        bad "after a second Enter the screen differs from before the press: $(whereabouts notes_hidden noted)"
    fi
    # Space on the card's body is meant to do what Enter does there: show the
    # notes, and hide them again on a second press, leaving the panel where
    # the checks after this expect it. The second press is held against
    # notes_hidden, not keyboard_down: the panel after the notes were added,
    # not the one before, is what the checks after this expect.
    "${NENV[@]}" wtype -k space
    sleep 1
    shot notes_space || { summary; exit 1; }
    if same notes_shown notes_space; then
        ok "Space on the body shows the notes as Enter does"
    else
        bad "after Space the screen differs from Enter's: $(whereabouts notes_space notes_shown)"
    fi
    "${NENV[@]}" wtype -k space
    sleep 1
    shot notes_space_hidden || { summary; exit 1; }
    if same notes_hidden notes_space_hidden; then
        ok "and a second Space hides them, the panel back as it was"
    else
        bad "after a second Space the screen differs: $(whereabouts notes_space_hidden notes_hidden)"
    fi

    # A card with more notes than fit shows them in an area capped at
    # NOTES_MAX tall, the padding over it, and scrolls the rest inside it:
    # thirty more notes on every task, so whichever card has the focus has
    # them, run far past the cap at a line apiece. Hidden again, the panel
    # is as it was with them hidden.
    # The notes stay on every task for the checks after this block.
    for uuid in $(task "+$TAG" _uuids 2>/dev/null); do
        for n in $(seq 1 30); do
            task rc.verbose=nothing rc.confirmation=no "$uuid" annotate -- "long note $n" >/dev/null 2>&1
        done
    done
    settle
    shot long_noted || { summary; exit 1; }
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot long_notes_shown || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_notes_shown)
    grown=$((y1 - y0 - keyboard_h))
    if [ "$grown" -eq $((PADDING + NOTES_MAX)) ]; then
        ok "thirty-one lines of notes stop at the cap (${grown}px, the padding and ${NOTES_MAX}px)"
    else
        bad "a long-noted card grew ${grown}px, expected $((PADDING + NOTES_MAX))px —
      the notes area should stop at NOTES_MAX_PX and scroll the rest"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot long_notes_hidden || { summary; exit 1; }
    if same long_noted long_notes_hidden; then
        ok "and a second Enter hides them, the panel back as it was"
    else
        bad "after hiding the long notes the screen differs: $(whereabouts long_notes_hidden long_noted)"
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

    # Ctrl+Enter refines the focused card, the first as the panel opened. The
    # spawned refine stops at the missing herdr.
    if PATH="$NESTED_SPAWN_PATH" command -v herdr >/dev/null; then
        skip "Ctrl+Enter refining (herdr is in $NESTED_SPAWN_PATH, so a refine would really open)"
    else
        refines() { cat "$SB/notifications" 2>/dev/null | grep -c "herdr is not installed"; }
        before=$(refines)
        "${NENV[@]}" wtype -M ctrl -k Return -m ctrl
        for _ in $(seq 1 50); do [ "$(refines)" -gt "$before" ] && break; sleep 0.1; done
        if [ "$(refines)" -gt "$before" ]; then
            ok "Ctrl+Enter on an unrefined card starts a refine"
        else
            bad "Ctrl+Enter started no refine (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
        fi
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
    # one-line cards and their gaps, as three tucked cards are than one, less
    # Clear all's strip under the Waiting tab's bar and the card gap over it:
    # the button's 31px and the 8px gap.
    clear_strip_h=39
    waiting_h=$((y1 - y0))
    extra=$((keyboard_h - waiting_h - (three_h - one_h) + clear_strip_h))
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

    # ─── Clear all on the Waiting tab ────────────────────────────────────────
    # Park a second task, so the Waiting tab has two and one task stays on
    # All. Clear all shows on the Waiting tab alone, and Ctrl+Shift+Delete presses
    # it, the tabs being outside the focus chain.
    count() {  # filter…
        task rc.verbose=nothing "$@" count 2>/dev/null
    }
    task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:write the glossary" \
        modify wait:someday </dev/null >/dev/null 2>&1
    settle
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 5
    sleep 1
    shot clear_before || { summary; exit 1; }

    # The first press only arms it, as Remove's does: Confirm clear all, and
    # nothing deleted. It takes the focus off the cards, so the first card
    # loses its darker fill and its action row, and the panel is shorter.
    "${NENV[@]}" wtype -M ctrl -M shift -k Delete -m shift -m ctrl
    sleep 1
    shot clear_armed || { summary; exit 1; }
    read -r _ _ y0 y1 < <(measure clear_before)
    before_h=$((y1 - y0))
    read -r _ _ y0 y1 < <(measure clear_armed)
    armed_h=$((y1 - y0))
    if [ "$armed_h" -gt 0 ] && [ "$armed_h" -lt "$before_h" ] && [ "$(count "+$TAG" status:waiting)" = 2 ]; then
        ok "the first Ctrl+Shift+Delete arms Clear all, takes the focus off the cards and deletes nothing (${before_h}px to ${armed_h}px)"
    else
        bad "after the first Ctrl+Shift+Delete the panel is ${armed_h}px against ${before_h}px, with
      $(count "+$TAG" status:waiting) task(s) still waiting — expected shorter, the focused
      card's row gone, and 2"
    fi

    # Escape cancels: Clear all back, the focus back on the card it was on,
    # and the panel still up.
    "${NENV[@]}" wtype -k Escape
    sleep 1
    shot clear_cancelled || { summary; exit 1; }
    if same clear_before clear_cancelled; then
        ok "Escape puts Clear all back and the focus on its card, keeping the panel"
    else
        bad "after Escape on an armed Clear all the screen is not as it was before arming"
    fi

    # Armed again, Enter confirms: both waiting tasks deleted, one after the
    # other, each through task status; the task still on All and the other
    # tag's are left alone.
    "${NENV[@]}" wtype -M ctrl -M shift -k Delete -m shift -m ctrl
    sleep 0.5
    notified_before=$(grep -c '^Tasks Deleted: ' "$SB/notifications" 2>/dev/null || true)
    notified_before=${notified_before:-0}
    "${NENV[@]}" wtype -k Return
    for _ in $(seq 1 20); do
        [ "$(count "+$TAG" status:waiting)" = 0 ] && break
        sleep 0.5
    done
    settle
    deleted=$(count "+$TAG" status:deleted)
    left=$(count "+$TAG" status:pending)
    elsewhere=$(count +niritasks_e2e_elsewhere status:pending)
    if [ "$deleted" = 2 ] && [ "$left" = 1 ] && [ "$elsewhere" = 1 ]; then
        ok "Enter on Confirm clear all deletes both waiting tasks and no other"
    else
        bad "after Clear all: ${deleted} deleted, ${left} pending here, ${elsewhere} on the
      other tag — expected 2, 1 and 1"
    fi
    notified=$(grep -c '^Tasks Deleted: ' "$SB/notifications" 2>/dev/null || true)
    notified=$(( ${notified:-0} - notified_before ))
    if [ "$notified" = 2 ]; then
        ok "each went through task status deleted, notification and all"
    else
        bad "${notified} 'Deleted' notifications for two tasks cleared — Clear all did not
      run task status on each"
    fi

    # The panel keeps the keyboard, back on All with the one task left.
    shot clear_after || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure clear_after)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ] &&
        [ "$((y1 - y0))" -lt "$keyboard_h" ]; then
        ok "the panel keeps the keyboard in the middle, on All with one card (rows ${y0}-${y1})"
    else
        bad "after Clear all the panel covers columns ${x0}-${x1}, rows ${y0}-${y1} —
      reaching ${OUT_W} means it gave up the keyboard; 0-0 means it is gone"
    fi
    "${NENV[@]}" wtype -k Escape
    settle

    # Past the cap: nine more beside the one Clear all left make ten tasks on
    # All, eight cards and "+2 more". Down from the eighth card onto "+2 more"
    # shows every card at once, focused on the ninth: the panel grows by the
    # two cards it hid, where focusing "+2 more" itself would lose the eighth
    # card's buttons and grow nothing.
    for n in 1 2 3 4 5 6 7 8 9; do add "past the cap $n"; done
    settle
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    for _ in 1 2 3 4 5 6 7; do "${NENV[@]}" wtype -k Down; sleep 0.2; done
    sleep 1
    shot long_eighth || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_eighth)
    eighth_h=$((y1 - y0))
    "${NENV[@]}" wtype -k Down
    sleep 1
    shot long_more || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_more)
    more_h=$((y1 - y0))
    if [ "$eighth_h" -gt 0 ] && [ "$more_h" -gt "$eighth_h" ]; then
        ok "Down onto \"+2 more\" shows every card (${eighth_h}px to ${more_h}px)"
    else
        bad "Down from the eighth card took the panel from ${eighth_h}px to ${more_h}px —
      it should grow by the two cards \"+2 more\" hid; shorter means it focused
      \"+2 more\" instead of showing them"
    fi
    "${NENV[@]}" wtype -k Escape
    settle

    # ─── the Ideas tab ───────────────────────────────────────────────────────
    # 7 opens Ideas: no cards, one text area in the middle, taking every key
    # as typing, the s, 1 and ] that press a button or pick a tab elsewhere
    # included. It saves a second after typing stops, and again at once when
    # Escape goes back to the task list, the panel staying up; a second
    # Escape gives the keyboard back.
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot ideas_before || { summary; exit 1; }
    "${NENV[@]}" wtype 7
    # The caret blinks, and no two frames are alike, until GTK's blink
    # timeout (10s without a key) stops it.
    sleep 11
    shot ideas_tab || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure ideas_tab)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
        ok "7 opens the Ideas tab in the middle (columns ${x0}-${x1}, rows ${y0}-${y1})"
    else
        bad "on the Ideas tab the panel covers columns ${x0}-${x1}, rows ${y0}-${y1}, expected
      within ${SURFACE_LEFT}-${SURFACE_RIGHT}"
    fi
    "${NENV[@]}" wtype 'first idea'
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'second: s, 1 and ] are typing'
    sleep 2.5
    want=$'first idea\nsecond: s, 1 and ] are typing'
    if [ "$(ideas)" = "$want" ]; then
        ok "what is typed on Ideas is saved once typing stops, the panel still up"
    else
        bad "a second after typing stopped, $IDEAS holds '$(ideas)', expected '$want'"
    fi
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'third'
    "${NENV[@]}" wtype -k Escape
    settle
    want=$'first idea\nsecond: s, 1 and ] are typing\nthird'
    if [ "$(ideas)" = "$want" ]; then
        ok "Escape saves what was typed since, going back to the task list"
    else
        bad "after Escape, $IDEAS holds '$(ideas)', expected '$want'"
    fi
    shot ideas_out || { summary; exit 1; }
    read -r x0 x1 _ _ < <(measure ideas_out)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
        ok "and the panel stays up in the middle (columns ${x0}-${x1})"
    else
        bad "after one Escape on Ideas the panel drew columns ${x0}-${x1}, expected it still
      within ${SURFACE_LEFT}-${SURFACE_RIGHT}"
    fi
    # Back on the task list: the cards, as the keyboard first showed them,
    # from the tab picked before Ideas (All, where task panel opened).
    if same ideas_out ideas_before; then
        ok "Escape from Ideas is back on the tab it came from"
    else
        read -r x0 x1 y0 y1 < <(measure ideas_out ideas_before)
        bad "after Escape from Ideas the panel differs from the task list before it in
      columns ${x0}-${x1}, rows ${y0}-${y1}"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
    shot ideas_released || { summary; exit 1; }
    read -r x0 x1 _ _ < <(measure ideas_released)
    if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
        ok "a second Escape tucks the panel back to its peek (columns ${x0}-${x1})"
    else
        bad "after two Escapes from Ideas the panel drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
    fi
else
    skip "the panel's cards in the middle of the screen, their buttons, the filter tabs, Clear all, the Ideas tab, and Escape back (needs wtype)"
fi

# ─── a card's Complete and Move to workspace ─────────────────────────────────
# c completes the focused task and keeps the list up. m swaps the cards for
# the ~/Projects folders the task could move to, and Escape puts the cards
# back as they were. Enter on a folder is not pressed: the move would run in
# the nested niri's spawn, with the real HOME and ~/Projects. write_path.rs
# checks the retagging, against its sandbox.
if command -v wtype >/dev/null; then
    add "move me"
    add "complete me"
    settle
    completed() { task rc.verbose=nothing "+$TAG" status:completed count 2>/dev/null; }
    before=$(completed)
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype c
    for _ in $(seq 1 50); do [ "$(completed)" -gt "$before" ] && break; sleep 0.1; done
    if [ "$(completed)" -eq $((before + 1)) ]; then
        ok "c completes the focused task"
    else
        bad "c completed $(($(completed) - before)) tasks, expected 1"
    fi
    settle
    if find "$HOME/Projects" -mindepth 1 -maxdepth 1 -type d ! -name '.*' ! -name e2e 2>/dev/null | grep -q .; then
        shot move_before || { summary; exit 1; }
        "${NENV[@]}" wtype m
        let_cursor_rest
        shot move_list || { summary; exit 1; }
        if same move_before move_list; then
            bad "m left the panel as it was — no project list"
        else
            ok "m swaps the cards for the project list"
        fi
        "${NENV[@]}" wtype -k Escape
        sleep 1
        shot move_back || { summary; exit 1; }
        if same move_before move_back; then
            ok "Escape from the project list puts the cards back as they were"
        else
            read -r x0 x1 y0 y1 < <(measure move_back move_before)
            bad "after Escape from the project list the panel differs in columns ${x0}-${x1}, rows ${y0}-${y1}"
        fi
    else
        skip "the project list (no ~/Projects folder to move a task to)"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
else
    skip "a card's Complete and Move to workspace (needs wtype)"
fi

# ─── nothing pending shows nothing ───────────────────────────────────────────
# rc.bulk=0: completing more than two tasks at once otherwise stops to ask,
# and with no terminal to answer, completes none of them. Completed tasks
# become finished cards, which the hover/peek does not show, so the edge is
# empty; the keyboard's panel would still have the Finished tab.
task rc.verbose=nothing rc.confirmation=no rc.bulk=0 "+$TAG" done </dev/null >/dev/null 2>&1
settle
shot empty || { summary; exit 1; }
if same baseline empty; then
    ok "the panel goes away when the workspace has no tasks"
else
    read -r x0 x1 y0 y1 < <(measure empty)
    bad "something is still drawn with no tasks (columns ${x0}-${x1}, rows ${y0}-${y1})"
fi

# ─── Mod+Alt+W's project list, with no task ──────────────────────────────────
# `project open` shows the project list on a workspace with no tasks, where
# the panel had nothing to show, and Escape hides it again. Nothing is
# picked: the open would run in the nested niri's spawn, on the real
# ~/Projects. Its own XDG_CACHE_HOME keeps its gh refresh off the real cache.
if command -v wtype >/dev/null && [ -d "$HOME/Projects" ]; then
    "${NENV[@]}" XDG_CACHE_HOME="$SB/cache" "$NIRITASKS" project open >/dev/null 2>&1
    let_cursor_rest
    shot open_list || { summary; exit 1; }
    if same baseline open_list; then
        bad "project open drew nothing on a workspace with no tasks"
    else
        ok "project open shows the project list with no task on the workspace"
    fi
    "${NENV[@]}" wtype -k Escape
    sleep 1
    shot open_closed || { summary; exit 1; }
    if same baseline open_closed; then
        ok "Escape closes the project list, leaving nothing drawn"
    else
        read -r x0 x1 y0 y1 < <(measure open_closed)
        bad "after Escape from the project list something is still drawn (columns ${x0}-${x1}, rows ${y0}-${y1})"
    fi
else
    skip "Mod+Alt+W's project list (needs wtype and ~/Projects)"
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

# ─── the Ideas tab survives a restart ────────────────────────────────────────
# The daemon above is a new one: Ideas opens on what the last one saved, the
# cursor at its end, so a line typed now lands after it.
if command -v wtype >/dev/null; then
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 7
    sleep 1
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'after the restart'
    "${NENV[@]}" wtype -k Escape
    "${NENV[@]}" wtype -k Escape
    settle
    want=$'first idea\nsecond: s, 1 and ] are typing\nthird\nafter the restart'
    if [ "$(ideas)" = "$want" ]; then
        ok "a restarted daemon's Ideas tab opens on the saved ideas and adds to them"
    else
        bad "after a restart, $IDEAS holds '$(ideas)', expected '$want' — the
      first three lines missing means the new daemon did not load them"
    fi
else
    skip "the Ideas tab after a daemon restart (needs wtype)"
fi

# ─── the real daemon, untouched ──────────────────────────────────────────────
nested_service_untouched

summary
[ "$fail" -eq 0 ]
