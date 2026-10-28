#!/usr/bin/env bash
# Every test suite this machine can run, in one command.
#
#   bash tests/all.sh
#
# There are six suites and they do not have the same prerequisites:
#   - cargo test needs only cargo and taskwarrior;
#   - the herdr plugin's tests need python3 and git, and fake herdr;
#   - the refine mod's tests need claude (`claude plugin test`);
#   - e2e-tag.sh needs niri;
#   - e2e-panel.sh needs Pillow, and niri to nest a niri in;
#   - e2e-box.sh needs wtype and python3, and niri to nest a niri in.
# A machine missing any of them is the normal case — an SSH session has no Wayland display, a fresh
# checkout has no wtype — and that is the whole reason this file exists.
# Running the six by hand means reading six error messages and deciding, each
# time, whether "wtype is required" meant the code is broken or the machine is.
#
# NIRITASKS_HEADLESS=1 skips the three e2e suites, whatever the machine has:
# they nest a niri and borrow a workspace, which is wrong for a run nobody is
# watching, such as the one Explain branch makes before a branch report. cargo,
# the plugin tests and the mod tests are what is left. The mod suite is here
# because the mod is TypeScript that cargo never sees: without it a broken
# hook or panel would pass this file and show up only in a live session.
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
# They run cheapest first: cargo test touches nothing, e2e-tag.sh at most
# names a spare workspace, and the panel and box tests each run in a nested
# niri of their own (tests/lib/nested-niri.sh), parked on the last workspace
# of your monitor. Neither presses a key or takes a screenshot on your
# desktop, so you can keep working while they run — just keep off that
# workspace.
#
# Several of these can run at once on one niri — agents in separate
# worktrees, say. Each run's sandbox is its own, each nested niri parks alone
# on a workspace of its own, each e2e-tag.sh run names its own scratch
# workspace, and the moments they rearrange your workspaces take turns
# through one lock (tests/lib/parent-lock.sh). The suites inside one run
# still go one after another.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

# The e2e scripts default to the installed `niritasks`, not the one you just built, and
# a green run against the old binary is worse than no run at all. Exported so
# the value reported here is the value they use.
NIRITASKS="${NIRITASKS:-niritasks}"
export NIRITASKS

passed=(); failed=(); skipped=()

headless_skip() {
    [ -n "${NIRITASKS_HEADLESS:-}" ] || return 1
    echo "headless run (NIRITASKS_HEADLESS)"
}

# Why this suite cannot run, or nothing at all if it can.
why_cargo() {
    command -v cargo >/dev/null || { echo "no cargo on \$PATH"; return; }
    command -v task  >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
}
why_tag() {
    headless_skip && return
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v niri >/dev/null || { echo "no niri"; return; }
    command -v flock >/dev/null || { echo "no flock (util-linux)"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
}
why_panel() {
    headless_skip && return
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v niri >/dev/null || { echo "no niri"; return; }
    command -v flock >/dev/null || { echo "no flock (util-linux)"; return; }
    command -v task >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
    command -v python3 >/dev/null || { echo "no python3"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
    [ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display"; return; }
    python3 -c "import PIL" 2>/dev/null || { echo "no python3 Pillow (sudo apt install python3-pil)"; return; }
}
why_box() {
    headless_skip && return
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v wtype >/dev/null || { echo "no wtype (sudo apt install wtype)"; return; }
    command -v niri  >/dev/null || { echo "no niri"; return; }
    command -v flock >/dev/null || { echo "no flock (util-linux)"; return; }
    command -v task  >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
    command -v python3 >/dev/null || { echo "no python3"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
    [ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display"; return; }
}

why_plugin() {
    command -v python3 >/dev/null || { echo "no python3"; return; }
    command -v git >/dev/null || { echo "no git"; return; }
}

why_mod() {
    command -v claude >/dev/null || { echo "no claude on \$PATH"; return; }
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

suite "cargo test"           "$(why_cargo)"   cargo test --no-fail-fast
suite "tests/e2e-tag.sh"     "$(why_tag)"     bash tests/e2e-tag.sh
suite "herdr/task-docs tests" "$(why_plugin)" \
    python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs
suite "refine mod tests"     "$(why_mod)"     claude plugin test claude/refine-mod

if [ -z "$(why_panel)" ] || [ -z "$(why_box)" ]; then
    echo
    echo "  ! the panel and box tests park a nested niri on a spare workspace"
    echo "    at the end of this monitor: keep off those workspaces until they finish."
fi
suite "tests/e2e-panel.sh" "$(why_panel)" bash tests/e2e-panel.sh
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
