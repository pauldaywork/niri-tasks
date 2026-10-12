# End the Try Shell When Its Nested niri Exits — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the nested niri a try opened dies (its window closed from your niri, or otherwise), the try shell ends and cleanup runs at once, instead of sitting on dead sockets until you type `exit`.

**Architecture:** `try.sh` starts a watcher once the nested daemon is up: a background subshell polling `kill -0 "$NESTED"` every 0.5s. When the nested niri is gone it prints a line, then sends SIGHUP to the try shell (whose pid `try.bashrc` writes to `$SB/shell.pid`), or to `try.sh` itself when that fails. The watcher's pid goes in `WATCH`, which `tests/lib/nested-niri.sh`'s `cleanup` already kills first, so a typed `exit` or a closed terminal never trips it.

**Tech Stack:** bash, niri 26.04 (`niri msg`), util-linux `script` (for the probe's pty).

**Spec:** Taskwarrior task `6914a564-76f4-43ff-92d6-197a8c9d089e` — read it with `task rc.json.array=on 6914a564-76f4-43ff-92d6-197a8c9d089e export`; its description and annotations are the spec.

## Global Constraints

- The watcher lives in `try.sh`, started after the daemon is up: a background subshell polling `kill -0` on the nested niri's pid every 0.5s, its pid kept in `WATCH` so cleanup kills it first and a typed exit never trips it.
- It ends the shell with SIGHUP (an interactive bash ignores SIGTERM). A command still running in the shell gets the HUP too, as on a closed terminal.
- `try.bashrc` writes the shell's pid to `$TRY_SB/shell.pid` as its first line, only when the shell is interactive (`[[ $- == *i* ]]`): the daemon-start `bash -c` sources the rc too.
- If the HUP to the pid in `shell.pid` fails, the watcher HUPs `try.sh` itself.
- Before the HUP the watcher prints, after a newline (the prompt is mid-line): `the nested niri exited, closing the try`.
- Out of scope: fencing the workspace drop from a close done in your niri; how e2e runs park their windows; the e2e tests themselves. `nested-niri.sh` changes are comments only.
- Never touch `~/.cargo/bin`, the `niri-tasks` service or install.sh's links; never run `install.sh`.
- Commits: Conventional Commits, subject ≤72 chars (aim ~50), imperative, lowercase; body wrapped at 72 saying what and why; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comments match the repo's style: full sentences that say why, in the plain voice `try.sh` and `nested-niri.sh` use.
- Work only in this worktree: `/home/paul/.worktrees/niri-tasks/task-fix-end-the-try-shell-when-its-nested-6914a564`. Scratch files go in a scratch directory outside the repo (`$SCRATCH` below), never committed.
- **Every `try.sh` run opens a focused nested-niri window on the user's current workspace for a few seconds.** Before each probe run, tell the user in one line that a window is about to open.

Already verified while planning: bash reaps a background child while it waits on a foreground one, so `kill -0 "$NESTED"` from a subshell fails within one poll of the nested niri exiting (a probe with `sleep 1 &` watched from a subshell saw it gone after 1s and HUPed the foreground child, which returned 129). Bash defers a trapped signal until the foreground command returns, so the fallback HUP to `try.sh` ends the try only once its current foreground command finishes; the watcher starts after the daemon, so that path covers only the short gap before the try shell's rc runs.

---

### Task 1: The watcher, the pid file and the comments

**Files:**
- Modify: `try.sh` — header (lines 13–17), the `try.bashrc` heredoc (starts line 130), and a new block between the daemon start (`nested env "${TRY_ENV[@]}" bash -c ...`, line 170–171) and the banner `cat <<EOF` (line 173)
- Modify: `tests/lib/nested-niri.sh:38-41` (header comment on try.sh)
- Test: `$SCRATCH/probe-watch.sh` (scratch, not committed)

**Interfaces:**
- Consumes (from `tests/lib/nested-niri.sh`, unchanged): `$NESTED` (nested niri pid, a child of `try.sh`), `$SB` (sandbox dir), `$WATCH` (killed and waited first by `cleanup`), `cleanup`.
- Produces: `$SB/shell.pid` — the interactive try shell's pid, written by `try.bashrc`'s first command. Stdout line `the nested niri exited, closing the try`, preceded by a newline.

- [ ] **Step 1: Write the probe**

Create `$SCRATCH/probe-watch.sh`. It runs `try.sh` in a pty under `script`, ends the try one of three ways, and checks the Done-when list from the spec:

```bash
#!/usr/bin/env bash
# Runs try.sh in a pty and ends it one way:
#   close    close the nested window from this niri
#   exit     type exit in the try shell
#   hangup   close the terminal (kill script, so the pty hangs up)
# Run from the worktree root. Prints PASS/FAIL lines; exits 1 on any FAIL.
set -u
how=${1:?usage: probe-watch.sh close|exit|hangup}
MSG="the nested niri exited, closing the try"
dir=$(mktemp -d)
mkfifo "$dir/in"
exec 3<>"$dir/in"   # held open, so the try shell never reads EOF
SHELL=/bin/bash script -qfec "bash try.sh" "$dir/out" <&3 >/dev/null 2>&1 &
SCRIPT=$!
fails=0
pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; fails=$((fails+1)); }
now() { date +%s.%N; }
since() { python3 -c 'import sys; print(f"{float(sys.argv[2]) - float(sys.argv[1]):.2f}")' "$1" "$(now)"; }

# Until the banner: a build can take minutes. Then the shell's pid, which
# its rc writes at once.
SB=""
for _ in $(seq 1 1200); do
    SB=$(sed -n 's/^Daemon log: \(.*\)\/daemon\.log.*$/\1/p' "$dir/out" | head -n1)
    [ -n "$SB" ] && break
    kill -0 "$SCRIPT" 2>/dev/null || break
    sleep 0.5
done
for _ in $(seq 1 50); do
    [ -n "$SB" ] && [ -s "$SB/shell.pid" ] && break
    sleep 0.1
done
if [ -z "$SB" ] || [ ! -s "$SB/shell.pid" ]; then
    echo "FAIL  the try never wrote \$SB/shell.pid; its output:"
    tr -d '\r' < "$dir/out"
    kill -9 "$SCRIPT" 2>/dev/null
    exit 1
fi
sleep 1   # let the shell print its prompt
SHELL_PID=$(cat "$SB/shell.pid")
TRY_PID=$(ps -o sid= -p "$SHELL_PID" | tr -d ' ')
case "$(ps -o args= -p "$TRY_PID")" in
    *try.sh*) pass "shell.pid ($SHELL_PID) is the try shell, in try.sh's session ($TRY_PID)" ;;
    *) fail "shell.pid's session leader $TRY_PID is not try.sh: $(ps -o args= -p "$TRY_PID")" ;;
esac
# The daemon is reparented away from try.sh, so it is added by its pid file.
descendants() { local p; for p in $(ps -o pid= --ppid "$1"); do echo "$p"; descendants "$p"; done; }
PIDS="$(descendants "$TRY_PID") $(cat "$SB/daemon.pid")"
NIRI=$(pgrep -f "niri -c $SB/niri.kdl")
WIN=$(niri msg -j windows | python3 -c '
import json, sys
print(next((w["id"] for w in json.load(sys.stdin) if w.get("pid") == int(sys.argv[1])), ""))' "$NIRI")
[ -n "$NIRI" ] && [ -n "$WIN" ] || fail "could not find the nested niri ($NIRI) or its window ($WIN)"

t0=$(now)
case "$how" in
    close)  niri msg action close-window --id "$WIN" ;;
    exit)   echo exit >&3 ;;
    hangup) kill -9 "$SCRIPT" ;;
esac

if [ "$how" = close ]; then
    for _ in $(seq 1 30); do grep -q "$MSG" "$dir/out" && break; sleep 0.05; done
    if grep -q "$MSG" "$dir/out"; then
        pass "the message printed after $(since "$t0")s"
    else
        fail "no \"$MSG\" within 1.5s"
    fi
fi

for _ in $(seq 1 150); do kill -0 "$TRY_PID" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$TRY_PID" 2>/dev/null; then
    fail "try.sh still runs 15s after the $how"
else
    pass "try.sh ended $(since "$t0")s after the $how"
fi

[ "$how" = exit ] && { grep -q "$MSG" "$dir/out" && fail "exit printed \"$MSG\"" || pass "exit printed no message"; }
[ -d "$SB" ] && fail "$SB is still there" || pass "\$SB is gone"
sleep 2   # tail --pid polls once a second, and a killed watcher's sleep lasts 0.5s
left=""
for p in $PIDS; do kill -0 "$p" 2>/dev/null && left="$left $p($(ps -o comm= -p "$p"))"; done
[ -z "$left" ] && pass "no process of the try is left" || fail "left running:$left"

exec 3>&-
rm -rf "$dir"
[ "$fails" -eq 0 ]
```

Then `chmod +x "$SCRATCH/probe-watch.sh"`.

- [ ] **Step 2: Run the probe against today's try.sh and see it fail**

Tell the user a try window is about to open for a few seconds, then run from the worktree root:

Run: `bash "$SCRATCH/probe-watch.sh" close`
Expected: `FAIL  the try never wrote $SB/shell.pid; its output:` followed by try.sh's output ending in the banner (today's rc writes no pid). The probe kills `script`, which hangs up the pty and so tears the try down. Check `pgrep -fa 'niri -c /tmp'` shows no nested niri left from it; if one is, `kill` it.

- [ ] **Step 3: Write the pid line in try.bashrc**

In `try.sh`, the heredoc starts:

```bash
cat > "$SB/try.bashrc" <<'RC'
# The try shell's rc, written by try.sh. Not ~/.bashrc, which could put
# herdr back on PATH.
# Its own history: the try shell's commands stay out of ~/.bash_history.
HISTFILE="$TRY_SB/history"
```

Make the pid the rc's first command, right after its two-line header comment:

```bash
cat > "$SB/try.bashrc" <<'RC'
# The try shell's rc, written by try.sh. Not ~/.bashrc, which could put
# herdr back on PATH.
# Its pid, first, for try.sh to end this shell when the nested niri exits.
# Only an interactive shell's: daemon_restart's bash -c sources this too,
# and would leave a dead pid behind.
if [[ $- == *i* ]]; then echo $$ > "$TRY_SB/shell.pid"; fi
# Its own history: the try shell's commands stay out of ~/.bash_history.
HISTFILE="$TRY_SB/history"
```

`if` rather than `[[ ]] &&`, so a non-interactive source never ends on a failed test.

- [ ] **Step 4: Add the watcher**

In `try.sh`, between

```bash
nested env "${TRY_ENV[@]}" bash -c '. "$TRY_SB/try.bashrc" && daemon_restart' ||
    die "the nested daemon would not start"
```

and the banner's `cat <<EOF`, insert:

```bash

# A nested niri that dies, its window closed from your niri say, ends the
# try at once, rather than leave the try shell on dead sockets until you
# type exit. The shell gets SIGHUP, as from a closed terminal: an
# interactive bash ignores SIGTERM. Until the shell has written its pid,
# try.sh gets it instead, and its HUP trap runs once what it is running
# returns. cleanup kills the watcher first, so exit never trips it.
(
    while kill -0 "$NESTED" 2>/dev/null; do sleep 0.5; done
    printf '\nthe nested niri exited, closing the try\n'
    kill -HUP "$(cat "$SB/shell.pid" 2>/dev/null)" 2>/dev/null || kill -HUP $$
) &
WATCH=$!
```

`$$` in the subshell is `try.sh`'s pid. `kill -HUP ""` fails when there is no pid file, which takes the fallback.

- [ ] **Step 5: Say it in try.sh's header**

In `try.sh`, the header reads:

```bash
# terminal becomes the try shell, whose niritasks, task and niri msg reach
# the nested niri and the copy:
#
#   reload   rebuild, and restart the nested daemon on the new binary
#   exit     close the nested niri and throw the copy away
#
```

Add after the `exit` line's blank line:

```bash
# terminal becomes the try shell, whose niritasks, task and niri msg reach
# the nested niri and the copy:
#
#   reload   rebuild, and restart the nested daemon on the new binary
#   exit     close the nested niri and throw the copy away
#
# Closing the nested window from your niri ends the try shell too, and the
# copy goes as with exit.
#
```

- [ ] **Step 6: Fix nested-niri.sh's comment**

In `tests/lib/nested-niri.sh`, replace

```bash
# try.sh starts the same sandbox by hand with `nested_start --focused`: the
# window is not parked but left where niri opens it, on your workspace with
# the focus, sized like any other window, and nothing watches it. Its config
# keeps niri's animations and key repeat, which only a test wants off.
```

with

```bash
# try.sh starts the same sandbox by hand with `nested_start --focused`: the
# window is not parked but left where niri opens it, on your workspace with
# the focus, sized like any other window, and nothing watches its focus.
# try.sh watches only that the nested niri still runs, in WATCH, to end the
# try when it does not. Its config keeps niri's animations and key repeat,
# which only a test wants off.
```

- [ ] **Step 7: Check the syntax**

Run: `bash -n try.sh && bash -n tests/lib/nested-niri.sh && echo ok`
Expected: `ok`

- [ ] **Step 8: Run the probe, all three ways**

Tell the user a try window will open and close three times, one after another. Then:

Run: `for how in close exit hangup; do echo "== $how"; bash "$SCRATCH/probe-watch.sh" "$how" || echo "PROBE FAILED: $how"; done`
Expected, for `close`: PASS lines for shell.pid, `the message printed after 0.xs` (under 1s), `try.sh ended …`, `$SB is gone`, `no process of the try is left`. For `exit`: the same bar the message, plus `exit printed no message`. For `hangup`: shell.pid, ended, gone, nothing left. No `FAIL` and no `PROBE FAILED`.

If `close` ends but the message is missing, check that the watcher's stdout is the pty (it is the subshell's inherited stdout). If a process is left, the line names it: a `sleep` or `tail` means the 2s grace was too short to tell, so rerun once before treating it as real.

- [ ] **Step 9: Commit**

```bash
git add try.sh tests/lib/nested-niri.sh
git commit -F - <<'EOF'
fix: end the try shell when its nested niri exits

Closing the nested window from your niri, or the nested niri dying any
other way, left the try shell on dead sockets until you typed exit.
try.sh now watches the nested niri's pid and, when it is gone, says so
and hangs up the try shell, as a closed terminal would, so cleanup runs
at once. try.bashrc records the interactive shell's pid for it; cleanup
stops the watcher first, so exit and a closed terminal end a try as
before.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

### Task 2: The user's own check

The spec's Done-when is "close the nested window with your niri's close bind", which only the user can press. Hand it over; do not run it yourself.

- [ ] **Step 1: Ask the user to check by hand**

Tell the user, in these words or close to them:

> In this worktree, run `bash try.sh`. Once the try shell's prompt is up, focus the nested window and close it with your niri's close bind. You should see `the nested niri exited, closing the try` and get your own prompt back within a second, and `$SB` (the dir in the `Daemon log:` line) should be gone. Then run it again and type `exit`: no message. `ps -ef | grep -E 'niri -c /tmp|niritasks daemon|tail -n0 -F'` should show nothing of the try after each (your own `niritasks daemon` stays).

- [ ] **Step 2: Record what they report**

If they report a failure, go back to Task 1 with what they saw. Otherwise the task is done; land it with the `finish-worktree` skill.
