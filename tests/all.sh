#!/usr/bin/env bash
# Every test suite this machine can run, in one command.
#
#   bash tests/all.sh
#
# There are four suites and they do not have the same prerequisites: one needs
# only cargo and taskwarrior, one needs a tmux server, one needs Pillow and a
# compositor that will take screenshots, one needs wtype and a Wayland display.
# A machine missing any of them is the normal case — an SSH session has no
# Wayland display, a fresh checkout has no wtype — and that is the whole reason
# this file exists. Running the four by hand means reading four error messages
# and deciding, each time, whether "wtype is required" meant the code is broken
# or the machine is.
#
# So each suite's prerequisites are checked here first, and one that cannot run
# is reported as SKIP with the reason. A skip is not a failure: this exits
# non-zero only when a suite that actually ran said no. That is what makes it
# safe to put in a hook or a loop on a machine where only half of it can run.
#
# The prerequisites below mirror the checks at the top of each script. The
# script's own check is still the authority — if one of these drifts out of
# date, the suite runs and fails on its own terms, which is loud rather than
# silent, and the right way round for a mistake like that to land.
#
# They run in order of how much they take over the machine: cargo test touches
# nothing, e2e-tag.sh makes tmux sessions, e2e-overlay.sh takes screenshots and
# with them the clipboard, and e2e-box.sh types into whatever has focus. So the
# cheap suites have already reported by the time you have to leave the keyboard
# alone, and a failure in the fast half does not cost you a minute of not
# touching the machine to find out about.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

# The e2e scripts default to the installed `niritasks`, not the one you just built, and
# a green run against the old binary is worse than no run at all. Exported so
# the value reported here is the value they use.
NIRITASKS="${NIRITASKS:-niritasks}"
export NIRITASKS

passed=(); failed=(); skipped=()

# Why this suite cannot run, or nothing at all if it can.
why_cargo() {
    command -v cargo >/dev/null || { echo "no cargo on \$PATH"; return; }
    command -v task  >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
}
why_tag() {
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v tmux >/dev/null || { echo "no tmux"; return; }
    command -v niri >/dev/null || { echo "no niri"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
}
why_overlay() {
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v niri >/dev/null || { echo "no niri"; return; }
    command -v task >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
    python3 -c "import PIL" 2>/dev/null || { echo "no python3 Pillow (sudo apt install python3-pil)"; return; }
}
why_box() {
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v wtype >/dev/null || { echo "no wtype (sudo apt install wtype)"; return; }
    command -v niri  >/dev/null || { echo "no niri"; return; }
    [ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display"; return; }
}

suite() {  # name, reason it cannot run (empty if it can), command...
    local name="$1" why="$2"; shift 2
    if [ -n "$why" ]; then
        printf '\n\033[1m── %s ──\033[0m  SKIP: %s\n' "$name" "$why"
        skipped+=("$name — $why")
        return
    fi
    printf '\n\033[1m── %s ──\033[0m\n' "$name"
    local start=$SECONDS
    if "$@"; then
        passed+=("$name ($((SECONDS - start))s)")
    else
        failed+=("$name")
    fi
}

echo "repo      $ROOT"
echo "niritasks $(command -v "$NIRITASKS" 2>/dev/null || echo "not found ($NIRITASKS)")"

suite "cargo test"           "$(why_cargo)"   cargo test
suite "tests/e2e-tag.sh"     "$(why_tag)"     bash tests/e2e-tag.sh

if [ -z "$(why_overlay)" ]; then
    echo
    echo "  ! the next two suites take the machine over: screenshots use the"
    echo "    clipboard, and the box test types into whatever has focus."
    echo "    Leave the keyboard alone until they finish."
fi
suite "tests/e2e-overlay.sh" "$(why_overlay)" bash tests/e2e-overlay.sh
suite "tests/e2e-box.sh"     "$(why_box)"     bash tests/e2e-box.sh

echo
echo "──────────────────────────────────────────────────────────────"
for s in ${passed+"${passed[@]}"};  do echo "  PASS  $s"; done
for s in ${skipped+"${skipped[@]}"}; do echo "  SKIP  $s"; done
for s in ${failed+"${failed[@]}"};  do echo "  FAIL  $s"; done
echo
echo "${#passed[@]} passed   ${#failed[@]} failed   ${#skipped[@]} skipped"

# A skip is not a failure. Only a suite that ran and said no fails this.
[ "${#failed[@]}" -eq 0 ]
