# Task box e2e test in the same nested niri — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tests/e2e-box.sh` types into a nested niri it owns, the same way `tests/e2e-panel.sh` already does, so it never presses keys on the real desktop and never stops `niri-tasks.service`.

**Architecture:** Move the nested-niri harness out of `tests/e2e-panel.sh` into `tests/lib/nested-niri.sh`, which both scripts source. The harness makes the sandbox, starts and parks the nested niri, watches its focus, and tears everything down. It also turns key repeat off in the nested niri. Then `e2e-box.sh` runs every `wtype`, `niri msg`, `niritasks task …` and its daemon through the harness's `nested` prefix. The no-daemon half runs with no daemon *inside the nested niri*, where the box looks for one, instead of stopping the user's service.

**Tech Stack:** bash, niri 26.04 (`niri msg`), wtype, python3, taskwarrior.

**Spec:** Taskwarrior task `9c875e20` — read it with `task rc.json.array=on 9c875e20 export`. Its last annotation, "Decided (2026-10-01): scope grows to migrate tests/e2e-box.sh…", is the requirement for this plan. It overrides that task's earlier "Out of scope: … e2e-box.sh". The rest of the task, and `docs/superpowers/plans/2026-10-01-e2e-panel-nested-niri.md` (Tasks 1–4, done), describe the harness being moved.

## Global Constraints

- Nothing either e2e test does may reach the real desktop. Every `wtype`, `niri msg action …`, `niri msg -j …` about the box, `niritasks task …` and `niritasks daemon` runs inside the nested niri. The parent niri is only asked to park the nested window and to watch its focus.
- `niri-tasks.service` is never stopped, started or restarted. Its main pid must be the same at the end of each run (`nested_service_untouched`).
- No `pkill`, and no closing boxes on the real desktop.
- The nested niri config contains `input { keyboard { repeat-rate 0; }; }`. In a probe on 2026-10-01, the first box in a fresh nested niri doubled typed characters in 3 of 3 runs with repeat on, and was clean in 2 of 2 runs with it off. (Note the `;` after `}` — `niri validate` rejects the one-line form without it.)
- The pointer checks (×, "+ Add note", scrolling) stay manual. They are out of scope.
- `e2e-panel.sh` must keep passing 13/13 with the same numbers (peek 1566-1600, cards 420-1180) after the harness moves out of it.
- Leave nothing behind: no window, workspace, socket or temp dir.
- Comments and output follow the existing scripts' voice: plain sentences saying why.

## Facts the executor needs

- A probe on 2026-10-01 drove the box inside the nested niri. `niritasks task add` opened it, it took the nested focus, typed text and Ctrl+Enter wrote the task, and `due:friday` was parsed. The parent's focused window never changed, and the real service pid was unchanged. With no daemon running inside the nested niri, `niritasks task add` built its own box, because the box looks for `niri-tasks.sock` under `$XDG_RUNTIME_DIR` (`src/ipc.rs::socket_path`), which is the nested runtime dir.
- `env` execs, so `env … cmd &` gives `$!` = cmd's pid. A backgrounded **shell function** gives the subshell's pid. The harness only backgrounds `env` directly.
- Run against this worktree's build: `cargo build --release`, then `NIRITASKS=./target/release/niritasks bash tests/<script>`.
- Each run opens a niri window on the current workspace for a moment and parks it on the last workspace of that monitor. Don't switch to that workspace.
- **Never run `tests/all.sh` or the *old* `tests/e2e-box.sh`.** The old box test types into the real desktop. Run the box test only once Task 6 has rewritten it.

---

### Task 5: Move the nested-niri harness into tests/lib/nested-niri.sh

**Files:**
- Create: `tests/lib/nested-niri.sh`
- Modify: `tests/e2e-panel.sh`

**Interfaces:**
- Produces (Task 6 relies on these exact names):
  - Sourcing runs the prerequisite checks (niri, taskwarrior, python3, `$NIRI_SOCKET`, `$WAYLAND_DISPLAY`). It creates `SB` (sandbox dir), with `TASKRC`/`TASKDATA` exported into it, plus the nested runtime dir. It sets the EXIT/INT/TERM traps that tear everything down, and sets `NIRITASKS` (default `niritasks`).
  - `NESTED_W=1600`, `NESTED_H=1000`.
  - `pass fail skipped` counters, and `ok`, `bad`, `skip`, `die`, `summary`.
  - `nested_start`: starts and parks the nested niri, prints `nested niri: window <id>, sockets in <dir>`, passes `the nested niri is parked unfocused on a spare workspace, 1600x1000`, and starts the focus watch. It sets `NENV`.
  - `nested <cmd…>`: runs a command inside the nested niri (foreground only).
  - `guard`: fails and ends the run if the focus watch saw the nested window focused, shown, gone, or unaskable.
  - `nested_daemon_start <log file>`: starts `niritasks daemon` inside the nested niri, sets `DAEMON`, and dies with the log's tail if the daemon exits within 3s.
  - `nested_daemon_stop`: stops it and waits; `DAEMON` becomes empty.
  - `nested_service_untouched`: calls `guard`, then passes or fails on the real service's main pid.

- [ ] **Step 1: Create `tests/lib/nested-niri.sh`**

```bash
# A niri of its own, for the e2e tests that must not touch your desktop:
# tests/e2e-panel.sh and tests/e2e-box.sh. Source it, then call nested_start.
#
#   . "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"
#   nested_start
#
# It starts a nested niri — a window on yours, with its own runtime dir and
# sockets and one 1600x1000 output of flat colour — and parks that window,
# floating and unfocused, on the last workspace of the monitor you start
# from. Everything under test then runs inside it through `nested`: the
# daemon, the box, `niri msg`, and wtype, whose keys reach only the niri its
# WAYLAND_DISPLAY names. Nothing you type reaches the nested niri, and nothing
# typed into it reaches you — unless you go to that workspace, which focuses
# the window. Then the run fails, saying so, rather than trust what follows.
#
# Sourcing it also makes the sandbox: a TASKDATA of its own, so the real task
# database is never touched, and traps that tear all of it down on any exit.
# Your own niri-tasks daemon keeps running throughout, and never sees these
# tasks: the daemon socket a nested program looks for is under the nested
# runtime dir. nested_service_untouched proves it at the end of a run.

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }
[ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display to open the nested niri on" >&2; exit 1; }

NIRITASKS="${NIRITASKS:-niritasks}"

# The nested niri's one output, in pixels: the size its window is parked at.
NESTED_W=1600
NESTED_H=1000

SB=""; RT=""
# The nested niri connects to this one by its absolute path, since its own
# XDG_RUNTIME_DIR is somewhere else.
case "$WAYLAND_DISPLAY" in
    /*) PARENT_WAYLAND="$WAYLAND_DISPLAY" ;;
    *) PARENT_WAYLAND="${XDG_RUNTIME_DIR:?}/$WAYLAND_DISPLAY" ;;
esac

SB="$(mktemp -d)"
# Short on purpose: the nested niri's sockets and the daemon's go in here, and
# a Unix socket path longer than ~108 characters cannot be bound.
RT="$(mktemp -d /tmp/nte2e.XXXXXX)"
mkdir -p "$SB/data"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

# The real daemon is never touched; this is how the end of the run proves it.
SERVICE_PID=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)

NESTED=""; WIN=""; WATCH=""; DAEMON=""; NENV=()
cleanup() {
    [ -n "$WATCH" ] && kill "$WATCH" 2>/dev/null && wait "$WATCH" 2>/dev/null
    [ -n "$DAEMON" ] && kill "$DAEMON" 2>/dev/null && wait "$DAEMON" 2>/dev/null
    # Its window closes with it, and niri drops the workspace it leaves empty.
    # Anything still running inside it loses its display and exits too.
    [ -n "$NESTED" ] && kill "$NESTED" 2>/dev/null && wait "$NESTED" 2>/dev/null
    [ -n "$RT" ] && rm -rf "$RT"
    if [ -n "${NIRITASKS_E2E_KEEP:-}" ]; then
        echo "the run's files are kept in $SB"
    else
        [ -n "$SB" ] && rm -rf "$SB"
    fi
}
trap cleanup EXIT
trap 'exit 1' INT TERM

pass=0; fail=0; skipped=0
ok()   { echo "  PASS  $*"; pass=$((pass+1)); }
bad()  { echo "  FAIL  $*"; fail=$((fail+1)); }
skip() { echo "  SKIP  $*"; skipped=$((skipped+1)); }
die()  { echo "$*" >&2; exit 1; }
summary() { echo; echo "passed: $pass   failed: $fail   skipped: $skipped"; }

# Run a command inside the nested niri. Foreground only: in the background,
# $! would be a subshell's pid, not the command's.
nested() { "${NENV[@]}" "$@"; }

# The parent's windows that are already nested niris, so the new one can be
# told apart from them.
nested_niri_windows() {
    niri msg -j windows | python3 -c '
import json, sys
print(" ".join(str(w["id"]) for w in json.load(sys.stdin) if w["app_id"] == "niri"))'
}

# Why the nested window can no longer be trusted, or nothing while it can.
# Nothing is only ever a good answer: when this niri cannot be asked, that is
# a reason of its own, never an empty one that reads as "all is well".
UNASKABLE="cannot ask this niri where the nested window is"
focus_reason() {
    python3 - "$WIN" 2>/dev/null <<'PY' || echo "$UNASKABLE"
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next((w for w in get("windows") if w["id"] == int(sys.argv[1])), None)
if win is None:
    print("the nested niri's window is gone")
elif win["is_focused"]:
    print("the nested niri's window has the focus")
else:
    space = next((s for s in get("workspaces") if s["id"] == win["workspace_id"]), None)
    if space is not None and space["is_active"]:
        print(f"the nested niri's workspace ({space['output']}, {space['idx']}) is on screen")
PY
}

# From nested_start on, the window must stay unfocused and off screen. This
# watches for the whole run; guard ends the run the moment it has seen
# otherwise. One failed question is a hiccup; three in a row is a niri that
# cannot be trusted to answer.
watch_focus() {
    local why blind=0
    while sleep 0.25; do
        why=$(focus_reason)
        if [ "$why" = "$UNASKABLE" ]; then
            blind=$((blind+1))
            [ "$blind" -ge 3 ] || continue
        else
            blind=0
        fi
        [ -n "$why" ] && { echo "$why" > "$SB/tampered"; return; }
    done
}
guard() {
    [ -s "$SB/tampered" ] || return 0
    local why; why=$(cat "$SB/tampered")
    case "$why" in
        *"is gone"*|"$UNASKABLE") bad "$why, so this run can no longer be trusted" ;;
        *) bad "$why — anything typed there would reach the nested niri,
      so this run can no longer be trusted. Keep off that workspace while
      this runs" ;;
    esac
    summary
    exit 1
}

nested_start() {
    # Animations off, so niri itself never draws a frame in between. Key
    # repeat off: these tests never hold a key, and a client still starting
    # up can handle a release late enough for its repeat to fire — the first
    # box in a fresh nested niri typed doubled letters that way. A flat
    # background, so a frame with nothing on it is the same every time. One
    # named workspace, which is the tag tasks are filed under. The startup
    # command is how the test learns the nested niri's own sockets.
    cat > "$SB/niri.kdl" <<EOF
hotkey-overlay { skip-at-startup; }
animations { off; }
input { keyboard { repeat-rate 0; }; }
xwayland-satellite { off; }
output "winit" { scale 1; }
layout { background-color "#406080"; }
workspace "e2e"
spawn-sh-at-startup "env > $SB/nested.env.tmp && mv $SB/nested.env.tmp $SB/nested.env"
EOF

    local before spare opened_on landed why size
    before=$(nested_niri_windows) || die "cannot list this niri's windows"

    env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
    NESTED=$!

    for _ in $(seq 1 50); do
        [ -s "$SB/nested.env" ] && break
        kill -0 "$NESTED" 2>/dev/null || break
        sleep 0.2
    done
    [ -s "$SB/nested.env" ] || die "the nested niri did not start:
$(tail -n 20 "$SB/niri.log")"
    local n_socket n_wayland
    n_socket=$(sed -n 's/^NIRI_SOCKET=//p' "$SB/nested.env")
    n_wayland=$(sed -n 's/^WAYLAND_DISPLAY=//p' "$SB/nested.env")
    [ -n "$n_socket" ] && [ -n "$n_wayland" ] ||
        die "the nested niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"
    NENV=(env XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$n_wayland" NIRI_SOCKET="$n_socket")

    for _ in $(seq 1 25); do
        WIN=$(python3 -c '
import sys
new = set(sys.argv[2].split()) - set(sys.argv[1].split())
print(new.pop() if len(new) == 1 else "")' "$before" "$(nested_niri_windows)")
        [ -n "$WIN" ] && break
        sleep 0.2
    done
    [ -n "$WIN" ] || die "the nested niri's window never appeared on this niri"
    echo "nested niri: window $WIN, sockets in $RT"

    # Floating, so its size is exactly what is set; on the last workspace of
    # its monitor, which niri always keeps empty; and without the focus:
    # --focus false leaves the focus on the workspace you are on.
    niri msg action move-window-to-floating --id "$WIN" >/dev/null ||
        die "niri would not float the nested niri's window (move-window-to-floating)"
    niri msg action set-window-width --id "$WIN" "$NESTED_W" >/dev/null ||
        die "niri would not set the nested niri's window width (set-window-width)"
    niri msg action set-window-height --id "$WIN" "$NESTED_H" >/dev/null ||
        die "niri would not set the nested niri's window height (set-window-height)"
    spare=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next(w for w in get("windows") if w["id"] == int(sys.argv[1]))
spaces = get("workspaces")
output = next(s["output"] for s in spaces if s["id"] == win["workspace_id"])
print(output, max(s["idx"] for s in spaces if s["output"] == output))' "$WIN") ||
        die "cannot find a spare workspace for the nested niri"
    read -r opened_on spare <<<"$spare"
    niri msg action move-window-to-workspace --window-id "$WIN" --focus false "$spare" >/dev/null ||
        die "niri would not park the nested niri's window on workspace $spare (move-window-to-workspace)"
    # The index is resolved against the focused output, which may not be the
    # one the window opened on: check it landed on that output's last
    # workspace, alone.
    landed=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next(w for w in get("windows") if w["id"] == int(sys.argv[1]))
space = next(s for s in get("workspaces") if s["id"] == win["workspace_id"])
others = [w for w in get("windows") if w["workspace_id"] == space["id"] and w["id"] != win["id"]]
print(space["output"] == sys.argv[2] and space["idx"] == int(sys.argv[3]) and not others)' \
        "$WIN" "$opened_on" "$spare" 2>/dev/null) ||
        die "cannot ask this niri where the nested window was parked"
    [ "$landed" = True ] ||
        die "the nested niri's window was not parked alone on workspace $spare of $opened_on: it may have gone to another monitor"

    for _ in $(seq 1 25); do
        [ -z "$(focus_reason)" ] && break
        sleep 0.2
    done
    why=$(focus_reason)
    [ -z "$why" ] || die "could not park the nested niri: $why"

    for _ in $(seq 1 25); do
        size=$(nested niri msg -j outputs 2>/dev/null | python3 -c '
import json, sys
o = next(iter(json.load(sys.stdin).values()))["logical"]
print(o["width"], o["height"])' 2>/dev/null)
        [ "$size" = "$NESTED_W $NESTED_H" ] && break
        sleep 0.2
    done
    [ "$size" = "$NESTED_W $NESTED_H" ] ||
        die "the nested niri's output is ${size:-unknown}, not ${NESTED_W}x${NESTED_H}"
    ok "the nested niri is parked unfocused on a spare workspace, ${NESTED_W}x${NESTED_H}"

    watch_focus &
    WATCH=$!
}

nested_daemon_start() {  # log file
    "${NENV[@]}" "$NIRITASKS" daemon >"$1" 2>&1 &
    DAEMON=$!
    sleep 3
    kill -0 "$DAEMON" 2>/dev/null || die "the daemon exited at once:
$(tail -n 20 "$1")"
}

nested_daemon_stop() {
    kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
    DAEMON=""
}

nested_service_untouched() {
    guard
    local now
    now=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)
    if [ "$now" = "$SERVICE_PID" ]; then
        ok "niri-tasks.service was left alone (main pid ${now})"
    else
        bad "niri-tasks.service changed under the run: main pid ${SERVICE_PID} before, ${now} after"
    fi
}
```

Before writing, diff this against what is in `tests/e2e-panel.sh` today. The python snippets, the park sequence and the focus watch must be the same logic, moved. The only intended changes are:
- the `repeat-rate` line and its comment;
- the generic `guard` wording ("this run" instead of "its frames");
- the `nested`, `nested_daemon_*` and `nested_service_untouched` functions;
- `NESTED_W`/`NESTED_H` for `OUT_W`/`OUT_H`;
- the `python3` prerequisite;
- the KEEP message.

If today's file has anything this block lacks, keep it and say so in your report.

- [ ] **Step 2: Cut the harness out of `tests/e2e-panel.sh`**

Edit `tests/e2e-panel.sh` so that:

1. **Prerequisites.** Replace the block from `command -v niri >/dev/null || …` down to and including `NIRITASKS="${NIRITASKS:-niritasks}"` with:

```bash
python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

. "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"
```

2. **Constants.** In the constants block, `OUT_W=1600` becomes `OUT_W=$NESTED_W` and `OUT_H=1000` becomes `OUT_H=$NESTED_H`. Change its comment to `# The nested niri's one output (tests/lib/nested-niri.sh), in pixels.` Everything else in that block stays as it is.

3. **Remove the moved code.** Delete everything from `SB=""; RT=""` down to the end of the `guard() { … }` function: the sandbox, cleanup, traps, `ok`…`summary`, the "a niri of its own" and "park it out of the way" sections, `focus_reason`, the park checks, `watch_focus` and `guard`. Add, in its place:

```bash
mkdir -p "$SB/shots"
```

4. **Start the nested niri.** Directly before the `TAG=$(…)` line, insert:

```bash
nested_start
```

5. **First daemon.** Replace the first daemon start, from `"${NENV[@]}" "$NIRITASKS" daemon >"$SB/daemon.err" 2>&1 &` through its `kill -0 … || die …` lines, with:

```bash
nested_daemon_start "$SB/daemon.err"
```

6. **Cold-start restart.** Replace the restart, from `kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null` through the `kill -0 "$DAEMON" … die "the restarted daemon exited at once: …"` lines, with:

```bash
nested_daemon_stop
sleep 1
nested_daemon_start "$SB/daemon2.err"
```

7. **Service check.** Replace the body of the `# ─── the real daemon, untouched ───` section (from `guard` to the closing `fi`) with:

```bash
nested_service_untouched
```

8. **Header comment.** Where it starts "So this measures pixels, on a screen of its own.", add one sentence saying the nested niri comes from `tests/lib/nested-niri.sh`, which `e2e-box.sh` shares.

Then check that the panel-only code stayed: `add`, `settle`, `loads`, `same`, `shot`, `frame_size`, `measure`, and every check section. Run `grep -n "NENV\|OUT_W\|OUT_H" tests/e2e-panel.sh`. The remaining `"${NENV[@]}"` uses (screenshot, `tag`, `task panel`, wtype) are fine. `NENV` is set by `nested_start` before any of them run.

- [ ] **Step 3: Run it**

```bash
bash -n tests/lib/nested-niri.sh tests/e2e-panel.sh
cargo build --release
NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh
```

Expected: `passed: 13   failed: 0   skipped: 0`, exit 0, with the same numbers as before: `columns 1566-1600` for the peek and cold start, and `columns 420-1180` for the keyboard's cards. Run it twice. Afterwards check:

```bash
niri msg -j windows | python3 -c 'import json,sys; print([w["id"] for w in json.load(sys.stdin) if w["app_id"]=="niri"])'
ls -d /tmp/nte2e.* 2>/dev/null
systemctl --user is-active niri-tasks.service
```

Expected: `[]`, nothing, `active`.

- [ ] **Step 4: Commit**

```bash
git add tests/lib/nested-niri.sh tests/e2e-panel.sh
git commit -m "Move the e2e tests' nested niri into tests/lib, with key repeat off

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Run tests/e2e-box.sh inside the nested niri

**Files:**
- Modify: `tests/e2e-box.sh` (top and bottom rewritten; `run_suite` body converted mechanically)

**Interfaces:**
- Consumes (Task 5, `tests/lib/nested-niri.sh`): `SB`, `NIRITASKS`, `TASKDATA`, `ok`, `bad`, `summary`, `fail`, `nested`, `NENV`, `guard`, `nested_start`, `nested_daemon_start`, `nested_daemon_stop`, `nested_service_untouched`.
- Produces: the finished box test. `run_suite`'s checks and messages are unchanged in meaning.

- [ ] **Step 1: Rewrite the top of the file**

Replace everything from line 1 down to, but not including, `run_suite() {` with the block below. The header's first three paragraphs (down to "…README.md lists them as the manual check.") stay word for word from the current file. Copy them in where the block says so.

```bash
#!/usr/bin/env bash
# End-to-end test of the task box: keypress -> box -> submit -> task written.
#
#   bash tests/e2e-box.sh
#
# [keep the current header's paragraphs from "This is not a cargo test" down
#  to "…README.md lists them as the manual check." word for word]
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
. "$(dirname "${BASH_SOURCE[0]}")/lib/nested-niri.sh"

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
    "${NENV[@]}" "$NIRITASKS" task "$@" >/dev/null 2>&1 &
    for _ in $(seq 1 40); do
        [ -n "$(box_id)" ] && { sleep 0.6; return 0; }
        sleep 0.1
    done
    return 1
}
```

Then copy these helpers **unchanged** from the current file, in their current order: `uuid_of`, `notes_of`, `stamped_notes_of`, `undo_count`, `field_of`, `pending`. They call only `task`, which reads the sandbox through the exported `TASKRC`/`TASKDATA`.

Then add:

```bash
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
```

- [ ] **Step 2: Convert `run_suite` mechanically**

Inside `run_suite() { … }` only, every key press and every `niritasks` call must go to the nested niri:
- each `wtype ` becomes `nested wtype `;
- each `"$NIRITASKS" task` becomes `nested "$NIRITASKS" task`.

Do it with this script, which touches only the lines between `run_suite() {` and the next line that is exactly `}`:

```bash
python3 - tests/e2e-box.sh <<'PY'
import re, sys
p = sys.argv[1]
lines = open(p).read().split("\n")
start = lines.index("run_suite() {")
end = next(i for i in range(start + 1, len(lines)) if lines[i] == "}")
for i in range(start, end):
    lines[i] = re.sub(r'(?<![\w-])wtype ', 'nested wtype ', lines[i])
    lines[i] = lines[i].replace('"$NIRITASKS" task', 'nested "$NIRITASKS" task')
open(p, "w").write("\n".join(lines))
PY
```

Add a `guard` call as the last line of `run_suite`, just after its `close_any_box`, so each suite ends by checking that nothing reached the nested niri from outside.

- [ ] **Step 3: Rewrite the bottom of the file**

Replace everything after `run_suite`'s closing `}` with:

```bash
nested_start

# Both paths matter: the daemon serves the box when it is running, and the CLI
# builds its own when it is not. The fallback is the reason this tool does not
# depend on a daemon, so it is tested rather than assumed. The box looks for
# a daemon under the nested niri's runtime dir, so the second half needs only
# none running there — yours keeps running throughout.
nested_daemon_start "$SB/daemon.err"
run_suite "served by the daemon"
nested_daemon_stop
sleep 1

run_suite "fallback, no daemon running"

nested_service_untouched
summary
[ "$fail" -eq 0 ]
```

The old pre-run cleanup is gone with it: the `close_any_box` and `pkill -f "$NIRITASKS task (add|edit|note)"` before the suites, and the `systemctl` stop and start. A fresh nested niri has no stale box to close. That `pkill` matched the user's own boxes on the real desktop.

- [ ] **Step 4: Check nothing still reaches the real desktop**

```bash
bash -n tests/e2e-box.sh
grep -nE '(^|[^-a-z])wtype ' tests/e2e-box.sh | grep -v 'nested wtype\|command -v wtype\|sudo apt'
grep -n 'niri msg' tests/e2e-box.sh | grep -v 'nested niri msg'
grep -n '"\$NIRITASKS"' tests/e2e-box.sh | grep -v 'nested "\$NIRITASKS"\|NENV'
grep -n 'systemctl\|pkill' tests/e2e-box.sh
```

Expected: `bash -n` is silent and all four greps print nothing. A comment line that only *mentions* `wtype` is fine; read every hit.

- [ ] **Step 5: Run it, three times**

Note your desktop's focused window and the service pid first:

```bash
niri msg -j focused-window | python3 -c 'import json,sys; w=json.load(sys.stdin) or {}; print(w.get("id"), w.get("app_id"))'
systemctl --user show -p MainPID --value niri-tasks.service
```

Then:

```bash
time NIRITASKS=./target/release/niritasks bash tests/e2e-box.sh
```

Expected:
- `nested niri: window …` and the parked PASS line.
- Both suites, `=== served by the daemon ===` and `=== fallback, no daemon running ===`, with every check PASS. In particular `Ctrl+Enter wrote the task intact` must pass: with doubled letters it fails.
- `niri-tasks.service was left alone (main pid <same>)`, `failed: 0`, exit 0.

Afterwards, the desktop's focused window is the same id as before, and the Task 5 leftover checks hold (`[]`, no `/tmp/nte2e.*`, `active`). Run it three times; all must pass. Record the wall time of one run for Task 7.

If a check fails, rerun with `NIRITASKS_E2E_KEEP=1` and read `$SB/daemon.err` and `$SB/niri.log`. Fix the cause, not the check. If `settle_keys` turns out unnecessary, leave it in anyway: it is harmless, and whether it is needed is not this task's question.

- [ ] **Step 6: Prove the focus guard stops it**

Run it in the background with output to a log in your scratch area. Poll the log (a bounded loop) until `=== served by the daemon ===` appears. Then focus the nested window from the parent:

```bash
WIN=$(sed -n 's/^nested niri: window \([0-9]*\),.*/\1/p' "$LOG")
niri msg action focus-window --id "$WIN"
```

Wait for the run to end. Expected: a `FAIL  the nested niri's window has the focus — anything typed there would reach the nested niri, …` line (or the workspace-on-screen variant), the summary, and exit 1. Then repeat the leftover checks, and make sure focus is back on a desktop window.

- [ ] **Step 7: Commit**

```bash
git add tests/e2e-box.sh
git commit -m "Type the task box's e2e keys into the nested niri, not the desktop

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: README and tests/all.sh: neither e2e test takes the machine over

**Files:**
- Modify: `README.md` ("Running the tests" and "Why the three scripts are not cargo tests")
- Modify: `tests/all.sh` (header comment, `why_box`, warnings)

**Interfaces:**
- Consumes: the box test's wall time from Task 6 Step 5.

- [ ] **Step 1: README — command block**

Change the `e2e-box.sh` line to say where it runs and how long it takes now. Round the Task 6 wall time to 5s, and keep the column alignment:

```
bash tests/e2e-box.sh       # the task box, in a nested niri         ~NNs
```

Change `tests/all.sh`'s `~100s` by the box test's difference from `~65s`.

- [ ] **Step 2: README — the all.sh paragraph and table**

In the paragraph about `tests/all.sh`, replace "It warns you before the two that take the machine over." with "Neither e2e suite takes the machine over: both run in a nested niri parked on a spare workspace, and it tells you which one to keep off."

Replace the `tests/e2e-box.sh` table row with:

```markdown
| `tests/e2e-box.sh` | `wtype`, niri, a Wayland session, taskwarrior | A nested niri window for the run, parked like the panel test's; keep off that workspace until it finishes. Its keys go only to that nested niri. Not your keyboard or the `niri-tasks` daemon |
```

- [ ] **Step 3: README — the e2e-box.sh paragraph**

Replace the paragraph that begins "Two things about `e2e-box.sh` in particular." and ends "…it means the task panel blinks out for a minute." with:

```markdown
`e2e-box.sh` runs in the same kind of nested niri (`tests/lib/nested-niri.sh`
starts it for both). Its keys are pressed with `wtype` pointed at that nested
niri, so they never reach your windows, and you can keep working while it
runs — keep off the workspace it is parked on, as for the panel test. It
proves both ways the box opens: served by a daemon running inside the nested
niri, then with none running there, when the CLI builds the box itself. Your
own `niri-tasks` daemon is never stopped.
```

- [ ] **Step 4: README — "Why the three scripts are not cargo tests"**

In the `tests/e2e-box.sh` paragraph, replace "It runs the whole thing twice, once served by the daemon and once with the daemon stopped, because the fallback is the reason this tool does not depend on a daemon." with "It runs the whole thing twice, inside a nested niri of its own, once served by a daemon there and once with none, because the fallback is the reason this tool does not depend on a daemon."

- [ ] **Step 5: tests/all.sh**

Header — replace the paragraph that starts "They run in order of how much they take over the machine:" with:

```bash
# They run cheapest first: cargo test touches nothing, e2e-tag.sh at most
# names a spare workspace, and the panel and box tests each run in a nested
# niri of their own (tests/lib/nested-niri.sh), parked on the last workspace
# of your monitor. Neither presses a key or takes a screenshot on your
# desktop, so you can keep working while they run — just keep off that
# workspace.
```

`why_box` — make it mirror what `e2e-box.sh` and the harness check:

```bash
why_box() {
    command -v "$NIRITASKS" >/dev/null || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS="; return; }
    command -v wtype >/dev/null || { echo "no wtype (sudo apt install wtype)"; return; }
    command -v niri  >/dev/null || { echo "no niri"; return; }
    command -v task  >/dev/null || { echo "no taskwarrior on \$PATH"; return; }
    [ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)"; return; }
    [ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display"; return; }
}
```

Warnings — replace the two warning blocks around the panel and box suites with one block before the panel suite:

```bash
if [ -z "$(why_panel)" ] || [ -z "$(why_box)" ]; then
    echo
    echo "  ! the panel and box tests park a nested niri on the last workspace"
    echo "    of this monitor: keep off that workspace until they finish."
fi
suite "tests/e2e-panel.sh" "$(why_panel)" bash tests/e2e-panel.sh
suite "tests/e2e-box.sh"     "$(why_box)"     bash tests/e2e-box.sh
```

- [ ] **Step 6: Check, and now run everything**

```bash
grep -n "whatever has focus\|leave the keyboard\|Leave the keyboard\|blinks out\|daemon stopped\|take the machine over\|takes the machine over" README.md tests/all.sh tests/e2e-box.sh tests/e2e-panel.sh
bash -n tests/all.sh
NIRITASKS=./target/release/niritasks bash tests/all.sh
```

Expected:
- The grep finds no stale claims. Read each hit: "Neither e2e suite takes the machine over" is the new text.
- `all.sh` is now safe to run: both e2e suites run nested. It reports cargo test, e2e-tag, e2e-panel and e2e-box all PASS, and exits 0.
- Your desktop's focused window is unchanged afterwards, and the leftover checks hold.

- [ ] **Step 7: Commit**

```bash
git add README.md tests/all.sh
git commit -m "Describe the box's e2e test running in the nested niri too

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
