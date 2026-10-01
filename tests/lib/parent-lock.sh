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
