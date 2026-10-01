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
# hooks.location points at a directory that does not exist: left alone,
# taskwarrior would run the real ~/.task/hooks on every sandbox task.
printf 'data.location=%s/data\nhooks.location=%s/hooks\n' "$SB" "$SB" > "$SB/taskrc"
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
nested() {
    [ "${#NENV[@]}" -gt 0 ] || die "nested before nested_start"
    "${NENV[@]}" "$@"
}

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

    # Vblank waits are off for the nested niri alone: parked out of sight, it
    # never hears that a frame was shown, so a buffer swap that waits for
    # vblank stalls it for a second or more, and every key and `niri msg` with
    # it. Mesa's vblank_mode=0 is what was measured to fix that here;
    # __GL_SYNC_TO_VBLANK=0 is set in case it helps on NVIDIA, unverified.
    env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        vblank_mode=0 __GL_SYNC_TO_VBLANK=0 niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
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
    # Notifications: every task added through the box or the CLI runs
    # notify-send (src/notify.rs) over the session bus, which is shared with
    # the real desktop and would pop up there. A stub first on PATH records
    # them in $SB/notifications instead.
    mkdir -p "$SB/bin"
    cat > "$SB/bin/notify-send" <<'STUB'
#!/bin/sh
echo "$*" >> "${NOTIFY_LOG:?}"
exit 0
STUB
    chmod +x "$SB/bin/notify-send"
    # -u DISPLAY and GDK_BACKEND: if the nested niri dies, GTK must not fall
    # back to X11 and open a box on the real desktop. env -u comes first.
    NENV=(env -u DISPLAY XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$n_wayland"
          NIRI_SOCKET="$n_socket" GDK_BACKEND=wayland
          NOTIFY_LOG="$SB/notifications" PATH="$SB/bin:$PATH")

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
    [ "${#NENV[@]}" -gt 0 ] || die "nested_daemon_start before nested_start"
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
