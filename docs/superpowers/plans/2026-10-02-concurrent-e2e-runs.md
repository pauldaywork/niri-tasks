# Concurrent e2e Runs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Several agents, each in its own worktree, can run `bash tests/all.sh` (or any single e2e script) at the same time, and every run passes or fails on its own merits.

**Architecture:** The sandbox (`$SB`, `$RT`, `TASKDATA`, the notify stub) is already per run; what clashes is the one parent niri every run shares. `nested_start` finds its window by the nested niri's pid instead of diffing `app_id == "niri"` windows, and parks it on the last workspace of its monitor, re-picking until it is alone there. `e2e-tag.sh` names its borrowed workspace `niritasks-e2e-scratch-$$`, never picks another run's scratch workspace, and unnames only its own. Every step that picks a workspace of the parent by index and then acts on it — parking, tearing a nested niri down, naming and unnaming the scratch workspace — runs under one `flock` per parent niri, from a new `tests/lib/parent-lock.sh`, because niri only takes workspace references by index or name and an index can shift between reading it and using it.

**Tech Stack:** bash, python3 (JSON from `niri msg -j`), `flock` from util-linux (`/usr/bin/flock`), niri 26.04 IPC.

**Spec:** Taskwarrior task `c6dd6a21-cb46-4696-b1f5-1f72c374a972`. Read it with `task rc.json.array=on c6dd6a21-cb46-4696-b1f5-1f72c374a972 export`. Its description and notes are the spec.

## Global Constraints

- `nested_start` finds its window by the nested niri's pid: `niri msg -j windows` gives each window's `pid`, compared with `$NESTED`. No before/after comparison of `app_id=niri` windows.
- Each nested window is parked alone on its own spare workspace. If another window gets there first, it re-picks the new last workspace and tries again. It never shares a workspace.
- `e2e-tag.sh` names its borrowed scratch workspace uniquely per run (with `$$`), skips workspaces another run has already named, and unnames only its own.
- Update the header comments in `tests/lib/nested-niri.sh` and `tests/all.sh` to say concurrent runs are supported.
- Done when: three `bash tests/all.sh` runs started together from separate worktrees all pass, twice in a row, with no leftover nested windows or scratch workspace names, and a single run still passes.
- Out of scope: speeding up the suites, and running the suites in parallel inside one `all.sh`.
- House style: comments say *why*, in the plain voice of the surrounding scripts. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Findings and decisions made while planning (flag any you disagree with)

1. **A lock as well as the retry.** niri 26.04 takes a workspace reference only as an index or a name (`set-workspace-name --workspace`, `move-window-to-workspace`; checked with `--help`), and indexes shift: when a run's nested niri exits, niri drops the workspace it leaves empty and every later index moves down by one. Without a lock, `e2e-tag.sh` can name a workspace by an index that now points at another run's scratch workspace, and `set-workspace-name` silently renames it — a clash the retry cannot see. So the read-pick-act-check sequences take `flock` on `$XDG_RUNTIME_DIR/niritasks-e2e-<basename of $NIRI_SOCKET>.lock`: one lock per parent niri, shared by every worktree. The retry stays, for windows a person moves while a run parks. `flock` is util-linux, already on the machine; nothing custom is built.
2. **Teardown also takes the lock.** `cleanup` kills the nested niri and waits for its window to be gone while holding the lock, so the workspace it frees is dropped before another run reads indexes.
3. **The lock file is never deleted.** Deleting a lock file other processes may be about to open is the classic flock race; an empty file in the runtime dir costs nothing and goes at logout.
4. **Parking checks "alone on the monitor it opened on", not "at index N".** An index that moved because a person closed a workspace is no reason to fail; sharing a workspace is. Landing on another monitor still dies at once, as now (`move-window-to-workspace` resolves the index on the focused output).
5. **`e2e-tag.sh` ignores every `niritasks-e2e-scratch*` name when it picks `OTHER`.** Otherwise one run's "other workspace" can be a second run's scratch, which vanishes under it.
6. **`e2e-tag.sh` traps fixed:** `trap cleanup EXIT INT TERM` runs `cleanup` on Ctrl-C and then carries on with the script. It becomes `trap cleanup EXIT` plus `trap 'exit 1' INT TERM`, as `nested-niri.sh` does, so an interrupted run still unnames its workspace exactly once and stops.
7. **Unnaming an unknown name is harmless.** `niri msg action unset-workspace-name zzz-no-such-ws-name` exits 0 and leaves the focused workspace's name alone (checked), so `cleanup` can unname `$SCRATCH` without checking it still exists.
8. **The panel and box scripts need no change.** Every file they write is under `$SB` or `$RT`; their tasks go to the sandboxed `TASKDATA`; the `e2e` workspace and every `niri msg action` they run are inside their own nested niri; the only shared path in the binary that is not per run, the GitHub repo cache (`src/github.rs`), is reached only from `project open`, which neither runs. `cargo test`'s temp dirs carry the pid (`tests/write_path.rs`, `tests/link_pane.rs`), and each worktree has its own `target/`.
9. **No committed test harness for concurrency.** The failing test is two e2e scripts started together, run from the shell in the steps below; a script to launch N runs would be a suite of its own, which the spec puts out of scope.

---

### Task 1: The parent lock, pid lookup and park retry in `nested-niri.sh`

**Files:**
- Create: `tests/lib/parent-lock.sh`
- Modify: `tests/lib/nested-niri.sh` — header comment (lines 1–20), `cleanup()` (about lines 54–66), `nested_niri_windows()` (about lines 87–93), `nested_start()`'s window lookup and parking (about lines 190–240)

**Interfaces:**
- Produces (`tests/lib/parent-lock.sh`, sourced by both `nested-niri.sh` and, in Task 2, `e2e-tag.sh`):
  - `PARENT_LOCK` — the lock file's path.
  - `parent_lock` — takes the lock, waiting up to 60s. Returns non-zero if it could not. Returns 0 at once if this shell already holds it.
  - `parent_unlock` — releases it. Safe to call when not held.
- Produces (in `nested-niri.sh`): `nested_window` — prints the parent-niri window id whose `pid` is `$NESTED`, or nothing.

- [ ] **Step 1: Reproduce the clash (the failing test)**

Two nested-niri scripts started together each see two new `app_id == "niri"` windows, so neither can tell which is its own.

```bash
cargo build --release
B=$PWD/target/release/niritasks
( NIRITASKS=$B bash tests/e2e-panel.sh > /tmp/conc-panel.log 2>&1; echo "panel rc=$?" ) &
( NIRITASKS=$B bash tests/e2e-box.sh   > /tmp/conc-box.log   2>&1; echo "box rc=$?" ) &
wait
grep -h "never appeared\|was not parked\|passed:" /tmp/conc-panel.log /tmp/conc-box.log
```

Expected: at least one `rc=1`, with `the nested niri's window never appeared on this niri` in its log. (If both start more than ~5s apart they can both pass; start them again.) Afterwards check nothing was left behind: `niri msg -j windows | python3 -c 'import json,sys; print([w["id"] for w in json.load(sys.stdin) if w["app_id"]=="niri"])'` prints `[]`.

- [ ] **Step 2: Create `tests/lib/parent-lock.sh`**

```bash
# One lock per parent niri, shared by every test run on it from any worktree:
# tests/lib/nested-niri.sh and tests/e2e-tag.sh source it.
#
# niri takes a workspace reference only as an index or a name, and an index
# moves: when one run's nested niri exits, niri drops the workspace it leaves
# empty and every later index shifts down. A run that reads the workspaces,
# picks one by index and then acts on it can act on the wrong one — park its
# window beside another, or rename another run's scratch workspace. So each
# read-pick-act-check runs under this lock, and so does each teardown that
# frees a workspace. Hold it for seconds, never for a whole run.
#
# The file is never deleted: removing a lock file another process may be
# about to open is how two processes end up holding "the" lock at once.

PARENT_LOCK="${XDG_RUNTIME_DIR:-/tmp}/niritasks-e2e-$(basename "${NIRI_SOCKET:-niri}").lock"
PARENT_LOCK_FD=""

# Already held is held: cleanup takes the lock too, and a run interrupted
# while holding it would otherwise wait out its own lock on a second fd.
parent_lock() {
    [ -z "$PARENT_LOCK_FD" ] || return 0
    command -v flock >/dev/null || { echo "flock is required (util-linux)" >&2; return 1; }
    exec {PARENT_LOCK_FD}>>"$PARENT_LOCK" || return 1
    flock -w 60 "$PARENT_LOCK_FD"
}

parent_unlock() {
    [ -n "$PARENT_LOCK_FD" ] || return 0
    flock -u "$PARENT_LOCK_FD"
    exec {PARENT_LOCK_FD}>&-
    PARENT_LOCK_FD=""
}
```

- [ ] **Step 3: Source it and require `flock` in `nested-niri.sh`**

Directly after the existing `command -v python3 …` line near the top, add:

```bash
command -v flock >/dev/null || { echo "flock is required (util-linux)" >&2; exit 1; }
```

And directly after the `[ -n "${WAYLAND_DISPLAY:-}" ] || …` line:

```bash
. "$(dirname "${BASH_SOURCE[0]}")/parent-lock.sh"
```

- [ ] **Step 4: Replace `nested_niri_windows` with `nested_window`**

Replace the whole function and its comment:

```bash
# The parent's windows that are already nested niris, so the new one can be
# told apart from them.
nested_niri_windows() {
    ...
}
```

with:

```bash
# This run's nested niri's window on the parent, found by the nested niri's
# pid. Not by which niri windows are new: another run starting at the same
# moment opens one too, and then there is no telling them apart.
nested_window() {
    niri msg -j windows | python3 -c '
import json, sys
print(next((w["id"] for w in json.load(sys.stdin) if w.get("pid") == int(sys.argv[1])), ""))' "$NESTED"
}
```

`$NESTED` is the window's own pid: `env … niri &` makes `$!` the `env` process, and `env` execs `niri` in place.

- [ ] **Step 5: Use it in `nested_start`**

Delete these two lines from `nested_start`:

```bash
    local before spare opened_on landed why size
    before=$(nested_niri_windows) || die "cannot list this niri's windows"
```

and put this in their place:

```bash
    local spare opened_on landed why size
```

Then replace the window-finding loop:

```bash
    for _ in $(seq 1 25); do
        WIN=$(python3 -c '
import sys
new = set(sys.argv[2].split()) - set(sys.argv[1].split())
print(new.pop() if len(new) == 1 else "")' "$before" "$(nested_niri_windows)")
        [ -n "$WIN" ] && break
        sleep 0.2
    done
```

with:

```bash
    for _ in $(seq 1 25); do
        WIN=$(nested_window 2>/dev/null)
        [ -n "$WIN" ] && break
        sleep 0.2
    done
```

The `die "the nested niri's window never appeared on this niri"` after it stays.

- [ ] **Step 6: Park under the lock, retrying until alone**

Replace everything from `spare=$(python3 -c '` (the first spare-workspace lookup) through the line `die "the nested niri's window was not parked alone on workspace $spare of $opened_on: it may have gone to another monitor"` with:

```bash
    # The monitor it opened on, which is where it is parked.
    opened_on=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next(w for w in get("windows") if w["id"] == int(sys.argv[1]))
print(next(s["output"] for s in get("workspaces") if s["id"] == win["workspace_id"]))' "$WIN") ||
        die "cannot find which monitor the nested niri's window opened on"

    # Alone on a workspace of its own, every time: never beside a window of
    # yours or another run's. Under the lock (tests/lib/parent-lock.sh) no
    # other run moves a window or frees a workspace between the pick and the
    # check; the retry is for windows you move meanwhile. Each try re-picks
    # the last workspace, which niri has made anew if the last one filled.
    parent_lock || die "could not take the lock on this niri's workspaces ($PARENT_LOCK)"
    for _ in 1 2 3 4 5; do
        spare=$(python3 -c '
import json, subprocess, sys
spaces = json.loads(subprocess.run(["niri", "msg", "-j", "workspaces"],
                                   capture_output=True, text=True, check=True).stdout)
print(max(s["idx"] for s in spaces if s["output"] == sys.argv[1]))' "$opened_on") ||
            { parent_unlock; die "cannot find a spare workspace for the nested niri"; }
        niri msg action move-window-to-workspace --window-id "$WIN" --focus false "$spare" >/dev/null ||
            { parent_unlock; die "niri would not park the nested niri's window on workspace $spare (move-window-to-workspace)"; }
        # The index is resolved against the focused output, which may not be
        # the one the window opened on.
        landed=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
windows = get("windows")
win = next(w for w in windows if w["id"] == int(sys.argv[1]))
space = next(s for s in get("workspaces") if s["id"] == win["workspace_id"])
if space["output"] != sys.argv[2]:
    print("elsewhere")
elif any(w["workspace_id"] == space["id"] and w["id"] != win["id"] for w in windows):
    print("shared")
else:
    print("alone")' "$WIN" "$opened_on" 2>/dev/null) ||
            { parent_unlock; die "cannot ask this niri where the nested window was parked"; }
        [ "$landed" = shared ] || break
        sleep 0.2
    done
    parent_unlock
    case "$landed" in
        alone) ;;
        elsewhere) die "the nested niri's window was not parked on $opened_on: it may have gone to another monitor" ;;
        *) die "the nested niri's window could not be parked alone on a workspace of $opened_on: each one tried had a window on it" ;;
    esac
```

The `move-window-to-floating` and `set-window-width/height` calls above it stay as they are (they act on the window by id and touch no workspace).

- [ ] **Step 7: Tear down under the lock**

In `cleanup()`, replace:

```bash
    # Its window closes with it, and niri drops the workspace it leaves empty.
    # Anything still running inside it loses its display and exits too.
    [ -n "$NESTED" ] && kill "$NESTED" 2>/dev/null && wait "$NESTED" 2>/dev/null
```

with:

```bash
    # Its window closes with it, and niri drops the workspace it leaves empty.
    # Anything still running inside it loses its display and exits too. That
    # shifts the index of every later workspace, so it happens under the lock,
    # which is held until the window is gone. Without the lock it still goes.
    if [ -n "$NESTED" ]; then
        parent_lock 2>/dev/null
        kill "$NESTED" 2>/dev/null && wait "$NESTED" 2>/dev/null
        if [ -n "$WIN" ]; then
            for _ in $(seq 1 25); do
                [ -z "$(nested_window 2>/dev/null)" ] && break
                sleep 0.2
            done
        fi
        parent_unlock
    fi
```

`nested_window` reads `$NESTED`, which still holds the dead pid here, so it finds the window only while niri still shows it. Pids are not reused within a second, so this cannot match another window.

- [ ] **Step 8: Update the header comment**

In the comment block at the top of `nested-niri.sh`, after the paragraph ending `Then the run fails, saying so, rather than trust what follows.`, add this paragraph:

```bash
#
# Several runs can share your niri at once, from one checkout or from many
# worktrees. Each finds its own window by its nested niri's pid, parks it
# alone on a workspace of its own, and moves windows on your niri only while
# holding a lock (tests/lib/parent-lock.sh), so two runs never pick the same
# workspace at the same moment.
```

- [ ] **Step 9: A single run still passes**

Run: `NIRITASKS=$PWD/target/release/niritasks bash tests/e2e-panel.sh 2>&1 | tail -3`
Expected: `failed: 0`, and the line `PASS  the nested niri is parked unfocused on a spare workspace, 1600x1000`.

Run: `NIRITASKS=$PWD/target/release/niritasks bash tests/e2e-box.sh 2>&1 | tail -3`
Expected: `failed: 0`.

- [ ] **Step 10: The clash from Step 1 is gone**

Run the Step 1 commands again, three times.
Expected: `panel rc=0` and `box rc=0` every time, each log ending `failed: 0`. Then `niri msg -j windows | python3 -c 'import json,sys; print([w["id"] for w in json.load(sys.stdin) if w["app_id"]=="niri"])'` prints `[]`.

Also start two of the same script together, which shares even the script:

```bash
B=$PWD/target/release/niritasks
for i in 1 2; do ( NIRITASKS=$B bash tests/e2e-box.sh > /tmp/conc-box$i.log 2>&1; echo "box$i rc=$?" ) & done; wait
```

Expected: `box1 rc=0`, `box2 rc=0`.

- [ ] **Step 11: Commit**

```bash
git add tests/lib/parent-lock.sh tests/lib/nested-niri.sh
git commit -m "Find each nested niri's window by its pid and park it alone under a lock shared by every run

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A scratch workspace of its own in `e2e-tag.sh`

**Files:**
- Modify: `tests/e2e-tag.sh` — the prerequisites at the top (about lines 29–31), the `FOCUSED`/`OTHER` lookup (about lines 41–48), `cleanup` and the traps (about lines 56–64), the scratch block (about lines 66–91), and the header comment's last paragraph (lines 26–28)

**Interfaces:**
- Consumes: `tests/lib/parent-lock.sh` from Task 1 — `parent_lock`, `parent_unlock`, `PARENT_LOCK`.
- Produces: nothing other tasks use. The scratch workspace is now named `niritasks-e2e-scratch-<pid of the run>`.

- [ ] **Step 1: Reproduce the clash (the failing test)**

```bash
B=$PWD/target/release/niritasks
for i in 1 2 3; do ( NIRITASKS_E2E_SCRATCH=1 NIRITASKS=$B bash tests/e2e-tag.sh > /tmp/conc-tag$i.log 2>&1; echo "tag$i rc=$?" ) & done; wait
grep -h "FAIL\|named workspace" /tmp/conc-tag*.log
niri msg -j workspaces | python3 -c 'import json,sys; print([w["name"] for w in json.load(sys.stdin) if (w["name"] or "").startswith("niritasks-e2e-scratch")])'
```

Expected: all three name the same workspace `niritasks-e2e-scratch`, and the first to finish unnames it under the others, so at least one later `--session` case reports `FAIL … does not match any named workspace` or the like. This race depends on timing; if all three pass, run it again — the bug is plain from the code either way (one fixed name, unnamed by whoever finishes first). The last command must print `[]` afterwards; if it does not, unname what it lists with `niri msg action unset-workspace-name <name>`.

- [ ] **Step 2: Source the lock and require `flock`**

After the line `[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }`, add:

```bash
command -v flock >/dev/null || { echo "flock is required (util-linux)" >&2; exit 1; }
. "$(dirname "${BASH_SOURCE[0]}")/lib/parent-lock.sh"

# Every run's borrowed workspace starts with this. The pid makes each run's
# name its own, so no run renames, uses or unnames another's.
SCRATCH_PREFIX="niritasks-e2e-scratch"
```

- [ ] **Step 3: Never take another run's scratch workspace as `OTHER`**

Replace the `FOCUSED`/`OTHER` lookup:

```bash
read -r FOCUSED OTHER < <(niri msg -j workspaces | python3 -c "
import json,sys
ws=[w for w in json.load(sys.stdin) if w.get('name')]
focused=next((w['name'] for w in ws if w['is_focused']), '')
other=next((w['name'] for w in ws if not w['is_focused']), '')
print(focused, other)
")
```

with:

```bash
# Another run's scratch workspace is never the other one: that run unnames it
# when it finishes, maybe halfway through this one.
read -r FOCUSED OTHER < <(niri msg -j workspaces | python3 -c "
import json,sys
ws=[w for w in json.load(sys.stdin) if w.get('name')]
focused=next((w['name'] for w in ws if w['is_focused']), '')
other=next((w['name'] for w in ws
            if not w['is_focused'] and not w['name'].startswith(sys.argv[1])), '')
print(focused, other)
" "$SCRATCH_PREFIX")
```

- [ ] **Step 4: Unname under the lock, and fix the traps**

Replace:

```bash
cleanup() {
    [ -n "$SCRATCH" ] && niri msg action unset-workspace-name "$SCRATCH" >/dev/null 2>&1
    [ -n "$FAKE_HOME" ] && rm -rf "$FAKE_HOME"
}
# INT and TERM as well as EXIT: interrupting a run must not leave a workspace
# named after this test sitting in the switcher.
trap cleanup EXIT INT TERM
```

with:

```bash
# Only this run's own name is ever taken off. Unnamed, the empty workspace is
# one niri drops, which moves later indexes, so it happens under the lock.
cleanup() {
    if [ -n "$SCRATCH" ]; then
        parent_lock 2>/dev/null
        niri msg action unset-workspace-name "$SCRATCH" >/dev/null 2>&1
        parent_unlock
    fi
    [ -n "$FAKE_HOME" ] && rm -rf "$FAKE_HOME"
}
# INT and TERM end the run, and the run's end unnames the workspace: an
# interrupted run must not leave a workspace named after this test sitting in
# the switcher. A trap that ran cleanup on INT itself would then carry on with
# the script.
trap cleanup EXIT
trap 'exit 1' INT TERM
```

- [ ] **Step 5: Name a workspace of its own, under the lock**

Replace the whole scratch block, from `if [ -z "$OTHER" ] || [ -n "${NIRITASKS_E2E_SCRATCH:-}" ]; then` to its closing `fi` (the one before `if [ -z "$FOCUSED" ]; then`), with:

```bash
if [ -z "$OTHER" ] || [ -n "${NIRITASKS_E2E_SCRATCH:-}" ]; then
    # Under the lock no other run names a workspace or frees one between the
    # pick and the naming, so the index still points where it did. The check
    # afterwards is for whatever you did meanwhile: if the name did not land
    # on an empty workspace of this monitor, it comes off and there is no
    # scratch workspace.
    if parent_lock; then
        scratch_idx=$(niri msg -j workspaces | python3 -c "
import json,sys
ws = json.load(sys.stdin)
focused = next((w for w in ws if w['is_focused']), None)
output = focused['output'] if focused else None
spare = [w for w in ws
         if not w.get('name')
         and w.get('active_window_id') is None
         and (output is None or w['output'] == output)]
print(spare[0]['idx'] if spare else '')
")
        if [ -n "$scratch_idx" ]; then
            SCRATCH="$SCRATCH_PREFIX-$$"
            if niri msg action set-workspace-name --workspace "$scratch_idx" "$SCRATCH" >/dev/null 2>&1 &&
               niri msg -j workspaces | python3 -c "
import json,sys
ws = json.load(sys.stdin)
focused = next((w for w in ws if w['is_focused']), None)
mine = next((w for w in ws if w.get('name') == sys.argv[1]), None)
sys.exit(0 if mine and mine.get('active_window_id') is None
            and (focused is None or mine['output'] == focused['output']) else 1)
" "$SCRATCH"; then
                OTHER="$SCRATCH"
                echo "no second named workspace to hand — named workspace $scratch_idx" \
                     "'$SCRATCH' for the run, and will unname it afterwards"
            else
                niri msg action unset-workspace-name "$SCRATCH" >/dev/null 2>&1
                SCRATCH=""
            fi
        fi
        parent_unlock
    else
        echo "could not take the lock on this niri's workspaces ($PARENT_LOCK)" >&2
    fi
fi
```

Unchanged: the `if [ -z "$OTHER" ]` check right after still exits with `need a second workspace to test against, and no empty one was free to borrow` when this found nothing.

- [ ] **Step 6: Update the header comment**

Replace the header's last paragraph:

```bash
# It needs two named workspaces to show the contrast, and will borrow niri's
# trailing empty workspace as the second one when there is only ever a single
# project open — naming it without focusing it, and unnaming it on the way out.
```

with:

```bash
# It needs two named workspaces to show the contrast, and will borrow niri's
# trailing empty workspace as the second one when there is only ever a single
# project open — naming it without focusing it, and unnaming it on the way out.
# Several runs can do this at once: each names its own workspace
# niritasks-e2e-scratch-<pid>, under the lock in tests/lib/parent-lock.sh, and
# never borrows or unnames another run's.
```

- [ ] **Step 7: A single run still passes, both ways**

Run: `NIRITASKS=$PWD/target/release/niritasks bash tests/e2e-tag.sh 2>&1 | tail -2`
Expected: `failed: 0`, with the same `passed:` count as before this task.

Run: `NIRITASKS_E2E_SCRATCH=1 NIRITASKS=$PWD/target/release/niritasks bash tests/e2e-tag.sh 2>&1 | grep -E "named workspace|passed:"`
Expected: `… named workspace N 'niritasks-e2e-scratch-<pid>' for the run …` and `failed: 0`.

- [ ] **Step 8: The clash from Step 1 is gone**

Run the Step 1 commands again, three times.
Expected: `tag1 rc=0`, `tag2 rc=0`, `tag3 rc=0` every time, three different `niritasks-e2e-scratch-<pid>` names across the logs, and the last command prints `[]`.

Then interrupt one: `NIRITASKS_E2E_SCRATCH=1 NIRITASKS=$PWD/target/release/niritasks timeout -s INT 0.5 bash tests/e2e-tag.sh; niri msg -j workspaces | python3 -c 'import json,sys; print([w["name"] for w in json.load(sys.stdin) if (w["name"] or "").startswith("niritasks-e2e-scratch")])'`
Expected: `[]`. (If 0.5s ends it before it names anything, that also prints `[]`; try 1 or 2 seconds so the interrupt lands mid-run.)

- [ ] **Step 9: Commit**

```bash
git add tests/e2e-tag.sh
git commit -m "Give each e2e-tag run a scratch workspace of its own, named under the shared lock

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Say so in `all.sh`, and prove three runs at once

**Files:**
- Modify: `tests/all.sh` — header comment (lines 25–30) and the `why_tag`/`why_panel`/`why_box` prerequisite checks (about lines 47–70)

**Interfaces:**
- Consumes: Task 1 and Task 2's scripts, unchanged.
- Produces: nothing.

- [ ] **Step 1: Check the panel and box scripts again for anything shared**

Run: `grep -nE '/tmp|\$HOME|XDG_|\.cache|\.local|niri msg action' tests/e2e-panel.sh tests/e2e-box.sh | grep -v 'NENV\|nested '`
Expected: no lines (every `niri msg action` they run goes through `nested`/`"${NENV[@]}"`, and every path is under `$SB`). If a line shows up, it is a shared path or a call on the parent niri: report it rather than leave it.

- [ ] **Step 2: Add `flock` to the prerequisites**

`all.sh` mirrors each script's own checks. In `why_tag`, `why_panel` and `why_box`, directly after the `command -v niri …` line of each, add:

```bash
    command -v flock >/dev/null || { echo "no flock (util-linux)"; return; }
```

- [ ] **Step 3: Update the header comment**

Replace:

```bash
# They run cheapest first: cargo test touches nothing, e2e-tag.sh at most
# names a spare workspace, and the panel and box tests each run in a nested
# niri of their own (tests/lib/nested-niri.sh), parked on the last workspace
# of your monitor. Neither presses a key or takes a screenshot on your
# desktop, so you can keep working while they run — just keep off that
# workspace.
```

with:

```bash
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
```

- [ ] **Step 4: A single run still passes**

Run: `NIRITASKS=$PWD/target/release/niritasks bash tests/all.sh 2>&1 | tail -6`
Expected: `4 passed   0 failed   0 skipped`.

- [ ] **Step 5: Commit the docs before the big run**

```bash
git add tests/all.sh
git commit -m "Say in all.sh that several runs can share one niri, and check for flock

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: Three runs at once from separate worktrees, twice**

Three throwaway worktrees at this commit, each with its own `target/` (so each run's `cargo test` builds; the first round is slow), all using the one release binary:

```bash
B=$PWD/target/release/niritasks
D=$(mktemp -d /tmp/niritasks-conc.XXXXXX)
for i in 1 2 3; do git worktree add --detach "$D/wt$i" HEAD; done
for round in 1 2; do
    for i in 1 2 3; do
        ( cd "$D/wt$i" && NIRITASKS=$B bash tests/all.sh > "$D/round$round-wt$i.log" 2>&1; echo "round $round wt$i rc=$?" ) &
    done
    wait
    tail -n 1 "$D"/round$round-wt*.log
done
```

Expected: six `rc=0` lines, and every log ending `4 passed   0 failed   0 skipped`. If one fails, read its log (`grep -n FAIL "$D"/round*-wt*.log`) and fix the cause before going on; a failure here is the point of this step, not noise.

Run it with a 30-minute timeout (`timeout` 1800000 on the Bash tool) or in the background; three `cargo test` builds and six e2e runs take a while.

- [ ] **Step 7: Nothing left behind**

```bash
niri msg -j windows | python3 -c 'import json,sys; print("nested windows:", [w["id"] for w in json.load(sys.stdin) if w["app_id"]=="niri"])'
niri msg -j workspaces | python3 -c 'import json,sys; print("scratch names:", [w["name"] for w in json.load(sys.stdin) if (w["name"] or "").startswith("niritasks-e2e-scratch")])'
ls /tmp | grep -c '^nte2e\.' || true
```

Expected: `nested windows: []`, `scratch names: []`, and `0` leftover `nte2e.*` runtime dirs (`cleanup` removes each run's `$RT`).

- [ ] **Step 8: Remove the throwaway worktrees**

```bash
for i in 1 2 3; do git worktree remove --force "$D/wt$i"; done
rm -rf "$D"
git worktree prune
```

Expected: `git worktree list` shows no `niritasks-conc` entries. Nothing to commit from this step.
