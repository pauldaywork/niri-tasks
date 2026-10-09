# Try a Worktree Build in a Nested niri — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `bash try.sh` in any checkout or worktree builds that checkout and opens it, by hand, in a focused nested niri with a copy of the real tasks — without touching the installed binary, the live daemon, the real task database, herdr, or e2e runs and agents in other worktrees.

**Architecture:** `tests/lib/nested-niri.sh` already builds the sandbox the e2e tests run in (its own nested niri, runtime dir, daemon socket, `TASKDATA`, notify-send stub, herdr-free `PATH`, parent lock). `nested_start` gains a `--focused` option that skips parking and the focus watcher, plus two optional variables (`NESTED_WORKSPACE`, `NESTED_EXTRA_KDL`); the e2e tests call it unchanged. A new `try.sh` at the repo root builds, seeds the sandbox's `TASKDATA` with an SQLite online backup of the real database, starts the nested niri focused on a workspace named after the project with `niri/niri-tasks.kdl` included and Mod on Right Alt, starts a daemon from `target/`, and turns the terminal into a "try shell" with `reload`; exiting the shell tears everything down.

**Tech Stack:** bash, niri 26.04 (`niri msg`, KDL config, `niri validate`), Taskwarrior 3.5 (`taskchampion.sqlite3`), python3 `sqlite3`, cargo.

**Spec:** Taskwarrior task `8994bf6d-c9b1-4e78-978e-12f45ec41faf` — read it with `task rc.json.array=on 8994bf6d-c9b1-4e78-978e-12f45ec41faf export`; its description and annotations are the spec.

## Global Constraints

- Never touch `~/.cargo/bin`, `systemctl --user` (beyond the read-only `systemctl --user show` nested-niri.sh already does) or install.sh's links. Never run `install.sh`, `cargo install` or `systemctl --user restart niri-tasks`.
- Reuse nested-niri.sh through an option on `nested_start` (`--focused`) that skips parking and the focus watcher; the e2e tests keep using it unchanged, so there is still one sandbox.
- The nested window opens focused on the current workspace and niri sizes it like any other window.
- Seed the sandbox `TASKDATA` with an SQLite online backup (Python's `sqlite3`) of the real `taskchampion.sqlite3`; writes go only to the copy.
- Name the nested niri's workspace after the worktree's project (niri-tasks gives tag `niri_tasks`).
- Include `niri/niri-tasks.kdl` as-is in the nested niri, its spawns finding the worktree binary, with Mod as Right Alt (xkb `lv3:ralt_switch`, `mod-key-nested "ISO_Level3_Shift"`); if niri will not take that, fall back to plain Alt.
- Notifications go to the e2e notify-send stub and the try shell shows its log.
- `reload` in the try shell runs `cargo build` and restarts only the nested daemon, keeping the window and the sandbox copy.
- Debug build by default; `--release` builds and runs release.
- The script is `try.sh` at the repo root, beside `install.sh`.
- Out of scope: exercising herdr actions (Start working, Go to session) against real herdr; installing a build for daily use (that stays `install.sh` on main).
- Commits: Conventional Commits, subject ≤72 chars (aim ~50), imperative, lowercase; body wrapped at 72 saying what and why; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comments match the repo's style: full sentences that say why, in the plain voice nested-niri.sh uses.
- Work only in this worktree: `/home/paul/.worktrees/niri-tasks/task-feat-try-a-worktree-build-in-a-nested-8994bf6d`. Scratch files go in the session scratchpad (`$SCRATCH` below), never committed.
- **Running `try.sh` or the probe opens a focused window on the user's current workspace.** Before each such run, tell the user in one line that a window is about to open for a few seconds.

Throughout, `SCRATCH` means the session scratchpad directory (for the controller: `/tmp/claude-1000/-home-paul--worktrees-niri-tasks-task-feat-try-a-worktree-build-in-a-nested-8994bf6d/72a1ad60-9931-49f3-915f-bdd5cce3e8ac/scratchpad`; a subagent may use any directory outside the repo).

Already verified while planning: `niri validate` accepts an `input { keyboard { xkb { options "lv3:ralt_switch"; }; }; mod-key-nested "ISO_Level3_Shift"; }` block, a `workspace "niri-tasks"` line and an `include` of the absolute path to `niri/niri-tasks.kdl`. Whether Right Alt actually works as Mod in the nested window can only be checked by hand (Task 3). The real `~/.taskrc` holds only `data.location`, so the sandbox's minimal taskrc loses nothing.

---

### Task 1: `nested_start --focused`, a workspace name and extra config

**Files:**
- Modify: `tests/lib/nested-niri.sh` (header comment; the `# NESTED_SPAWN_PATH (optional)` line near the top; `nested_start`, from `nested_start() {` through the `echo "nested niri: window $WIN, sockets in $RT"` line)
- Test: `$SCRATCH/probe-focused.sh` (scratch, not committed)

**Interfaces:**
- Produces, used by Task 2:
  - `nested_start [--focused]` — with `--focused`, returns right after the nested window is found (`$WIN` set, `$NENV` set, `$WATCH` empty), leaving the window where niri opened it; the generated config keeps niri's animations and key repeat. Any other argument dies with `nested_start: unknown option <arg>`.
  - `NESTED_WORKSPACE` (optional, default `e2e`) — the nested niri's one named workspace.
  - `NESTED_EXTRA_KDL` (optional, default empty) — KDL text appended to the nested niri's config.
- Unchanged: with no argument and neither variable set, the generated `$SB/niri.kdl` is today's plus one empty line, and everything after the window is found (float, size, park, focus check, size check, `watch_focus`) runs as before.

- [ ] **Step 1: Build the binary the probe and the e2e runs use**

Run: `cargo build --release 2>&1 | tail -n 3`
Expected: `Finished` line, no errors.

- [ ] **Step 2: Write the failing probe**

Create `$SCRATCH/probe-focused.sh`:

```bash
#!/usr/bin/env bash
# Probe for nested_start --focused. Run from the worktree root:
#   NIRITASKS=$PWD/target/release/niritasks bash "$SCRATCH/probe-focused.sh"
set -uo pipefail
. tests/lib/nested-niri.sh

NESTED_WORKSPACE=probe-ws
NESTED_EXTRA_KDL='input { mod-key-nested "Alt"; }'
nested_start --focused

names=$(nested niri msg -j workspaces | python3 -c '
import json, sys
print(" ".join(s["name"] for s in json.load(sys.stdin) if s["name"]))')
[ "$names" = probe-ws ] && ok "the one named workspace is probe-ws" || bad "named workspaces: '$names'"

grep -q 'mod-key-nested "Alt"' "$SB/niri.kdl" && ok "NESTED_EXTRA_KDL is in the config" || bad "NESTED_EXTRA_KDL is missing"
grep -q 'animations' "$SB/niri.kdl" && bad "a focused run turned animations off" || ok "animations left on"
grep -q 'repeat-rate' "$SB/niri.kdl" && bad "a focused run turned key repeat off" || ok "key repeat left on"

state=$(niri msg -j windows | python3 -c '
import json, sys
w = next(w for w in json.load(sys.stdin) if w["id"] == int(sys.argv[1]))
print(w["is_floating"], w["is_focused"])' "$WIN")
[ "$state" = "False True" ] && ok "the window is tiled and focused" || bad "is_floating is_focused: $state"

[ -z "$WATCH" ] && ok "no focus watcher" || bad "a focus watcher is running ($WATCH)"

summary
[ "$fail" -eq 0 ]
```

- [ ] **Step 3: Run the probe to see it fail**

Tell the user a window will open briefly. Run: `NIRITASKS=$PWD/target/release/niritasks bash "$SCRATCH/probe-focused.sh"`
Expected: FAIL lines — named workspaces `'e2e'`, `NESTED_EXTRA_KDL is missing`, `a focused run turned animations off`, `is_floating is_focused: True False`, a focus watcher running. (Today `nested_start` ignores its argument and parks the window.)

- [ ] **Step 4: Document the new option and variables**

In `tests/lib/nested-niri.sh`, after the header paragraph ending `...The nested niri's own spawns get NESTED_SPAWN_PATH when a test sets one.`, add:

```bash
#
# try.sh starts the same sandbox by hand with `nested_start --focused`: the
# window is not parked but left where niri opens it, on your workspace with
# the focus, sized like any other window, and nothing watches it. Its config
# keeps niri's animations and key repeat, which only a test wants off.
```

Replace the line `# NESTED_SPAWN_PATH (optional): the PATH the nested niri's own spawns run with.` with:

```bash
# NESTED_SPAWN_PATH (optional): the PATH the nested niri's own spawns run with.
# NESTED_WORKSPACE (optional): the nested niri's one named workspace, which is
# the tag tasks are filed under; e2e when unset.
# NESTED_EXTRA_KDL (optional): more of the nested niri's config, appended as is.
```

- [ ] **Step 5: Parse `--focused` and write the config from it**

At the top of `nested_start() {`, before the existing comment block that begins `# Animations off, so niri itself never draws a frame in between.`, insert:

```bash
    local focused=""
    while [ $# -gt 0 ]; do
        case "$1" in
            --focused) focused=1 ;;
            *) die "nested_start: unknown option $1" ;;
        esac
        shift
    done
```

Just before `cat > "$SB/niri.kdl" <<EOF`, insert:

```bash
    # By hand (--focused), niri keeps its animations and key repeat.
    local test_kdl='animations { off; }
input { keyboard { repeat-rate 0; }; }'
    [ -n "$focused" ] && test_kdl=""
```

Replace the heredoc body so it reads exactly:

```bash
    cat > "$SB/niri.kdl" <<EOF
hotkey-overlay { skip-at-startup; }
$test_kdl
xwayland-satellite { off; }
output "winit" { scale 1; }
layout { background-color "#406080"; }
workspace "${NESTED_WORKSPACE:-e2e}"
$spawn_env
${NESTED_EXTRA_KDL:-}
spawn-sh-at-startup "env > $SB/nested.env.tmp && mv $SB/nested.env.tmp $SB/nested.env"
EOF
```

This adds one line (`${NESTED_EXTRA_KDL:-}`, empty for the e2e tests — an empty line, which niri ignores) and keeps every other line where it was.

- [ ] **Step 6: Return early when focused**

Immediately after the line `echo "nested niri: window $WIN, sockets in $RT"`, insert:

```bash

    # By hand, the window stays where niri opened it: on your workspace, with
    # the focus, which is the point. Nothing below applies.
    [ -n "$focused" ] && return 0
```

- [ ] **Step 7: Check syntax, then run the probe to see it pass**

Run: `bash -n tests/lib/nested-niri.sh && echo syntax ok`
Expected: `syntax ok`

Tell the user a window will open briefly. Run: `NIRITASKS=$PWD/target/release/niritasks bash "$SCRATCH/probe-focused.sh"`
Expected: every line `PASS`, then `passed: 7   failed: 0   skipped: 0`, exit 0, and the window closes when the probe exits.

- [ ] **Step 8: Prove the e2e tests are unchanged**

Run: `NIRITASKS=$PWD/target/release/niritasks bash tests/all.sh 2>&1 | tail -n 12`
Expected: `cargo test`, `tests/e2e-tag.sh`, `tests/e2e-panel.sh` and `tests/e2e-box.sh` all `PASS` (or `SKIP` with a machine reason only), `0 failed`. Takes several minutes. A failure here is a regression in Step 5 or 6: diff `git diff tests/lib/nested-niri.sh` against the instructions before anything else.

- [ ] **Step 9: Commit**

```bash
git add tests/lib/nested-niri.sh
git commit -m "$(cat <<'EOF'
test: let nested_start open the nested niri focused

try.sh will start the e2e sandbox by hand. nested_start --focused
leaves the window where niri opens it and keeps niri's animations and
key repeat; NESTED_WORKSPACE names its workspace and NESTED_EXTRA_KDL
adds config. The e2e tests pass neither, so their nested niri is
unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: `try.sh` and the README

**Files:**
- Create: `try.sh` (repo root, beside `install.sh`)
- Modify: `README.md` — the last paragraph of the `> [!IMPORTANT]` callout that begins `> And to try a change by hand rather than under test,` (around line 437), plus a new section after the callout
- Test: `$SCRATCH/try-smoke.sh` (scratch, not committed)

**Interfaces:**
- Consumes (Task 1): `nested_start --focused`, `NESTED_WORKSPACE`, `NESTED_EXTRA_KDL`; and from nested-niri.sh as it already is: `$SB`, `$NENV`, `nested`, `die`, `cleanup`, `$DAEMON`, `NESTED_SPAWN_PATH`, `NIRITASKS`, the `$SB/bin/notify-send` stub writing to `$SB/notifications`.
- Produces (user-facing): `bash try.sh [--release]`; in the try shell, `reload`; files `$SB/daemon.pid`, `$SB/daemon.log`, `$SB/try.bashrc`.

Design notes the implementer needs:
- **The worktree binary is found through `$SB/bin/niritasks`**, a symlink to `target/<profile>/niritasks`. `$SB/bin` is first on both the nested `PATH` (`NENV`) and `NESTED_SPAWN_PATH`, so the try shell, the daemon and niri-tasks.kdl's `spawn "niritasks" …` all reach it, and `cargo build` replacing the file is picked up through the link.
- **The daemon is owned by the try shell's `daemon_restart`**, not by nested-niri.sh's `nested_daemon_start`: `reload` runs inside the child shell, which cannot set the parent's `$DAEMON`. Its pid goes in `$SB/daemon.pid`; try.sh's EXIT trap reads it into `$DAEMON` before calling nested-niri.sh's `cleanup`, which kills it.
- **The try shell's rc is not `~/.bashrc`**: that could put herdr's directory back on `PATH`.
- **`cargo` is called by absolute path**, found before the sandbox, since the nested `PATH` drops every directory holding a herdr.
- **`XDG_DATA_HOME` is the sandbox's** (as `tests/e2e-panel.sh` does), so the Ideas tab saves nowhere real. Not in the spec; it follows "writes go only to the copy".

- [ ] **Step 1: Record the untouched state and write the failing smoke test**

Create `$SCRATCH/try-smoke.sh`:

```bash
#!/usr/bin/env bash
# Smoke test for try.sh, driving the try shell through its stdin. Run from
# the worktree root: bash "$SCRATCH/try-smoke.sh" [--release]
set -uo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
fails=0
check() { if eval "$2"; then echo "  PASS  $1"; else echo "  FAIL  $1"; fails=$((fails+1)); fi; }

svc_before=$(systemctl --user show -p MainPID --value niri-tasks.service)
bin_before=$(sha256sum ~/.cargo/bin/niritasks)
db_before=$(task rc.verbose=nothing export | sha256sum)
real_count=$(task rc.verbose=nothing +niri_tasks count)

printf '%s\n' \
    'echo "tag=$(niritasks tag)"' \
    'echo "bin=$(readlink -f "$(command -v niritasks)")"' \
    'echo "count=$(task rc.verbose=nothing +niri_tasks count)"' \
    'task rc.verbose=nothing add "test: try smoke" >/dev/null && echo added' \
    'notify-send try-smoke' \
    'sleep 1' \
    'reload' \
    'exit' |
    bash try.sh "$@" > "$here/try-smoke.log" 2>&1
status=$?
log=$(cat "$here/try-smoke.log")

check "try.sh exits 0"                         '[ "$status" -eq 0 ]'
check "the workspace's tag is niri_tasks"      'grep -qx "tag=niri_tasks" <<<"$log"'
check "niritasks is this worktree's build"     'grep -q "^bin=$PWD/target/" <<<"$log"'
check "the copy holds the real niri_tasks tasks" 'grep -qx "count=$real_count" <<<"$log"'
check "a task added in the copy"               'grep -qx added <<<"$log"'
check "the notification shows in the shell"    'grep -q "\[notify\] try-smoke" <<<"$log"'
check "the daemon started, then restarted"     '[ "$(grep -c "^daemon [0-9]* on " <<<"$log")" -eq 2 ]'
check "niri-tasks.service untouched"           '[ "$(systemctl --user show -p MainPID --value niri-tasks.service)" = "$svc_before" ]'
check "~/.cargo/bin/niritasks untouched"       '[ "$(sha256sum ~/.cargo/bin/niritasks)" = "$bin_before" ]'
check "the real database untouched"            '[ "$(task rc.verbose=nothing export | sha256sum)" = "$db_before" ]'
check "no try-smoke task in the real database" '[ "$(task rc.verbose=nothing description:"test: try smoke" count)" = 0 ]'
echo "log: $here/try-smoke.log"
[ "$fails" -eq 0 ]
```

(The real-database checks assume no one edits real tasks during the run; if one fails, rerun once before suspecting try.sh.)

- [ ] **Step 2: Run it to see it fail**

Run: `bash "$SCRATCH/try-smoke.sh"`
Expected: FAIL on `try.sh exits 0` and every check reading the log (`bash: try.sh: No such file or directory`); the untouched checks PASS.

- [ ] **Step 3: Write `try.sh`**

Create `try.sh` with exactly:

```bash
#!/usr/bin/env bash
# Try this checkout's niritasks by hand, in a nested niri of its own:
#
#   bash try.sh             # a debug build
#   bash try.sh --release   # a release build, to see how the panel animates
#
# It builds this checkout, copies your task database into a sandbox, and
# opens a nested niri as an ordinary window on your workspace, focused. Its
# one workspace is named after this project, so the panel shows the
# project's real tasks; niri/niri-tasks.kdl is included as is, with Mod on
# Right Alt, since your own niri takes any Super combination before the
# nested window sees it. A daemon built from this checkout serves it. This
# terminal becomes the try shell, whose niritasks, task and niri msg reach
# the nested niri and the copy:
#
#   reload   rebuild, and restart the nested daemon on the new binary
#   exit     close the nested niri and throw the copy away
#
# Notifications are printed in the try shell rather than shown on your
# desktop. Nothing is written back to your task database.
#
# It never touches what is installed: ~/.cargo/bin/niritasks, the
# niri-tasks service and install.sh's links stay as they are, and your own
# daemon keeps running. The sandbox is tests/lib/nested-niri.sh's, the one
# the e2e tests use, so several tries and test runs, from any worktrees, run
# beside each other. Installing a build for daily use is still install.sh.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd) || exit 1

PROFILE=debug
case "${1:-}" in
    "") ;;
    --release) PROFILE=release ;;
    *) echo "usage: bash try.sh [--release]" >&2; exit 2 ;;
esac
BUILD=(build --manifest-path "$ROOT/Cargo.toml")
[ "$PROFILE" = release ] && BUILD+=(--release)

# By its full path: the try shell's PATH drops every directory holding a
# herdr, and cargo may sit in one of them.
CARGO=$(command -v cargo) || { echo "cargo is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
"$CARGO" "${BUILD[@]}" || exit 1

# The project: the main checkout's folder, whether this is it or a worktree
# of it. niri-tasks gives the workspace niri-tasks, which is tag niri_tasks.
COMMON=$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
if [ -n "$COMMON" ]; then
    PROJECT=$(basename "$(dirname "$COMMON")")
else
    PROJECT=$(basename "$ROOT")
fi

# Asked before the sandbox's TASKRC takes the place of yours.
REAL_DB="$(task _get rc.data.location 2>/dev/null)/taskchampion.sqlite3"
[ -f "$REAL_DB" ] || { echo "no task database at $REAL_DB" >&2; exit 1; }

. "$ROOT/tests/lib/nested-niri.sh"
# The daemon is the try shell's (daemon_restart below), so its pid is read
# back here for cleanup to stop it. HUP too: closing the terminal is how a
# try often ends.
trap 'DAEMON=$(cat "$SB/daemon.pid" 2>/dev/null); cleanup' EXIT
trap 'exit 1' INT TERM HUP

TARGET=$("$CARGO" metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" |
    python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])') ||
    die "cargo would not say where it builds"
BIN="$TARGET/$PROFILE/niritasks"
[ -x "$BIN" ] || die "no $BIN after the build"

# An online backup: consistent even while your daemon writes, and the copy
# is the only database anything here writes to.
python3 - "$REAL_DB" "$TASKDATA/taskchampion.sqlite3" <<'PY' || die "could not copy $REAL_DB"
import pathlib, sqlite3, sys
src = sqlite3.connect(pathlib.Path(sys.argv[1]).as_uri() + "?mode=ro", uri=True)
dst = sqlite3.connect(sys.argv[2])
src.backup(dst)
dst.close()
src.close()
PY

# Through a link, so a rebuild is picked up and $SB/bin, first on every
# nested PATH, finds this build for the shell, the daemon and the binds.
mkdir -p "$SB/bin" "$SB/share"
ln -sf "$BIN" "$SB/bin/niritasks"
NIRITASKS="$SB/bin/niritasks"
# The binds' spawns get no herdr, as in the e2e tests.
NESTED_SPAWN_PATH="$SB/bin:/usr/bin:/bin"
NESTED_WORKSPACE="$PROJECT"
# Right Alt as Mod: lv3:ralt_switch makes it ISO_Level3_Shift. Your niri
# binds no Alt combination without Super, bar Alt+Print and Ctrl+Alt+Delete,
# so these reach the nested window.
NESTED_EXTRA_KDL="input { keyboard { xkb { options \"lv3:ralt_switch\"; }; }; mod-key-nested \"ISO_Level3_Shift\"; }
include \"$ROOT/niri/niri-tasks.kdl\""
nested_start --focused
# The Ideas tab saves under XDG_DATA_HOME: the sandbox's, never yours.
NENV+=(XDG_DATA_HOME="$SB/share")

# Notifications, as they come, in this terminal.
touch "$SB/notifications"
tail -n0 -F --pid=$$ "$SB/notifications" 2>/dev/null | sed -u 's/^/[notify] /' &

cat > "$SB/try.bashrc" <<'RC'
# The try shell's rc, written by try.sh. Not ~/.bashrc, which could put
# herdr back on PATH.
PS1="(try $TRY_PROJECT) \w \$ "

# Stop the nested daemon, if one runs, and start one on the current build.
daemon_restart() {
    local pid
    pid=$(cat "$TRY_SB/daemon.pid" 2>/dev/null)
    if [ -n "$pid" ] && kill "$pid" 2>/dev/null; then
        for _ in $(seq 1 50); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.1
        done
        kill -9 "$pid" 2>/dev/null
    fi
    # In a subshell, so an interactive shell neither reports it as a job
    # nor waits for it.
    ( niritasks daemon >>"$TRY_SB/daemon.log" 2>&1 & echo $! > "$TRY_SB/daemon.pid" )
    sleep 2
    pid=$(cat "$TRY_SB/daemon.pid")
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "the daemon exited at once:" >&2
        tail -n 20 "$TRY_SB/daemon.log" >&2
        return 1
    fi
    echo "daemon $pid on $(readlink -f "$(command -v niritasks)")"
}

# Rebuild this checkout, then restart the daemon on it. The window and the
# copy of your tasks stay.
reload() {
    local build=(build --manifest-path "$TRY_ROOT/Cargo.toml")
    [ "$TRY_PROFILE" = release ] && build+=(--release)
    "$TRY_CARGO" "${build[@]}" && daemon_restart
}
RC
TRY_ENV=(TRY_ROOT="$ROOT" TRY_SB="$SB" TRY_CARGO="$CARGO" TRY_PROFILE="$PROFILE" TRY_PROJECT="$PROJECT")
nested env "${TRY_ENV[@]}" bash -c '. "$TRY_SB/try.bashrc" && daemon_restart' ||
    die "the nested daemon would not start"

cat <<EOF

The window that just opened is a nested niri on workspace "$PROJECT"
(tag $(nested niritasks tag 2>/dev/null)), on a copy of your tasks and this
checkout's $PROFILE build. Its binds are niri/niri-tasks.kdl's with Right
Alt as Mod: Right Alt+Alt+T adds a task, Right Alt+Alt+Ctrl+T shows the
panel. Here, niritasks, task and niri msg reach it.

  reload   rebuild and restart the nested daemon
  exit     close it all; the copy is thrown away

Daemon log: $SB/daemon.log
EOF
nested env "${TRY_ENV[@]}" bash --rcfile "$SB/try.bashrc" -i
```

Then match `install.sh`'s file mode: run `stat -c %a install.sh`; if it prints `755`, run `chmod +x try.sh`.

- [ ] **Step 4: Check syntax and the generated config**

Run: `bash -n try.sh && echo syntax ok`
Expected: `syntax ok`

Check the KDL try.sh will hand niri, with `niri validate` on a copy built the same way:

```bash
cat > "$SCRATCH/try-config.kdl" <<EOF
workspace "niri-tasks"
input { keyboard { xkb { options "lv3:ralt_switch"; }; }; mod-key-nested "ISO_Level3_Shift"; }
include "$PWD/niri/niri-tasks.kdl"
EOF
niri validate -c "$SCRATCH/try-config.kdl" 2>&1 | tail -n 1
```

Expected: a line ending `config is valid`.

- [ ] **Step 5: Run the smoke test to see it pass**

Tell the user a window will open for about ten seconds. Run: `bash "$SCRATCH/try-smoke.sh"`
Expected: every check `PASS`, exit 0. If `the daemon started, then restarted` fails, read the log for `the daemon exited at once:` and the daemon's own error (a stale socket in the nested runtime dir is the first suspect: compare with how `nested_daemon_stop` / `nested_daemon_start` sequence a restart).

Then the release flag. Run: `cargo build --release 2>&1 | tail -n 1 && bash "$SCRATCH/try-smoke.sh" --release`
Expected: every check `PASS`, and `grep '^bin=' "$SCRATCH/try-smoke.log"` shows `.../target/release/niritasks`.

- [ ] **Step 6: Replace the README's try-by-hand advice**

In `README.md`, replace this paragraph (the last one inside the `> [!IMPORTANT]` callout):

```markdown
> And to try a change by hand rather than under test, either `bash install.sh`,
> which restarts the daemon for you, or `cargo install --path .` followed by
> `systemctl --user restart niri-tasks` — `cargo install` alone leaves the
> running daemon on the previous binary, so panel changes will not show up.
```

with:

```markdown
> To try a change by hand rather than under test, use `bash try.sh` (below),
> not `install.sh` or `cargo install`: both replace the `niritasks` your
> daemon and every other worktree's test runs use.
```

Then, directly after the callout (before `### Why the three scripts are not cargo tests`), add:

````markdown
### Trying a change by hand

```bash
bash try.sh             # a debug build
bash try.sh --release   # a release build, to see how the panel animates
```

Run from any checkout or worktree, `try.sh` builds it and opens it in a
nested niri — the same sandbox the e2e tests use (`tests/lib/nested-niri.sh`),
but as an ordinary window, focused, on your workspace. Its one workspace is
named after the project, so the panel shows the project's tasks from a copy
of your task database, taken with an SQLite online backup; nothing is written
back. `niri/niri-tasks.kdl` is included with Mod on Right Alt, since your own
niri takes Super combinations first: Right Alt+Alt+T adds a task, Right
Alt+Alt+Ctrl+T shows the panel. Notifications are printed in the terminal
rather than shown on your desktop.

The terminal becomes a try shell whose `niritasks`, `task` and `niri msg`
reach the nested niri. `reload` rebuilds and restarts the nested daemon,
keeping the window and the copy; `exit` closes everything. The installed
binary, the `niri-tasks` service and `install.sh`'s links are never touched,
so tries in several worktrees run beside each other and beside `tests/all.sh`.
herdr is kept out, so Start working and Go to session do nothing there.
Installing a build for daily use is still `bash install.sh`, on main.
````

- [ ] **Step 7: Commit**

```bash
git add try.sh README.md
git commit -m "$(cat <<'EOF'
feat: add try.sh to try a build in a nested niri

Trying a change by hand meant install.sh or cargo install, which
replace the niritasks the live daemon and other worktrees' e2e runs
use. try.sh builds the checkout and opens it in the e2e sandbox,
focused on a workspace named after the project, with a copy of the
real tasks, the niri-tasks binds on Right Alt, notifications in the
terminal and a reload command. The README points to it instead.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Acceptance by hand, with the user

No new code unless Step 2 needs the fallback. The controller runs this with the user; a subagent cannot press keys.

**Files:**
- Modify (only if Right Alt fails): `try.sh` (the `NESTED_EXTRA_KDL=` assignment, its comment and the banner), `README.md` (the Right Alt sentences)

- [ ] **Step 1: Record the untouched state**

```bash
systemctl --user show -p MainPID --value niri-tasks.service > "$SCRATCH/accept-svc"
sha256sum ~/.cargo/bin/niritasks > "$SCRATCH/accept-bin"
task rc.verbose=nothing export | sha256sum > "$SCRATCH/accept-db"
```

- [ ] **Step 2: The binds, with the user**

Ask the user to run `bash try.sh` in a terminal of this worktree, then in the nested window: Right Alt+Alt+T (a task box opens, filing under `niri_tasks`), Right Alt+Alt+Ctrl+T (the panel shows the project's real tasks), Right Alt+Return (a terminal attempt — it may fail without herdr; only that the bind fires matters).

If the binds do not fire with Right Alt, fall back to plain Alt: in `try.sh` replace the `NESTED_EXTRA_KDL=` assignment and the comment above it with

```bash
# Alt as Mod, niri's own default for a nested niri: Right Alt as Mod did not
# reach the binds. Your niri binds no Alt combination without Super, bar
# Alt+Print and Ctrl+Alt+Delete, so these reach the nested window.
NESTED_EXTRA_KDL="include \"$ROOT/niri/niri-tasks.kdl\""
```

and in the banner and README replace `Right Alt as Mod` / `with Mod on Right Alt` with `Alt as Mod` / `with Mod on Alt`, and the bind examples with `Alt+T adds a task, Alt+Ctrl+T shows the panel` (Mod+Alt+T is Alt+T when Mod is Alt). Rerun Task 2 Step 5, retry this step, and commit with `fix: put the try window's Mod on plain Alt`.

- [ ] **Step 3: reload, notifications, with the user**

With the try window still open: make a visible edit (for example, change a panel label string in `src/panel/`), run `reload` in the try shell, reopen the panel and confirm the edit shows; then revert the edit with `git checkout -- src/` and `reload` again. Add a task through the box and confirm its notification appears as a `[notify]` line in the try shell and not as a desktop popup. `exit`.

- [ ] **Step 4: Side by side**

With the user: open a try window from this worktree and from a second worktree (any other niri-tasks worktree, or a throwaway one: `git worktree add "$SCRATCH/wt2" HEAD`, then `bash "$SCRATCH/wt2/try.sh"`), and in a third terminal run `NIRITASKS=$PWD/target/release/niritasks bash tests/all.sh`. Expected: all.sh ends `0 failed`, both try windows keep working throughout, and each closes cleanly on `exit`. Remove the throwaway worktree afterwards with `git worktree remove "$SCRATCH/wt2"`.

- [ ] **Step 5: Check nothing real changed**

```bash
diff <(systemctl --user show -p MainPID --value niri-tasks.service) "$SCRATCH/accept-svc" && echo service same
sha256sum -c "$SCRATCH/accept-bin"
diff <(task rc.verbose=nothing export | sha256sum) "$SCRATCH/accept-db" && echo db same
```

Expected: `service same`, `...niritasks: OK`, `db same`. (The database check assumes the user made no real task changes during acceptance; if they did, confirm by hand that no `test:` task from the try sessions is in the real database.)
