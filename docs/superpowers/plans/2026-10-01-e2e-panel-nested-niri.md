# Task panel e2e test in its own nested niri — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tests/e2e-panel.sh` runs its own daemon inside a nested niri it starts and owns, so nothing on the real desktop can change what it measures, and the real `niri-tasks.service` keeps running.

**Architecture:** The script writes a niri config into its sandbox, starts `niri -c` nested (winit backend, a window on the user's niri) with its own short `XDG_RUNTIME_DIR`, reads the nested `NIRI_SOCKET`/`WAYLAND_DISPLAY` from a startup command's `env` dump, and parks that window floating at 1600x1000 on a spare workspace without focus. Everything under test — daemon, `niritasks task panel`, `wtype`, screenshots — runs with the nested environment; only parking and the focus watch talk to the parent niri. Frames are shot until two in a row match, then compared exactly: the nested screen is a flat colour, so expected columns are exact numbers and "unchanged" means pixel-identical.

**Tech Stack:** bash, niri 26.04 (`niri msg`), python3 + Pillow, taskwarrior, wtype.

**Spec:** Taskwarrior task `9c875e20` — read it with `task rc.json.array=on 9c875e20 export`; its description and annotations are the spec. Its "Decided", "Steps", "Done when" and "Out of scope" annotations are binding.

## Global Constraints

- The test owns its whole screen: nested niri, its own daemon, sandbox `TASKDATA` and runtime dir. It never stops, starts or restarts `niri-tasks.service`, and never touches `~/Pictures/Screenshots` or the parent's clipboard.
- The nested runtime dir is short (`mktemp -d /tmp/nte2e.XXXXXX`): Unix socket paths are cut at ~108 chars, and both niri's sockets and the daemon's `niri-tasks.sock` (`src/ipc.rs::socket_path`, under `$XDG_RUNTIME_DIR`) land there.
- The nested niri is launched with `WAYLAND_DISPLAY` set to the parent display's **absolute** path, because `XDG_RUNTIME_DIR` changes underneath it.
- Park the nested window floating at a fixed 1600x1000 on a spare workspace with `--focus false`, and **fail the run, saying why,** if the parent ever reports that window focused or its workspace active.
- Shoot until two consecutive frames match before measuring.
- Use exact expected numbers from the fixed output where the prototype showed exact results: peek starts at x 1566 of 1600; centred cards span x 420–1180 (between the Down-key frame pair); the Escape frame is identical to the tucked frame.
- Drop what only existed for the real desktop: the stillness gate, centre SKIPs, the notification column and box crops, `clear_of_edge`/`output_scale`, the screenshot-directory scraping, the "this workspace has no name" check.
- Keep every check's meaning: peek, stacking, other tag hidden, keyboard centres + buttons, Escape back, empty hides, cold start.
- Leave nothing behind: no window, workspace, socket or temp dir (`NIRITASKS_E2E_KEEP=1` still keeps the frames, and says where).
- Out of scope: the hover, blur, `e2e-box.sh`, `e2e-tag.sh`, any change to the panel itself (`src/`).
- Comments and output follow the existing script's voice: plain sentences saying *why*, no jargon headers.

## Facts the executor needs (from the prototype, 2026-10-01)

- `niri -c <config>` with **no** `--session` runs nested on the winit backend; its window has `app_id` `"niri"` in the parent's `niri msg -j windows`.
- Config lines that matter: `hotkey-overlay { skip-at-startup; }`, `animations { off; }`, `layout { background-color "#406080"; }`, `workspace "e2e"` (so `niritasks tag` inside it prints `e2e`).
- `niri msg action screenshot-screen --show-pointer false --path <file>` writes straight to `<file>`: no screenshot dir, no notification.
- `niri msg outputs` in the nested niri says the output is "flipped vertically"; screenshots are upright regardless. Ignore it.
- Parked off screen, the nested niri renders only when asked: the first screenshot after a change can show the panel mid-slide, the next shows it settled. That is why `shot` repeats until two frames match.
- When the user switches to the parked workspace the nested window gets focused once; keys typed then reach the panel. The focus watch exists for this.
- Parent output scale on this machine is 1.0. The nested config pins `output "winit" { scale 1; }` and the test checks the screenshot is exactly 1600x1000, so a different setup fails loudly rather than measuring wrong numbers.
- Panel constants these numbers come from: `PEEK_PX = 30`, `RING_PX = 4`, `SURFACE_WIDTH = 16 + 760 + 8 = 784` (`src/panel/surface.rs`), `CARD_WIDTH_PX = 760` (`src/panel/style.rs`). Tucked peek starts at `1600 - 30 - 4 = 1566` (the card's outline ring is drawn 4px outside it). Centred cards start at `(1600 - 760) / 2 = 420`. The centred surface spans `(1600 - 784)/2 = 408` to `1192`.
- If a measured exact number differs from the plan's on the first run, **do not loosen it to a tolerance**. Rerun with `NIRITASKS_E2E_KEEP=1`, open the kept frames, and work out from the constants above why; change the expectation only with that reason written in the comment, and report it.

## Running the script while developing

Always against the build in this worktree, never the installed binary:

```bash
cargo build --release
NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh
```

While it runs, a niri window appears briefly on the current workspace and is moved to the last (empty) workspace of that monitor. Don't switch to that workspace.

---

### Task 1: Nested niri setup, parking, focus watch and teardown

Rewrite `tests/e2e-panel.sh` from scratch: prerequisites, sandbox, nested niri launch, env capture, park, focus watch, daemon, tag check, real-service check, cleanup. No frame checks yet — Tasks 2 and 3 insert them at the marked place.

**Files:**
- Rewrite: `tests/e2e-panel.sh` (whole file)

**Interfaces:**
- Produces (used by Tasks 2 and 3, all defined in this file):
  - `NENV` — bash array; prefix any command with `"${NENV[@]}"` to run it inside the nested niri (daemon, `niri msg`, `wtype`, `niritasks task panel`).
  - `TAG` — the nested workspace's tag, `e2e`.
  - `DAEMON` — pid of the sandbox daemon (killed and restarted by the cold-start check).
  - `OUT_W=1600 OUT_H=1000 PEEK=30 RING=4 CARD=760 SURFACE=784 PEEK_X=1566 CARD_X=420 SURFACE_LEFT=408 SURFACE_RIGHT=1192`.
  - `ok "<msg>"`, `bad "<msg>"`, `skip "<msg>"`, `die "<msg>"` (prints to stderr, exits 1), `summary`.
  - `guard` — exits the run with a FAIL if the focus watch has seen the nested window focused or on screen.
  - `add "<description>"` — adds a task on `+$TAG` to the sandbox.
  - `settle` — `sleep 2`.
  - The marker comment line `# ─── the real daemon, untouched ───` — later tasks insert their sections immediately **above** it.

- [ ] **Step 1: Write the new script**

Replace the whole of `tests/e2e-panel.sh` with:

```bash
#!/usr/bin/env bash
# End-to-end test of the task panel: task state -> pixels on screen.
#
#   bash tests/e2e-panel.sh
#
# Not a cargo test, and cannot be. The panel is a layer surface whose whole
# behaviour is what the compositor puts on screen: whether cards are there, how
# far they poke out, how many there are. The window reports the size it asked
# for, not the size it got, and a layer surface that was never mapped reports
# nothing wrong while showing nothing at all — both have happened before.
#
# So this measures pixels, on a screen of its own. It starts a nested niri — a
# window on yours, with its own runtime dir, sockets and one 1600x1000 output
# of flat colour — and runs its own daemon in it against a sandboxed TASKDATA.
# Nothing on your desktop can reach those frames: not the wallpaper, not a
# terminal redrawing, not a window rule or a notification or the pointer. Your
# own niri-tasks daemon keeps running throughout and never sees these tasks.
#
# Because the screen is flat and fixed, the numbers are exact: the peek starts
# at column 1566, the keyboard's cards span 420-1180, and a frame that should
# not have changed is compared pixel for pixel. A parked nested niri draws only
# when asked, so its first frame after a change can be stale; every frame is
# shot until two in a row match.
#
# The nested window is parked, floating and unfocused, on the last workspace of
# the monitor you start it from. Keep off that workspace while it runs: going
# there focuses the window, and keys typed then would reach the panel. If that
# happens the run fails and says so, rather than measuring frames it can no
# longer trust.
#
# What it cannot check is the hover: nothing can move the pointer, so the slide
# out, the slide back and clicks passing beside the peek are checked by hand
# (README, "Testing"). The keyboard it can: `task panel` moves the panel to the
# middle of the screen, and with wtype, Down and Escape are pressed in the
# nested niri, never on your desktop.
set -uo pipefail

command -v niri >/dev/null || { echo "niri is required" >&2; exit 1; }
command -v task >/dev/null || { echo "taskwarrior is required" >&2; exit 1; }
[ -n "${NIRI_SOCKET:-}" ] || { echo "niri is not running (no \$NIRI_SOCKET)" >&2; exit 1; }
[ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display to open the nested niri on" >&2; exit 1; }
python3 -c "import PIL" 2>/dev/null || {
    echo "python3 Pillow is required: sudo apt install python3-pil" >&2; exit 1; }

NIRITASKS="${NIRITASKS:-niritasks}"

# The nested niri's one output, in pixels: the size its window is parked at.
OUT_W=1600
OUT_H=1000
# Mirror PEEK_PX, RING_PX and SURFACE_WIDTH in src/panel/surface.rs, and
# CARD_WIDTH_PX in src/panel/style.rs.
PEEK=30
RING=4
CARD=760
SURFACE=784
# Where the tucked peek starts: the peek, and the card's outline ring drawn
# outside it.
PEEK_X=$((OUT_W - PEEK - RING))
# Where the keyboard's cards start, in the middle of the screen.
CARD_X=$(((OUT_W - CARD) / 2))
# The centred surface, shadow and all.
SURFACE_LEFT=$(((OUT_W - SURFACE) / 2))
SURFACE_RIGHT=$((SURFACE_LEFT + SURFACE))

SB="$(mktemp -d)"
# Short on purpose: the nested niri's sockets and the daemon's go in here, and
# a Unix socket path longer than ~108 characters cannot be bound.
RT="$(mktemp -d /tmp/nte2e.XXXXXX)"
mkdir -p "$SB/data" "$SB/shots"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

# The nested niri connects to this one by its absolute path, since its own
# XDG_RUNTIME_DIR is somewhere else.
case "$WAYLAND_DISPLAY" in
    /*) PARENT_WAYLAND="$WAYLAND_DISPLAY" ;;
    *) PARENT_WAYLAND="${XDG_RUNTIME_DIR:?}/$WAYLAND_DISPLAY" ;;
esac

# The real daemon is never touched; this is how the end of the run proves it.
SERVICE_PID=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)

NESTED=""; WIN=""; WATCH=""; DAEMON=""
cleanup() {
    [ -n "$WATCH" ] && kill "$WATCH" 2>/dev/null && wait "$WATCH" 2>/dev/null
    [ -n "$DAEMON" ] && kill "$DAEMON" 2>/dev/null && wait "$DAEMON" 2>/dev/null
    # Its window closes with it, and niri drops the workspace it leaves empty.
    [ -n "$NESTED" ] && kill "$NESTED" 2>/dev/null && wait "$NESTED" 2>/dev/null
    rm -rf "$RT"
    if [ -n "${NIRITASKS_E2E_KEEP:-}" ]; then
        echo "frames kept in $SB/shots"
    else
        rm -rf "$SB"
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

# ─── a niri of its own ───────────────────────────────────────────────────────
# Animations off, so niri itself never draws a frame in between; the panel's
# slide is its own and still runs. A flat background, so a frame with nothing
# on it is the same every time. One named workspace, which is the tag the
# panel's tasks are filed under. The startup command is how the test learns
# the nested niri's own sockets.
cat > "$SB/niri.kdl" <<EOF
hotkey-overlay { skip-at-startup; }
animations { off; }
xwayland-satellite { off; }
output "winit" { scale 1; }
layout { background-color "#406080"; }
workspace "e2e"
spawn-sh-at-startup "env > $SB/nested.env.tmp && mv $SB/nested.env.tmp $SB/nested.env"
EOF

# The parent's windows that are already nested niris, so the new one can be
# told apart from them.
niri_windows() {
    niri msg -j windows | python3 -c '
import json, sys
print(" ".join(str(w["id"]) for w in json.load(sys.stdin) if w["app_id"] == "niri"))'
}
before=$(niri_windows) || die "cannot list this niri's windows"

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
N_SOCKET=$(sed -n 's/^NIRI_SOCKET=//p' "$SB/nested.env")
N_WAYLAND=$(sed -n 's/^WAYLAND_DISPLAY=//p' "$SB/nested.env")
[ -n "$N_SOCKET" ] && [ -n "$N_WAYLAND" ] || die "the nested niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"
NENV=(env XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$N_WAYLAND" NIRI_SOCKET="$N_SOCKET")

for _ in $(seq 1 25); do
    WIN=$(python3 -c '
import sys
new = set(sys.argv[2].split()) - set(sys.argv[1].split())
print(new.pop() if len(new) == 1 else "")' "$before" "$(niri_windows)")
    [ -n "$WIN" ] && break
    sleep 0.2
done
[ -n "$WIN" ] || die "the nested niri's window never appeared on this niri"
echo "nested niri: window $WIN, sockets in $RT"

# ─── park it out of the way ──────────────────────────────────────────────────
# Floating, so its size is exactly what is set; on the last workspace of its
# monitor, which niri always keeps empty; and without the focus, which stays
# on the window it opened over.
niri msg action move-window-to-floating --id "$WIN" >/dev/null
niri msg action set-window-width --id "$WIN" "$OUT_W" >/dev/null
niri msg action set-window-height --id "$WIN" "$OUT_H" >/dev/null
spare=$(python3 -c '
import json, subprocess, sys
get = lambda what: json.loads(subprocess.run(["niri", "msg", "-j", what],
                                             capture_output=True, text=True, check=True).stdout)
win = next(w for w in get("windows") if w["id"] == int(sys.argv[1]))
spaces = get("workspaces")
output = next(s["output"] for s in spaces if s["id"] == win["workspace_id"])
print(max(s["idx"] for s in spaces if s["output"] == output))' "$WIN") ||
    die "cannot find a spare workspace for the nested niri"
niri msg action move-window-to-workspace --window-id "$WIN" --focus false "$spare" >/dev/null

# Why the nested window can no longer be trusted, or nothing while it can.
focus_reason() {
    python3 - "$WIN" 2>/dev/null <<'PY'
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

for _ in $(seq 1 25); do
    [ -z "$(focus_reason)" ] && break
    sleep 0.2
done
why=$(focus_reason)
[ -z "$why" ] || die "could not park the nested niri: $why"

for _ in $(seq 1 25); do
    size=$("${NENV[@]}" niri msg -j outputs 2>/dev/null | python3 -c '
import json, sys
o = next(iter(json.load(sys.stdin).values()))["logical"]
print(o["width"], o["height"])' 2>/dev/null)
    [ "$size" = "$OUT_W $OUT_H" ] && break
    sleep 0.2
done
[ "$size" = "$OUT_W $OUT_H" ] || die "the nested niri's output is ${size:-unknown}, not ${OUT_W}x${OUT_H}"
ok "the nested niri is parked unfocused on a spare workspace, ${OUT_W}x${OUT_H}"

# From here on, the window must stay unfocused and off screen. This watches
# for the whole run; guard ends the run the moment it has seen otherwise.
watch_focus() {
    local why
    while sleep 0.25; do
        why=$(focus_reason)
        [ -n "$why" ] && { echo "$why" > "$SB/tampered"; return; }
    done
}
watch_focus &
WATCH=$!
guard() {
    [ -s "$SB/tampered" ] || return 0
    bad "$(cat "$SB/tampered") — anything typed there would reach the panel,
      so its frames can no longer be trusted. Keep off that workspace while
      this runs"
    summary
    exit 1
}

# ─── its own daemon ──────────────────────────────────────────────────────────
add() {  # description
    task rc.verbose=nothing rc.confirmation=no add "+$TAG" -- "$1" >/dev/null 2>&1
}
settle() { sleep 2; }  # the daemon ticks every 700ms

TAG=$("${NENV[@]}" "$NIRITASKS" tag 2>/dev/null)
if [ "$TAG" = e2e ]; then
    ok "the nested niri's workspace files its tasks under e2e"
else
    bad "the nested niri's workspace reads as tag '${TAG}', expected e2e"
    summary; exit 1
fi

"${NENV[@]}" "$NIRITASKS" daemon >"$SB/daemon.err" 2>&1 &
DAEMON=$!
sleep 3
kill -0 "$DAEMON" 2>/dev/null || die "the daemon exited at once:
$(tail -n 20 "$SB/daemon.err")"

# ─── the real daemon, untouched ──────────────────────────────────────────────
guard
now=$(systemctl --user show -p MainPID --value niri-tasks.service 2>/dev/null || echo 0)
if [ "$now" = "$SERVICE_PID" ]; then
    ok "niri-tasks.service was left alone (main pid ${now})"
else
    bad "niri-tasks.service changed under the run: main pid ${SERVICE_PID} before, ${now} after"
fi

summary
[ "$fail" -eq 0 ]
```

Notes for the executor:
- `env ... niri &` and `"${NENV[@]}" "$NIRITASKS" daemon &`: `env` execs, so `$!` is niri's / the daemon's own pid and `kill` reaches it. Do **not** wrap these in a shell function — a backgrounded function is a subshell and `$!` would be that subshell.
- `move-window-to-workspace <idx>` resolves the index on the monitor the window is on, which is the monitor it opened on (the focused one).

- [ ] **Step 2: Run it**

```bash
cargo build --release
NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh
```

Expected (window id and dir vary):

```
nested niri: window 123, sockets in /tmp/nte2e.AbC123
  PASS  the nested niri is parked unfocused on a spare workspace, 1600x1000
  PASS  the nested niri's workspace files its tasks under e2e
  PASS  niri-tasks.service was left alone (main pid 4567)

passed: 3   failed: 0   skipped: 0
```

Exit status 0. If the output size check dies, read the reported size and `niri msg -j windows` for the window's `layout` before changing anything.

- [ ] **Step 3: Check it left nothing behind**

Before and after a run, compare:

```bash
niri msg -j windows | python3 -c 'import json,sys; print([w["id"] for w in json.load(sys.stdin) if w["app_id"]=="niri"])'
niri msg workspaces
ls -d /tmp/nte2e.* 2>/dev/null
systemctl --user is-active niri-tasks.service
```

Expected after: no `niri` window, the same workspaces as before, no `/tmp/nte2e.*`, service `active`.

- [ ] **Step 4: Commit**

```bash
git add tests/e2e-panel.sh
git commit -m "Run the task panel's e2e test in a nested niri it starts, parks and tears down

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Shoot until still, measure, and the tucked checks

Add `shot`, `same`, `frame_size` and `measure`, then the baseline, one-task, three-task and other-tag checks, with exact numbers.

**Files:**
- Modify: `tests/e2e-panel.sh` — insert helpers after `settle() { ... }` and checks immediately above `# ─── the real daemon, untouched ───`.

**Interfaces:**
- Consumes (Task 1): `NENV`, `TAG`, `SB`, `OUT_W`, `OUT_H`, `PEEK`, `RING`, `PEEK_X`, `ok`, `bad`, `die`, `guard`, `add`, `settle`.
- Produces (Task 3 uses):
  - `shot <label>` — writes `$SB/shots/<label>.png` once two consecutive nested frames match; returns 1 after a FAIL if they never do. Calls `guard` before and after.
  - `same <label-a> <label-b>` — exit 0 when the two frames are pixel-identical.
  - `measure <label> [<against>=baseline]` — prints `x0 x1 y0 y1`: the first changed column, one past the last, the first changed row, one past the last; `0 0 0 0` when nothing changed.
  - `one_h` — height in px of the one-task peek (`y1 - y0`).
  - Frames `baseline`, `one`, `three`.

- [ ] **Step 1: Add the helpers**

Insert after the `settle() { sleep 2; }` line:

```bash
# Whether a frame loads: niri names the file before it has finished writing it.
loads() {
    python3 -c 'import sys; from PIL import Image; Image.open(sys.argv[1]).load()' "$1" 2>/dev/null
}

# Whether two frames are the same, pixel for pixel.
same() {  # label label
    python3 - "$SB/shots/$1.png" "$SB/shots/$2.png" <<'PY'
import sys
from PIL import Image, ImageChops
a, b = (Image.open(p).convert("RGB") for p in sys.argv[1:3])
sys.exit(0 if a.size == b.size and ImageChops.difference(a, b).getbbox() is None else 1)
PY
}

# Shoot the nested screen, and keep the frame once two in a row match. Parked
# off screen, the nested niri draws only when asked, so the first frame after
# a change can still show the last one, or the panel half way through sliding.
shot() {  # label
    local label="$1" n
    guard
    rm -f "$SB/shots/.prev.png"
    for n in $(seq 1 10); do
        rm -f "$SB/shots/.this.png"
        "${NENV[@]}" niri msg action screenshot-screen --show-pointer false \
            --path "$SB/shots/.this.png" >/dev/null 2>&1
        for _ in $(seq 1 25); do loads "$SB/shots/.this.png" && break; sleep 0.1; done
        loads "$SB/shots/.this.png" || { bad "the nested niri wrote no screenshot for $label"; return 1; }
        if [ "$n" -gt 1 ] && same .prev .this; then
            mv "$SB/shots/.this.png" "$SB/shots/$label.png"
            rm -f "$SB/shots/.prev.png"
            guard
            return 0
        fi
        mv "$SB/shots/.this.png" "$SB/shots/.prev.png"
        sleep 0.3
    done
    bad "the nested screen never held still for $label: ten frames, no two alike"
    return 1
}

frame_size() {  # label
    python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' "$SB/shots/$1.png"
}

# Where a frame differs from another (the baseline unless given), as
# "<x0> <x1> <y0> <y1>": the first column that changed and one past the last,
# the same for rows; "0 0 0 0" when nothing did.
#
# A line counts only when more than MIN_RUN of its pixels changed by more than
# SENSITIVITY levels. The cards' shadows fade into the background over many
# pixels, and these two numbers are what put the panel's edge at the same
# column every run; the exact expectations below were measured with them.
measure() {  # label [against]
    python3 - "$SB/shots/${2:-baseline}.png" "$SB/shots/$1.png" <<'PY'
import sys
from PIL import Image, ImageChops

SENSITIVITY = 6   # levels of difference that count as changed at all
MIN_RUN = 20      # changed pixels in a line before it is panel

base = Image.open(sys.argv[1]).convert("RGB")
frame = Image.open(sys.argv[2]).convert("RGB")
mask = (ImageChops.difference(base, frame)
        .convert("L")
        .point(lambda p: 255 if p > SENSITIVITY else 0))

def lines(img):
    w, h = img.size
    data = img.tobytes()
    return [y for y in range(h) if data[y * w:(y + 1) * w].count(255) >= MIN_RUN]

rows = lines(mask)
cols = lines(mask.transpose(Image.Transpose.TRANSPOSE))
if cols and rows:
    print(cols[0], cols[-1] + 1, rows[0], rows[-1] + 1)
else:
    print(0, 0, 0, 0)
PY
}
```

- [ ] **Step 2: Add the tucked checks**

Insert immediately above `# ─── the real daemon, untouched ───`:

```bash
# ─── baseline: the nested screen with nothing on it ──────────────────────────
shot baseline || exit 1
read -r base_w base_h < <(frame_size baseline)
[ "$base_w $base_h" = "$OUT_W $OUT_H" ] ||
    die "the nested screen shoots at ${base_w}x${base_h}, not ${OUT_W}x${OUT_H} — every number below assumes it"

# ─── one task: a peek on the right edge ──────────────────────────────────────
add "ship it"
settle
shot one || exit 1
read -r x0 x1 y0 y1 < <(measure one)
one_h=$((y1 - y0))
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "one task pokes out a ${PEEK}px peek and its ${RING}px ring (columns ${x0}-${x1})"
else
    bad "one task drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W} —
      0-0 means nothing drew; an earlier start means it is not tucked away.
      NIRITASKS_E2E_KEEP=1 keeps the frames to tell which"
fi

# ─── three tasks: taller, same peek ──────────────────────────────────────────
add "write the glossary"
add "a much longer description that has to ellipsise rather than wrap onto a second line"
settle
shot three || exit 1
read -r x0 x1 y0 y1 < <(measure three)
three_h=$((y1 - y0))
if [ "$one_h" -gt 0 ] && [ "$three_h" -gt $((one_h * 2)) ]; then
    ok "three tasks stack three cards (${three_h}px tall vs ${one_h}px for one)"
else
    bad "three tasks drew ${three_h}px of panel against ${one_h}px for one —
      the cards are not stacking, or a long one wrapped"
fi
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "and the peek stays the same width (columns ${x0}-${x1})"
else
    bad "the peek moved with the task count: columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
fi

# ─── a task on another tag stays off this panel ──────────────────────────────
task rc.verbose=nothing rc.confirmation=no add +niritasks_e2e_elsewhere -- "not here" >/dev/null 2>&1
settle
shot elsewhere || exit 1
if same three elsewhere; then
    ok "a task on another workspace's tag changes nothing on screen"
else
    read -r x0 x1 y0 y1 < <(measure elsewhere three)
    bad "a task on another tag changed columns ${x0}-${x1}, rows ${y0}-${y1}"
fi
```

- [ ] **Step 3: Run it**

```bash
NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh
```

Expected: Task 1's three PASS lines plus

```
  PASS  one task pokes out a 30px peek and its 4px ring (columns 1566-1600)
  PASS  three tasks stack three cards (...px tall vs ...px for one)
  PASS  and the peek stays the same width (columns 1566-1600)
  PASS  a task on another workspace's tag changes nothing on screen
```

`failed: 0`. If a column differs, follow "Facts the executor needs": keep the frames, find the reason, don't add a tolerance.

- [ ] **Step 4: Prove the exact checks can fail**

Temporarily change `PEEK_X=$((OUT_W - PEEK - RING))` to `PEEK_X=$((OUT_W - PEEK - RING - 1))`, run again, and confirm both peek checks FAIL with `expected 1565-1600` and the script exits 1. Revert the line (`git diff tests/e2e-panel.sh` shows only Task 2's additions).

- [ ] **Step 5: Commit**

```bash
git add tests/e2e-panel.sh
git commit -m "Measure the nested niri's panel exactly, once two frames in a row agree

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Keyboard, Escape, empty and cold-start checks; the focus guard proved

**Files:**
- Modify: `tests/e2e-panel.sh` — insert immediately above `# ─── the real daemon, untouched ───` (below Task 2's other-tag section).

**Interfaces:**
- Consumes: `NENV`, `NIRITASKS`, `TAG`, `DAEMON`, `SB`, `OUT_W`, `CARD`, `CARD_X`, `SURFACE_LEFT`, `SURFACE_RIGHT`, `PEEK_X`, `one_h`, `shot`, `same`, `measure`, `add`, `settle`, `ok`, `bad`, `skip`, `die` (Tasks 1–2). Frames `baseline`, `three`.
- Produces: the finished script.

- [ ] **Step 1: Add the checks**

Insert immediately above `# ─── the real daemon, untouched ───`:

```bash
# ─── the keyboard: every card wrapped, with its buttons, mid-screen ──────────
# `task panel` is the keybind's command: it asks this sandbox's daemon to hand
# its panel the keyboard, and the panel leaves the right edge for the middle of
# the screen. Everything it draws is then inside the centred surface.
"${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
settle
shot keyboard || exit 1
read -r x0 x1 _ _ < <(measure keyboard)
if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
    ok "the keyboard takes the panel off the right edge to the middle (columns ${x0}-${x1})"
else
    bad "with the keyboard the panel covers columns ${x0}-${x1}, expected within
      ${SURFACE_LEFT}-${SURFACE_RIGHT} — 0-0 means nothing drew; reaching ${OUT_W} means it
      did not leave the edge"
fi

# Down moves the darker fill from the first card to the second, and nothing
# else in the frame changes, so what differs between the two frames is exactly
# two cards: their columns, and with their action rows, well over twice the
# height of one card's one-line peek.
if command -v wtype >/dev/null; then
    "${NENV[@]}" wtype -k Down
    sleep 1
    shot keyboard_down || exit 1
    read -r x0 x1 y0 y1 < <(measure keyboard_down keyboard)
    key_h=$((y1 - y0))
    if [ "$x0" -eq "$CARD_X" ] && [ "$x1" -eq $((CARD_X + CARD)) ]; then
        ok "and shows its cards in the middle of the screen (columns ${x0}-${x1})"
    else
        bad "the keyboard's cards cover columns ${x0}-${x1}, expected
      ${CARD_X}-$((CARD_X + CARD)) — 0-0 means Down did not move the focus"
    fi
    if [ "$key_h" -ge $((one_h * 2 + 40)) ]; then
        ok "and each card grows its buttons (${key_h}px for two cards vs ${one_h}px for one tucked away)"
    else
        bad "two of the keyboard's cards are ${key_h}px tall against ${one_h}px for
      one tucked away — the action rows are missing"
    fi

    "${NENV[@]}" wtype -k Escape
    settle
    shot released || exit 1
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi
else
    skip "the panel's cards in the middle of the screen, their buttons, and Escape back (needs wtype)"
fi

# ─── nothing pending shows nothing ───────────────────────────────────────────
# rc.bulk=0: completing more than two tasks at once otherwise stops to ask,
# and with no terminal to answer, completes none of them. A panel with no
# tasks also gives the keyboard back, when wtype was not there to press Escape.
task rc.verbose=nothing rc.confirmation=no rc.bulk=0 "+$TAG" done </dev/null >/dev/null 2>&1
settle
shot empty || exit 1
if same baseline empty; then
    ok "the panel goes away when the workspace has no tasks"
else
    read -r x0 x1 y0 y1 < <(measure empty)
    bad "something is still drawn with no tasks (columns ${x0}-${x1}, rows ${y0}-${y1})"
fi

# ─── the map-once trap: a daemon that starts with nothing to show ────────────
# A layer surface that has never been mapped does not respond to a later
# present(), so this once stayed invisible for an entire session however many
# tasks were added afterwards.
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
sleep 1
"${NENV[@]}" "$NIRITASKS" daemon >"$SB/daemon2.err" 2>&1 &
DAEMON=$!
sleep 3
add "after a cold start"
settle
shot cold_start || exit 1
read -r x0 x1 _ _ < <(measure cold_start)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "a daemon started with nothing to show still shows the next task (columns ${x0}-${x1})"
else
    bad "after a cold start with no tasks the panel drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
fi
```

- [ ] **Step 2: Run the whole test with the desktop doing its worst**

With the animated swww wallpaper on and this terminal in the middle of the screen (the conditions every 2026-10-01 failure came from), note `systemctl --user show -p MainPID --value niri-tasks.service`, then:

```bash
time NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh
```

Expected: every line PASS, `failed: 0   skipped: 0`, exit 0, including

```
  PASS  the keyboard takes the panel off the right edge to the middle (columns ...-...)
  PASS  and shows its cards in the middle of the screen (columns 420-1180)
  PASS  and each card grows its buttons (...)
  PASS  Escape puts it back exactly as it was before the keyboard took it
  PASS  the panel goes away when the workspace has no tasks
  PASS  a daemon started with nothing to show still shows the next task (columns 1566-1600)
  PASS  niri-tasks.service was left alone (main pid <same as noted>)
```

Note the wall time for the README (Task 4). Then repeat Task 1 Step 3's leftover checks.

Run it twice more; all three runs must pass identically (the numbers in the PASS lines must not vary).

- [ ] **Step 3: Prove the focus guard fails loudly**

```bash
NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh > /tmp/claude-e2e-guard.log 2>&1 &
sleep 12
WIN=$(sed -n 's/^nested niri: window \([0-9]*\),.*/\1/p' /tmp/claude-e2e-guard.log)
niri msg action focus-window --id "$WIN"
wait $!; echo "exit $?"
cat /tmp/claude-e2e-guard.log; rm /tmp/claude-e2e-guard.log
```

(Use the scratchpad directory instead of `/tmp` if one is available to you.) Expected: a line `  FAIL  the nested niri's window has the focus — anything typed there would reach the panel, ...` (or `... workspace (...) is on screen`), the summary, and `exit 1`. Then repeat Task 1 Step 3's leftover checks: teardown must still be clean, and focus returns to a desktop window once the nested one closes.

- [ ] **Step 4: Commit**

```bash
git add tests/e2e-panel.sh
git commit -m "Check the keyboard, Escape, an empty panel and a cold start in the nested niri

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: README and tests/all.sh for the nested setup

**Files:**
- Modify: `README.md` — the running-the-tests block and table row (lines ~235–262), the `e2e-panel.sh` paragraph (~265–275), and the `tests/e2e-panel.sh` part of "Why the three scripts are not cargo tests" (~342–357).
- Modify: `tests/all.sh` — header comment (lines ~24–29), `why_panel`, and the warning before the takeover suites (~89–95).

**Interfaces:**
- Consumes: the finished script's behaviour (Tasks 1–3) and the wall time measured in Task 3 Step 2.

- [ ] **Step 1: README — command block**

In the block under "### Running the tests", replace the `e2e-panel.sh` line's timing `~35s` with the wall time from Task 3 Step 2, rounded to 5s, keeping the column alignment:

```
bash tests/e2e-panel.sh     # the task panel, in a nested niri       ~NNs
```

Update `tests/all.sh`'s `~105s` in the same block by the same difference.

- [ ] **Step 2: README — table row**

Replace the `tests/e2e-panel.sh` row with:

```markdown
| `tests/e2e-panel.sh` | niri, a Wayland session, `python3-pil`, taskwarrior; `wtype` for its keyboard checks, which are skipped without it | A nested niri window for the run, parked on the spare workspace at the end of your monitor; keep off that workspace until it finishes. Not your clipboard, your screenshots or the `niri-tasks` daemon |
```

- [ ] **Step 3: README — the `e2e-panel.sh` paragraph**

Replace the paragraph that begins "`e2e-panel.sh` needs the right edge to hold still" and ends "…what a failure actually looked like." with:

```markdown
`e2e-panel.sh` runs on a screen of its own: a nested niri, started with its own
runtime directory and a flat 1600x1000 output, with its own daemon against a
sandboxed `TASKDATA`. Your wallpaper, windows, window rules, notifications and
pointer cannot reach its frames, and your `niri-tasks` daemon keeps running
throughout. The nested niri's window opens over yours for a moment, then is
parked floating and unfocused on the last workspace of that monitor. Keep off
that workspace while it runs: going there focuses the window, and the run
fails, saying so, rather than measuring frames your typing could have reached.
`NIRITASKS_E2E_KEEP=1` leaves the frames on disk when you need to see what a
failure actually looked like.
```

- [ ] **Step 4: README — "Why the three scripts are not cargo tests"**

In the `tests/e2e-panel.sh` paragraph, replace "So it screenshots the right edge and measures the panel against a baseline taken with no tasks:" with "So it screenshots a nested niri of its own and measures the panel against a frame taken with no tasks:". Keep the rest of that sentence's list, then add "the keyboard's cards in the middle of the screen, and Escape putting them back," before "nothing for another tag's task".

Replace the following paragraph ("It counts lines rather than pixels. …") with:

```markdown
The nested screen is a flat colour at a fixed size, so its numbers are exact:
the peek starts at column 1566 of 1600, the keyboard's cards span 420–1180, and
a frame that should not have changed — another tag's task, Escape, an empty
panel — is compared pixel for pixel. A nested niri parked out of sight draws
only when asked, so each frame is shot until two in a row agree. It counts
lines rather than pixels: the cards' shadows fade into the background over
many pixels, and a column or row counts only once more than 20 of its pixels
changed, which is what puts the panel's edge at the same column every run.
```

- [ ] **Step 5: tests/all.sh**

Header comment, lines 24–29 — replace "e2e-panel.sh takes screenshots and with them the clipboard, and e2e-box.sh types into whatever has focus. So the cheap suites have already reported by the time you have to leave the keyboard alone" with "e2e-panel.sh opens a nested niri on a spare workspace, and e2e-box.sh types into whatever has focus. So the cheap suites have already reported by the time you have to leave the keyboard alone" (rewrap to the file's width). Also in the header's suite list (lines 6–8), "one needs Pillow and a compositor that will take screenshots" becomes "one needs Pillow and niri to nest a niri in".

`why_panel` — add, after the `NIRI_SOCKET` line:

```bash
    [ -n "${WAYLAND_DISPLAY:-}" ] || { echo "no Wayland display"; return; }
```

The warning — replace

```bash
if [ -z "$(why_panel)" ]; then
    echo
    echo "  ! the next two suites take the machine over: screenshots use the"
    echo "    clipboard, and the box test types into whatever has focus."
    echo "    Leave the keyboard alone until they finish."
fi
suite "tests/e2e-panel.sh" "$(why_panel)" bash tests/e2e-panel.sh
```

with

```bash
if [ -z "$(why_panel)" ]; then
    echo
    echo "  ! the panel test parks a nested niri on the last workspace of this"
    echo "    monitor: keep off that workspace until it finishes."
fi
suite "tests/e2e-panel.sh" "$(why_panel)" bash tests/e2e-panel.sh
if [ -z "$(why_box)" ]; then
    echo
    echo "  ! the box test types into whatever has focus."
    echo "    Leave the keyboard alone until it finishes."
fi
```

- [ ] **Step 6: Check nothing stale is left**

```bash
grep -n "right edge to hold still\|clipboard\|SHOTDIR\|Screenshots\|stillness\|notification" README.md tests/all.sh tests/e2e-panel.sh
```

Expected: no hit about `e2e-panel.sh` (hits about `e2e-box.sh` or the product's own features are fine — read each one). Then:

```bash
bash -n tests/all.sh tests/e2e-panel.sh && NIRITASKS=./target/release/niritasks bash tests/all.sh
```

Expected: `cargo test`, `tests/e2e-tag.sh` and `tests/e2e-panel.sh` PASS; `tests/e2e-box.sh` runs after its own warning (leave the keyboard alone for it); exit 0.

- [ ] **Step 7: Commit**

```bash
git add README.md tests/all.sh
git commit -m "Describe the panel's e2e test running in its own nested niri

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
