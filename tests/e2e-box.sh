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
# Runs against a sandboxed TASKDATA, so the real task database is untouched.
set -uo pipefail

command -v wtype >/dev/null || {
    echo "wtype is required: sudo apt install wtype" >&2
    exit 1
}
[ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display" >&2; exit 1; }
command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }

WT="${WT:-wt}"
SB="$(mktemp -d)"
trap 'rm -rf "$SB"' EXIT
mkdir -p "$SB/data"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

pass=0; fail=0
ok()  { echo "  PASS  $*"; pass=$((pass+1)); }
bad() { echo "  FAIL  $*"; fail=$((fail+1)); }

box_id() {
    niri msg -j windows 2>/dev/null | python3 -c "
import json,sys
for w in json.load(sys.stdin):
    if (w.get('app_id') or '')=='dev.niri-tasks.box':
        print(w['id']); break
"
}
focused_is_box() {
    niri msg -j focused-window 2>/dev/null | python3 -c "
import json,sys
try: w=json.load(sys.stdin)
except Exception: print('no'); raise SystemExit
print('yes' if (w or {}).get('app_id')=='dev.niri-tasks.box' else 'no')
"
}
open_box() {
    "$WT" task add >/dev/null 2>&1 &
    for _ in $(seq 1 40); do
        [ -n "$(box_id)" ] && { sleep 0.6; return 0; }
        sleep 0.1
    done
    return 1
}
pending() {
    task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
      | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0
}
close_any_box() {
    for id in $(box_id); do niri msg action close-window --id "$id" >/dev/null 2>&1; done
    sleep 0.5
}

run_suite() {
    local label="$1"
    echo
    echo "=== $label ==="

    if open_box; then
        ok "box opens"
        [ "$(focused_is_box)" = yes ] && ok "box takes keyboard focus" \
            || bad "box did not take focus — keys would go to the wrong window"
    else
        bad "box never opened"; return
    fi

    # Ctrl+Enter submits, and the text survives the trip.
    local before after desc
    before=$(pending)
    wtype "written by the end to end test"
    sleep 0.4
    wtype -M ctrl -k Return -m ctrl
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

    # A bare Return has to reach the text view, or the box cannot be multi-line,
    # which is its only reason to exist. It collapses to a space on the way out
    # because taskwarrior descriptions are one line.
    before=$(pending)
    if open_box; then
        wtype "first line"; wtype -k Return; wtype "second line"
        sleep 0.4
        [ "$(pending)" -eq "$before" ] && ok "bare Enter did not submit" \
            || bad "bare Enter submitted"
        wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        desc=$(task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
            | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print(next((t['description'] for t in ts if 'first line' in t['description']), ''))")
        [ "$desc" = "first line second line" ] && ok "newline collapsed on submit" \
            || bad "expected 'first line second line', got \"$desc\""
    else
        bad "box did not reopen"
    fi

    # Escape cancels; nothing typed is kept.
    before=$(pending)
    if open_box; then
        wtype "this should never be saved"; sleep 0.3
        wtype -k Escape; sleep 1.2
        [ "$(pending)" -eq "$before" ] && ok "Escape wrote nothing" || bad "Escape wrote a task"
        [ -z "$(box_id)" ] && ok "box closed on Escape" || bad "box still open after Escape"
    else
        bad "box did not reopen"
    fi

    # Submitting an empty box is a no-op rather than an empty task.
    before=$(pending)
    if open_box; then
        wtype -M ctrl -k Return -m ctrl; sleep 1.2
        [ "$(pending)" -eq "$before" ] && ok "empty submit wrote nothing" \
            || bad "empty submit wrote a task"
    else
        bad "box did not reopen"
    fi
    close_any_box
}

# Start from a clean slate. A previous run that failed part-way can leave a box
# on screen, and the checks below key off "is there a box window" — so a stale
# one makes the next run fail for reasons that have nothing to do with the code.
# That happened once and cost a confusing debugging detour.
close_any_box
pkill -f "$WT task add" 2>/dev/null

# Both paths matter: the daemon serves the box when it is running, and the CLI
# builds its own when it is not. The fallback is the reason this tool does not
# depend on a daemon, so it is tested rather than assumed.
WAS_ACTIVE=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
systemctl --user stop niri-tasks.service 2>/dev/null
sleep 1

"$WT" daemon >"$SB/daemon.err" 2>&1 &
DAEMON=$!
sleep 3
run_suite "served by the daemon"
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
sleep 1

run_suite "fallback, no daemon running"

[ "$WAS_ACTIVE" = active ] && systemctl --user start niri-tasks.service 2>/dev/null

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
