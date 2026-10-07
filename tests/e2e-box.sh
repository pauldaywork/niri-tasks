#!/usr/bin/env bash
# End-to-end test of the task box: keypress -> box -> submit -> task written.
#
#   bash tests/e2e-box.sh
#
# This is not a cargo test, and cannot be: it needs a running niri, a Wayland
# display and wtype to press the keys. `cargo test` covers the pure logic; this
# covers the part that only exists once a compositor is involved.
#
# It earns its place. Two bugs reached daily use that no unit test could have
# caught, both in that gap:
#
#   * The box opened as a window with the daemon's app_id rather than its own,
#     so anything matching on app-id — including the checks that were supposed
#     to prove it worked — looked straight past it.
#   * Ctrl+Enter did nothing. The key mapping was correct and tested; the event
#     controller was in the wrong propagation phase, so the focused text view
#     consumed Return before the window saw it. Only a real keypress through a
#     real compositor exercises that.
#
# It covers the one box in all three modes: adding (with due:friday parsed and
# notes typed as rows), editing an existing task's notes in place — changed,
# deleted with Backspace, dates kept — and `task note`, which opens the same
# box with the cursor in a new row. Driven the only way that proves anything
# here: real keypresses. The ×, "+ Add note" and scrolling take a pointer or
# eyes, which wtype has neither of — README.md lists them as the manual check.
# And Add & refine: Ctrl+Shift+Enter, and `task add --refine`'s Ctrl+Enter, add
# the task and start a refine on it, which is let to fail on a missing herdr so
# nothing opens in yours.
#
# It types only into a niri of its own (tests/lib/nested-niri.sh), parked
# unfocused on the last workspace of your monitor. wtype's keys reach the niri
# its WAYLAND_DISPLAY names and nothing else, so you can keep working while it
# runs — but keep off that workspace: going there focuses the nested window,
# and the run fails rather than trust keys that might have been yours. Your
# own niri-tasks daemon keeps running: the fallback half runs with no daemon
# inside the nested niri, which is where the box looks for one. The task
# database is a sandbox's.
set -uo pipefail

command -v wtype >/dev/null || {
    echo "wtype is required: sudo apt install wtype" >&2
    exit 1
}

# A refine spawned by Add & refine runs in the nested niri with this PATH: the
# notify-send stub, the system tools, and no herdr — so it stops at "herdr is
# not installed." and the stub logs that, which proves a refine was started on
# the new task without opening anything in your real herdr.
. "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"
NESTED_SPAWN_PATH="$SB/bin:/usr/bin:/bin"

box_id() {
    nested niri msg -j windows 2>/dev/null | python3 -c "
import json,sys
for w in json.load(sys.stdin):
    if (w.get('app_id') or '')=='dev.niri-tasks.box':
        print(w['id']); break
"
}
focused_is_box() {
    nested niri msg -j focused-window 2>/dev/null | python3 -c "
import json,sys
try: w=json.load(sys.stdin)
except Exception: print('no'); raise SystemExit
print('yes' if (w or {}).get('app_id')=='dev.niri-tasks.box' else 'no')
"
}
box_title() {
    nested niri msg -j focused-window 2>/dev/null | python3 -c "
import json,sys
try: w=json.load(sys.stdin) or {}
except Exception: w={}
print(w.get('title') or '')
"
}
# open_box add          — the add box
# open_box note <uuid>  — the note box for a task
open_box() {
    guard
    [ "${#NENV[@]}" -gt 0 ] || die "open_box before nested_start"
    "${NENV[@]}" "$NIRITASKS" task "$@" >/dev/null 2>&1 &
    for _ in $(seq 1 40); do
        [ -n "$(box_id)" ] && { sleep 0.6; return 0; }
        sleep 0.1
    done
    return 1
}
# The uuid of the pending task whose description contains $1.
uuid_of() {
    task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
      | python3 -c "
import json,sys
print(next((t['uuid'] for t in json.load(sys.stdin) if sys.argv[1] in t['description']), ''))
" "$1"
}
# A task's notes, one per line.
notes_of() {
    task rc.verbose=nothing rc.json.array=on "$1" export 2>/dev/null \
      | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print('\n'.join(a['description'] for a in (ts[0].get('annotations') or []))) if ts else None
"
}
# A task's notes with their stamps, one "<entry> <text>" per line.
stamped_notes_of() {
    task rc.verbose=nothing rc.json.array=on "$1" export 2>/dev/null \
      | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print('\n'.join(a['entry']+' '+a['description'] for a in (ts[0].get('annotations') or []))) if ts else None
"
}
# How many changes taskwarrior has logged — an unchanged save must add none.
undo_count() { wc -l < "$TASKDATA/undo.data" 2>/dev/null || echo 0; }
field_of() {
    task rc.verbose=nothing rc.json.array=on "$1" export 2>/dev/null \
      | python3 -c "import json,sys; ts=json.load(sys.stdin); print(ts[0].get(sys.argv[1],'') if ts else '')" "$2"
}
pending() {
    task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
      | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0
}
# How many refines Add & refine has started — each stops at the missing herdr.
refine_tries() { cat "$SB/notifications" 2>/dev/null | grep -c "herdr is not installed"; }
# Wait up to 5s for refine_tries to reach $1.
wait_refine_tries() {
    for _ in $(seq 1 50); do
        [ "$(refine_tries)" -ge "$1" ] && return 0
        sleep 0.1
    done
    return 1
}
# Make a freshly opened box read wtype's keys right. wtype hands each call its
# own keymap, and a box process that has just started can read the first
# virtual key it gets with the keyboard's earlier keymap instead — where
# wtype's first keycode is Escape. A Return sent first to a new box arrived as
# Escape: the box closed, and the steps after it typed into nothing. Typed text
# is read right, so a character typed and deleted at the end of the
# description (where every box opens its cursor) settles it and changes
# nothing.
settle_keys() {
    nested wtype "x"; nested wtype -k BackSpace
    sleep 0.2
}
close_any_box() {
    for id in $(box_id); do nested niri msg action close-window --id "$id" >/dev/null 2>&1; done
    sleep 0.5
}

run_suite() {
    local label="$1"
    echo
    echo "=== $label ==="

    if open_box add; then
        ok "box opens"
        [ "$(focused_is_box)" = yes ] && ok "box takes keyboard focus" \
            || bad "box did not take focus — keys would go to the wrong window"
    else
        bad "box never opened"; return
    fi

    # Ctrl+Enter submits, and the text survives the trip.
    local before after desc
    before=$(pending)
    nested wtype "written by the end to end test"
    sleep 0.4
    nested wtype -M ctrl -k Return -m ctrl
    sleep 1.5
    after=$(pending)
    if [ "$after" -gt "$before" ]; then
        desc=$(task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
            | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print(next((t['description'] for t in ts if 'end to end' in t['description']), ''))")
        [ "$desc" = "written by the end to end test" ] && ok "Ctrl+Enter wrote the task intact" \
            || bad "text differs: \"$desc\""
        [ -z "$(box_id)" ] && ok "box closed after submit" || bad "box still open after submit"
    else
        bad "Ctrl+Enter wrote nothing ($before -> $after)"
    fi

    # A bare Return in the description does not submit: it moves to the notes.
    before=$(pending)
    if open_box add; then
        nested wtype "first line"; nested wtype -k Return; nested wtype "a note"
        sleep 0.4
        [ "$(pending)" -eq "$before" ] && ok "bare Enter did not submit" \
            || bad "bare Enter submitted"
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        desc=$(task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
            | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print(next((t['description'] for t in ts if 'first line' in t['description']), ''))")
        [ "$desc" = "first line" ] && ok "Enter left the description on its line" \
            || bad "expected 'first line', got \"$desc\""
    else
        bad "box did not reopen"
    fi

    # Escape cancels; nothing typed is kept.
    before=$(pending)
    if open_box add; then
        nested wtype "this should never be saved"; sleep 0.3
        nested wtype -k Escape; sleep 1.2
        [ "$(pending)" -eq "$before" ] && ok "Escape wrote nothing" || bad "Escape wrote a task"
        [ -z "$(box_id)" ] && ok "box closed on Escape" || bad "box still open after Escape"
    else
        bad "box did not reopen"
    fi

    # Saving with no description writes nothing, and keeps the box open rather
    # than throwing away whatever notes were typed — only Escape discards. It
    # has to be closed here, or the next open_box finds it still up and the
    # keys meant for the next box land in this one.
    before=$(pending)
    if open_box add; then
        settle_keys
        nested wtype -M ctrl -k Return -m ctrl; sleep 1.2
        [ "$(pending)" -eq "$before" ] && ok "empty submit wrote nothing" \
            || bad "empty submit wrote a task"
        [ -n "$(box_id)" ] && ok "and left the box open" \
            || bad "empty submit closed the box"
        nested wtype -k Escape; sleep 1.2
        [ -z "$(box_id)" ] && ok "Escape then closed it" \
            || bad "box still open after Escape"
    else
        bad "box did not reopen"
    fi

    # Add still word-splits, so taskwarrior attributes parse.
    local dmark="due-$RANDOM" uuid
    if open_box add; then
        nested wtype "$dmark due:friday"
        sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        uuid=$(uuid_of "$dmark")
        if [ -n "$uuid" ]; then
            [ -n "$(field_of "$uuid" due)" ] && ok "Add filed due:friday as a due date" \
                || bad "Add left due:friday without a due date"
            [ "$(field_of "$uuid" description)" = "$dmark" ] && ok "and took it out of the description" \
                || bad "description was \"$(field_of "$uuid" description)\""
        else
            bad "Add with due:friday wrote no task"
        fi
    else
        bad "box did not reopen"
    fi

    # Notes while adding: Enter in the description moves to the first note,
    # Enter in a note adds a row below. An empty row is not a note.
    local marker="notes-$RANDOM" notes
    if open_box add; then
        nested wtype "$marker"
        nested wtype -k Return
        nested wtype "first note"
        nested wtype -k Return
        nested wtype -k Return                      # an empty row, left empty
        nested wtype "second note"
        sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        uuid=$(uuid_of "$marker")
        if [ -n "$uuid" ]; then
            notes=$(notes_of "$uuid")
            [ "$notes" = "$(printf 'first note\nsecond note')" ] \
                && ok "Enter moved from description to notes; one annotation per row, empty row dropped" \
                || bad "expected first/second note, got: $(printf '%s' "$notes" | tr '\n' '|')"
        else
            bad "add box with notes wrote no task"
        fi
    else
        bad "box did not reopen"
    fi

    # Edit an existing task's notes in place. Three notes, seeded from the CLI.
    local emark="edit-$RANDOM" stamps a_stamp c_stamp
    nested "$NIRITASKS" task add "$emark" >/dev/null 2>&1
    uuid=$(uuid_of "$emark")
    for n in "note A" "note B" "note C"; do nested "$NIRITASKS" task note "$uuid" "$n" >/dev/null 2>&1; done
    stamps=$(stamped_notes_of "$uuid")
    a_stamp=$(printf '%s\n' "$stamps" | sed -n 1p | cut -d' ' -f1)
    c_stamp=$(printf '%s\n' "$stamps" | sed -n 3p | cut -d' ' -f1)

    if [ -n "$uuid" ] && open_box edit "$uuid"; then
        [ "$(box_title)" = "Edit Task" ] && ok "edit opens the one box" \
            || bad "edit box title was \"$(box_title)\""
        settle_keys
        nested wtype -k Return                      # description -> note A, cursor at end
        nested wtype " edited"
        nested wtype -k Tab; nested wtype -k Tab           # note A's ×, then note B
        nested wtype -M ctrl a -m ctrl              # select all of note B
        nested wtype -k BackSpace                   # ...and clear it
        nested wtype -k BackSpace                   # an empty row: delete it, move up
        sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        [ "$(notes_of "$uuid")" = "$(printf 'note A edited\nnote C')" ] \
            && ok "a note edited in place and another deleted, in one save" \
            || bad "notes after edit: $(notes_of "$uuid" | tr '\n' '|')"
        [ "$(stamped_notes_of "$uuid" | cut -d' ' -f1 | tr '\n' ' ')" = "$a_stamp $c_stamp " ] \
            && ok "edited and untouched notes kept their dates" \
            || bad "stamps changed: $(stamped_notes_of "$uuid" | tr '\n' '|')"
    else
        bad "edit box did not open"
    fi

    # Saving without changing anything writes nothing at all.
    local undo_before
    undo_before=$(undo_count)
    if [ -n "$uuid" ] && open_box edit "$uuid"; then
        settle_keys                          # or an Escape here would pass too
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        [ "$(undo_count)" -eq "$undo_before" ] && ok "an unchanged save wrote nothing" \
            || bad "an unchanged save wrote to the task database"
    else
        bad "edit box did not reopen"
    fi

    # Note opens the same box with the cursor in a new empty row at the end.
    if [ -n "$uuid" ] && open_box note "$uuid"; then
        [ "$(box_title)" = "Edit Task" ] && ok "note opens the same box" \
            || bad "note box title was \"$(box_title)\""
        nested wtype "note D"
        nested wtype -k Return
        nested wtype "note E"
        sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        [ "$(notes_of "$uuid")" = "$(printf 'note A edited\nnote C\nnote D\nnote E')" ] \
            && ok "note added rows after the existing ones" \
            || bad "notes after note box: $(notes_of "$uuid" | tr '\n' '|')"
        [ "$(stamped_notes_of "$uuid" | sed -n 2p | cut -d' ' -f1)" = "$c_stamp" ] \
            && ok "adding notes left the old ones' dates alone" \
            || bad "note C's date changed"
    else
        bad "note box did not open"
    fi

    # Escape discards everything, without asking.
    if [ -n "$uuid" ] && open_box note "$uuid"; then
        nested wtype "this note should never be saved"; sleep 0.3
        nested wtype -k Escape; sleep 1.2
        [ "$(notes_of "$uuid" | grep -c .)" -eq 4 ] \
            && ok "Escape in the box wrote nothing" \
            || bad "Escape changed the notes"
        [ -z "$(box_id)" ] && ok "box closed on Escape" || bad "box still open after Escape"
    else
        bad "note box did not reopen"
    fi

    close_any_box
    guard
}

run_refine_suite() {
    local label="$1" before mark uuid
    echo
    echo "=== $label: Add & refine ==="

    if PATH="$NESTED_SPAWN_PATH" command -v herdr >/dev/null; then
        echo "  skip: herdr is in $NESTED_SPAWN_PATH, so a refine here would really open"
        return
    fi

    # Ctrl+Shift+Enter in the plain add box presses Add & refine.
    mark="refine-$RANDOM"; before=$(refine_tries)
    if open_box add; then
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -M shift -k Return -m shift -m ctrl
        sleep 1.5
        uuid=$(uuid_of "$mark")
        [ -n "$uuid" ] && ok "Ctrl+Shift+Enter added the task" \
            || bad "Ctrl+Shift+Enter wrote no task"
        wait_refine_tries $((before + 1)) && ok "and started refining it" \
            || bad "no refine was started (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
        [ -n "$(uuid_of "$mark")" ] && ok "a failed refine left the task added" \
            || bad "the task went missing after refine failed"
    else
        bad "add box never opened"
    fi

    # `task add --refine` makes Add & refine the default: Ctrl+Enter presses it.
    mark="refine-$RANDOM"; before=$(refine_tries)
    if open_box add --refine; then
        [ "$(box_title)" = "Add Task" ] && ok "--refine opens the add box" \
            || bad "--refine box title was \"$(box_title)\""
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        [ -n "$(uuid_of "$mark")" ] && ok "Ctrl+Enter in a --refine box added the task" \
            || bad "Ctrl+Enter in a --refine box wrote no task"
        wait_refine_tries $((before + 1)) && ok "and started refining it" \
            || bad "Ctrl+Enter in a --refine box started no refine (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
    else
        bad "--refine box never opened"
    fi

    # Plain Ctrl+Enter in the plain box adds and does not refine.
    mark="plain-$RANDOM"; before=$(refine_tries)
    if open_box add; then
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 3
        [ -n "$(uuid_of "$mark")" ] && ok "plain Ctrl+Enter added the task" \
            || bad "plain Ctrl+Enter wrote no task"
        [ "$(refine_tries)" -eq "$before" ] && ok "and did not refine it" \
            || bad "plain Ctrl+Enter started a refine"
    else
        bad "add box never opened"
    fi

    close_any_box
    guard
}

nested_start

# Both paths matter: the daemon serves the box when it is running, and the CLI
# builds its own when it is not. The fallback is the reason this tool does not
# depend on a daemon, so it is tested rather than assumed. The box looks for
# a daemon under the nested niri's runtime dir, so the second half needs only
# none running there — yours keeps running throughout.
nested_daemon_start "$SB/daemon.err"
run_suite "served by the daemon"
run_refine_suite "served by the daemon"
nested_daemon_stop
sleep 1

run_suite "fallback, no daemon running"
run_refine_suite "fallback, no daemon running"

nested_service_untouched
summary
[ "$fail" -eq 0 ]
