#!/usr/bin/env bash
# End-to-end test of `niritasks tag --session`: herdr session or project
# folder -> workspace -> tag.
#
#   bash tests/e2e-tag.sh
#
# Not a cargo test, and cannot be, for the same reason as tests/e2e-box.sh: it
# needs a running niri to ask for workspaces. `cargo test` covers the pure
# halves — session_from_env, workspace_for_session, project_from_cwd — and this
# covers the join: whether the name the binary finds still matches a workspace
# niri will admit to having.
#
# What it is really pinning is the property the flag exists for: the tag comes
# from the terminal, not from the focus. That is invisible to a unit test —
# both halves can be right while the command still answers "whatever workspace
# you are looking at", which is the bug the flag was added to avoid.
#
# It does not start herdr. The session is handed over exactly the way herdr
# hands it to a pane — HERDR_SESSION, with HERDR_SOCKET_PATH as the backup —
# so no session of yours is attached to, created or stopped. That herdr really
# does put HERDR_SESSION in its panes is the one thing only a live pane shows:
#   tr '\0' '\n' </proc/<pane shell pid>/environ | grep HERDR_
# The folder cases run with a throwaway $HOME, so they need no real folder
# either. Nothing here writes to the task database at all.
#
# It needs two named workspaces to show the contrast, and will borrow niri's
# trailing empty workspace as the second one when there is only ever a single
# project open — naming it without focusing it, and unnaming it on the way out.
set -uo pipefail

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }

NIRITASKS="${NIRITASKS:-niritasks}"
# Absolute, because the folder cases run from other directories and a relative
# NIRITASKS=./target/debug/niritasks would stop resolving there.
NIRITASKS=$(realpath "$(command -v "$NIRITASKS")" 2>/dev/null) \
    || { echo "no $NIRITASKS on \$PATH — build it, or set NIRITASKS=" >&2; exit 1; }

pass=0; fail=0
ok()  { echo "  PASS  $*"; pass=$((pass+1)); }
bad() { echo "  FAIL  $*"; fail=$((fail+1)); }

# The focused workspace, and some other named one to play against. The whole
# point of the flag is that those two give different answers.
read -r FOCUSED OTHER < <(niri msg -j workspaces | python3 -c "
import json,sys
ws=[w for w in json.load(sys.stdin) if w.get('name')]
focused=next((w['name'] for w in ws if w['is_focused']), '')
other=next((w['name'] for w in ws if not w['is_focused']), '')
print(focused, other)
")

# A machine with one project open has one named workspace, and the contrast this
# whole file is about needs two. Rather than refuse to run there, borrow the
# empty workspace niri always keeps at the end of the output: name it, use it,
# and take the name off again on the way out. It is named without being focused
# — moving focus would change the very thing under test.
SCRATCH=""
FAKE_HOME=""
cleanup() {
    [ -n "$SCRATCH" ] && niri msg action unset-workspace-name "$SCRATCH" >/dev/null 2>&1
    [ -n "$FAKE_HOME" ] && rm -rf "$FAKE_HOME"
}
# INT and TERM as well as EXIT: interrupting a run must not leave a workspace
# named after this test sitting in the switcher.
trap cleanup EXIT INT TERM

# NIRITASKS_E2E_SCRATCH=1 takes this path even when a second named workspace exists, so
# the fallback is testable on a machine that does not need it. A branch nobody
# can reach is a branch nobody has run.
if [ -z "$OTHER" ] || [ -n "${NIRITASKS_E2E_SCRATCH:-}" ]; then
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
        SCRATCH="niritasks-e2e-scratch"
        if niri msg action set-workspace-name --workspace "$scratch_idx" "$SCRATCH" >/dev/null 2>&1; then
            OTHER="$SCRATCH"
            echo "no second named workspace to hand — named workspace $scratch_idx" \
                 "'$SCRATCH' for the run, and will unname it afterwards"
        else
            SCRATCH=""
        fi
    fi
fi

if [ -z "$FOCUSED" ]; then
    echo "the focused workspace has no name — name it with Mod+Alt+Ctrl+W first" >&2
    exit 1
fi
if [ -z "$OTHER" ]; then
    echo "need a second workspace to test against, and no empty one was free to borrow" >&2
    exit 1
fi

# The tag rule, mirroring src/tag.rs on purpose: an expectation computed the
# same way by the same code would agree with any bug the code has.
tag_of() {
    python3 -c "
import re,sys
name = sys.argv[1].lower()
print(re.sub(r'_+\$', '', re.sub(r'^_+', '', re.sub(r'[^a-z0-9_]+', '_', name))))
" "$1"
}
# The herdr session name for a workspace, mirroring src/session.rs: per
# character, not run-collapsing, case kept, cut to 64, reserved names prefixed.
session_of() {
    python3 -c "
import re,sys
s = re.sub(r'[^A-Za-z0-9._-]', '_', sys.argv[1])[:64]
print('ws-' + s if s in ('', '.', '..', 'default') else s)
" "$1"
}

# Run `niritasks tag --session` with a clean slate of herdr variables plus the
# given assignments, from the current directory. Output in TAG_OUT, status in
# TAG_RC. The -u's matter: this test may itself be running inside a herdr pane,
# and an inherited HERDR_SESSION would answer every case the same way.
tag_with() {
    TAG_OUT=$(env -u HERDR_SESSION -u HERDR_SOCKET_PATH "$@" "$NIRITASKS" tag --session 2>&1)
    TAG_RC=$?
}

echo "focused workspace: $FOCUSED   other: $OTHER"
expected=$(tag_of "$OTHER")
session=$(session_of "$OTHER")

# ─── the property the flag exists for ─────────────────────────────────────────
# A session named after the workspace that is *not* focused must answer with
# that workspace, while the focus-derived answer stays on the focused one.
tag_with HERDR_SESSION="$session"
if [ "$TAG_RC" -eq 0 ] && [ "$TAG_OUT" = "$expected" ]; then
    ok "--session answers with the herdr session's workspace ($TAG_OUT)"
else
    bad "expected '$expected' (rc 0), got '$TAG_OUT' (rc $TAG_RC)"
fi

focused_tag=$("$NIRITASKS" tag 2>/dev/null)
if [ "$focused_tag" = "$(tag_of "$FOCUSED")" ]; then
    ok "bare niritasks tag still answers with the focused workspace ($focused_tag)"
else
    bad "bare niritasks tag gave '$focused_tag', expected '$(tag_of "$FOCUSED")'"
fi
[ "$TAG_OUT" != "$focused_tag" ] \
    && ok "the two disagree, which is the whole point of the flag" \
    || bad "both answers were '$TAG_OUT' — this test proves nothing as set up"

# ─── the socket path is the backup ────────────────────────────────────────────
tag_with HERDR_SOCKET_PATH="/nonexistent/herdr/sessions/$session/herdr.sock"
[ "$TAG_OUT" = "$expected" ] && ok "without HERDR_SESSION, the socket path names the session" \
    || bad "socket path: expected '$expected', got '$TAG_OUT' (rc $TAG_RC)"

# ─── the project folder, when there is no named session ───────────────────────
FAKE_HOME=$(mktemp -d)
mkdir -p "$FAKE_HOME/Projects/$OTHER/src"
# The default herdr session has no name, so it falls through to the folder.
pushd "$FAKE_HOME/Projects/$OTHER/src" >/dev/null
tag_with HOME="$FAKE_HOME" HERDR_SOCKET_PATH="/nonexistent/herdr/herdr.sock"
popd >/dev/null
[ "$TAG_OUT" = "$expected" ] && ok "in ~/Projects/<workspace>/…, the folder names the workspace" \
    || bad "folder: expected '$expected', got '$TAG_OUT' (rc $TAG_RC)"

# ─── the failure cases, which must fail rather than guess ─────────────────────
tag_with HERDR_SESSION="zzz-no-such-workspace"
if [ "$TAG_RC" -ne 0 ]; then
    ok "a session matching no workspace exits non-zero"
    case "$TAG_OUT" in
        *"does not match any named workspace"*)
            ok "and says why, naming the session" ;;
        *)  bad "message does not explain: $TAG_OUT" ;;
    esac
else
    bad "a session matching no workspace was resolved anyway: '$TAG_OUT'"
fi

# A renamed workspace must not be rescued by the folder: that answer would look
# right and be wrong.
pushd "$FAKE_HOME/Projects/$OTHER" >/dev/null
tag_with HOME="$FAKE_HOME" HERDR_SESSION="zzz-no-such-workspace"
popd >/dev/null
case "$TAG_RC:$TAG_OUT" in
    0:*) bad "an unmatched session was rescued by the folder: '$TAG_OUT'" ;;
    *"does not match any named workspace"*)
         ok "an unmatched session does not fall back to the folder" ;;
    *)   bad "an unmatched session failed for the wrong reason: $TAG_OUT" ;;
esac

# Neither a named session nor a project folder: nothing to take a workspace
# from, so it must say so rather than fall back to focus.
outside=$(cd / && env -u HERDR_SESSION -u HERDR_SOCKET_PATH "$NIRITASKS" tag --session 2>&1); rc=$?
if [ "$rc" -ne 0 ]; then
    ok "outside herdr and ~/Projects it exits non-zero rather than falling back to focus"
    case "$outside" in
        *"herdr session"*) ok "and says why" ;;
        *)                 bad "message does not explain: $outside" ;;
    esac
else
    bad "outside herdr and ~/Projects it answered '$outside' — a focus fallback the skill would trust"
fi

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
