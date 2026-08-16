#!/usr/bin/env bash
# End-to-end test of `wt tag --session`: tmux session -> workspace -> tag.
#
#   bash tests/e2e-tag.sh
#
# Not a cargo test, and cannot be, for the same reason as tests/e2e-box.sh: it
# needs a running niri to ask for workspaces and a running tmux server to make
# sessions in. `cargo test` covers the pure halves — session_base and
# workspace_for_session — and they were all that was covered before this file.
# The parts only a live system exercises are the two ends: whether a command
# inside a session can find out which session it is in, and whether the name it
# finds still matches a workspace niri will admit to having.
#
# What it is really pinning is the property the flag exists for: the tag comes
# from the terminal, not from the focus. That is invisible to a unit test —
# both halves can be right while the command still answers "whatever workspace
# you are looking at", which is the bug the flag was added to avoid.
#
# It makes tmux sessions named after real workspaces, always with a suffix well
# clear of the ones you have open (_91 and up), and kills only those. Nothing
# here writes to the task database at all.
set -uo pipefail

command -v tmux >/dev/null || { echo "tmux is required" >&2; exit 1; }
command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }

WT="${WT:-wt}"

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
if [ -z "$FOCUSED" ] || [ -z "$OTHER" ]; then
    echo "need two named workspaces (one focused, one not) to test against" >&2
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
# The tmux session name for a workspace, mirroring src/session.rs: per
# character, not run-collapsing, case kept.
session_of() {
    python3 -c "
import re,sys
print(re.sub(r'[^A-Za-z0-9_-]', '_', sys.argv[1]))
" "$1"
}

# Run `wt tag --session` inside a throwaway session of the given name, and put
# its output in TAG_OUT and its exit status in TAG_RC.
#
# '=' before the name is tmux's exact-match syntax. Without it a target can
# prefix-match, and these names sit next to the real sessions of the same
# workspaces — killing one of those would close whatever you had running in it.
tag_in_session() {
    local name="$1" out rc
    out=$(mktemp); rc=$(mktemp)
    tmux kill-session -t "=$name" 2>/dev/null
    tmux new-session -d -s "$name" "$WT tag --session >$out 2>&1; echo \$? >$rc"
    for _ in $(seq 1 60); do [ -s "$rc" ] && break; sleep 0.1; done
    tmux kill-session -t "=$name" 2>/dev/null
    TAG_OUT=$(cat "$out"); TAG_RC=$(cat "$rc" 2>/dev/null || echo 99)
    rm -f "$out" "$rc"
}

echo "focused workspace: $FOCUSED   other: $OTHER"
expected=$(tag_of "$OTHER")
base=$(session_of "$OTHER")

# ─── the property the flag exists for ─────────────────────────────────────────
# A session named after the workspace that is *not* focused must answer with
# that workspace, while the focus-derived answer stays on the focused one.
tag_in_session "${base}_91"
if [ "$TAG_RC" -eq 0 ] && [ "$TAG_OUT" = "$expected" ]; then
    ok "--session answers with the session's workspace ($TAG_OUT)"
else
    bad "expected '$expected' (rc 0), got '$TAG_OUT' (rc $TAG_RC)"
fi

focused_tag=$("$WT" tag 2>/dev/null)
if [ "$focused_tag" = "$(tag_of "$FOCUSED")" ]; then
    ok "bare wt tag still answers with the focused workspace ($focused_tag)"
else
    bad "bare wt tag gave '$focused_tag', expected '$(tag_of "$FOCUSED")'"
fi
[ "$TAG_OUT" != "$focused_tag" ] \
    && ok "the two disagree, which is the whole point of the flag" \
    || bad "both answers were '$TAG_OUT' — this test proves nothing as set up"

# ─── the suffix is stripped, whatever it is ───────────────────────────────────
tag_in_session "${base}_4242"
[ "$TAG_OUT" = "$expected" ] && ok "a multi-digit session suffix is stripped" \
    || bad "suffix _4242: expected '$expected', got '$TAG_OUT'"

tag_in_session "$base"
[ "$TAG_OUT" = "$expected" ] && ok "a session with no numeric suffix resolves too" \
    || bad "no suffix: expected '$expected', got '$TAG_OUT'"

# ─── the failure cases, which must fail rather than guess ─────────────────────
tag_in_session "zzz-no-such-workspace_91"
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

# Outside tmux there is no terminal to take a workspace from. env -u is what
# makes this honest: the check is on $TMUX, and this test runs inside one.
outside=$(env -u TMUX "$WT" tag --session 2>&1); rc=$?
if [ "$rc" -ne 0 ]; then
    ok "outside tmux it exits non-zero rather than falling back to focus"
    case "$outside" in
        *tmux*) ok "and says tmux is why" ;;
        *)      bad "message does not mention tmux: $outside" ;;
    esac
else
    bad "outside tmux it answered '$outside' — a focus fallback the skill would trust"
fi

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
