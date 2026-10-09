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
# plain Alt, since your own niri takes any Super combination before the
# nested window sees it. A daemon built from this checkout serves it. This
# terminal becomes the try shell, whose niritasks, task and niri msg reach
# the nested niri and the copy:
#
#   reload   rebuild, and restart the nested daemon on the new binary
#   exit     close the nested niri and throw the copy away
#
# Notifications are printed in the try shell rather than shown on your
# desktop. Nothing is written back to your task database. Terminals, editors
# and clones are not opened from a try window either: ghostty and VS Code
# hand off over the session bus to your running instances, and gh would
# clone into your real ~/Projects, so stubs for ghostty, code and gh print
# what would have run, as [notify] lines in the try shell. Mod+Alt+W can
# still make a new folder under ~/Projects.
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
# Stubs for what would reach your real desktop or ~/Projects from a bind:
# they record the call in the notification log and run nothing.
for prog in ghostty code gh; do
    cat > "$SB/bin/$prog" <<'STUB'
#!/bin/sh
echo "would run: ${0##*/} $*" >> "${NOTIFY_LOG:?}"
exit 0
STUB
    chmod +x "$SB/bin/$prog"
done
# The Ideas tab saves under XDG_DATA_HOME: the sandbox's, never yours. Set
# before the nested niri starts, so its bind spawns get it too.
export XDG_DATA_HOME="$SB/share"
NIRITASKS="$SB/bin/niritasks"
# The binds' spawns get no herdr, as in the e2e tests.
NESTED_SPAWN_PATH="$SB/bin:/usr/bin:/bin"
NESTED_WORKSPACE="$PROJECT"
# Alt as Mod, niri's own default for a nested niri: Right Alt as Mod did not
# reach the binds. Your niri binds no Alt combination without Super, bar
# Alt+Print and Ctrl+Alt+Delete, so these reach the nested window.
NESTED_EXTRA_KDL="include \"$ROOT/niri/niri-tasks.kdl\""
nested_start --focused

# Notifications, as they come, in this terminal.
touch "$SB/notifications"
tail -n0 -F --pid=$$ "$SB/notifications" 2>/dev/null | sed -u 's/^/[notify] /' &

cat > "$SB/try.bashrc" <<'RC'
# The try shell's rc, written by try.sh. Not ~/.bashrc, which could put
# herdr back on PATH.
# Its own history: the try shell's commands stay out of ~/.bash_history.
HISTFILE="$TRY_SB/history"
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
checkout's $PROFILE build. Its binds are niri/niri-tasks.kdl's with plain
Alt as Mod: Alt+T adds a task, Alt+Shift+T adds and refines, Alt+Ctrl+T
shows the panel, Alt+W opens a project and Alt+Return opens a terminal
(stubbed in a try window). Here, niritasks, task and niri msg reach it.

Terminals, editors and clones are not opened from it: ghostty, code and gh
are stubs, and what would have run is printed here as [notify] lines.
Mod+Alt+W can still make a new folder under ~/Projects.

  reload   rebuild and restart the nested daemon
  exit     close it all; the copy is thrown away

Daemon log: $SB/daemon.log
EOF
nested env "${TRY_ENV[@]}" bash --rcfile "$SB/try.bashrc" -i
