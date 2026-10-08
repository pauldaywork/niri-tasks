# Nested niri without HERDR_* Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** e2e runs started from a herdr pane behave as from a plain terminal: nothing inside the nested niri sees the parent pane's `HERDR_*`.

**Architecture:** `tests/lib/nested-niri.sh` is the one place both nested e2e suites (`tests/e2e-panel.sh`, `tests/e2e-box.sh`) start their nested niri and build `NENV`, the `env` prefix every nested command runs through. `nested_start` collects every `HERDR_*` name in its own environment at run time and passes each as `env -u NAME` both to the `niri` launch and at the front of `NENV`. A new `ok`/`bad` check in `nested_start` proves neither the nested niri's own environment (`$SB/nested.env`) nor a command run through `nested` has a `HERDR_` variable.

**Tech Stack:** bash 5 (`compgen -e`, arrays), GNU `env -u`, niri.

**Spec:** Taskwarrior task `a030807c-c139-42e1-a18e-ac41cb705a7b` — read it with `task rc.json.array=on a030807c-c139-42e1-a18e-ac41cb705a7b export`; its description and annotations are the spec.

## Global Constraints

- Unset every `HERDR_*` present at run time (prefix match, as `src/speak.rs` `dropped_vars` does), not a fixed list, in both the niri launch and `NENV`.
- Fix it once in the shared `tests/lib/nested-niri.sh` so `e2e-panel.sh` and `e2e-box.sh` both get it; `e2e-tag.sh` already unsets its own `HERDR_*` and is not touched.
- Build the `-u` args from `compgen -e | grep '^HERDR_'`.
- In `nested_start`, add an `ok` check that `$SB/nested.env` has no `HERDR_` line.
- Out of scope: changing how the CLI reads `HERDR_*` (`src/lib.rs`, `src/link.rs`); cargo tests.
- `env -u` options must come before any `NAME=value` assignment in an `env` command line (the existing comment on `NENV` says so).
- Commit messages: Conventional Commits; the task is a `fix:`, and this repo's e2e commits use the `e2e` scope, e.g. `test(e2e): …`.

---

### Task 1: Keep HERDR_* out of the nested niri

**Files:**
- Modify: `tests/lib/nested-niri.sh` — the `env -u NIRI_SOCKET … niri -c` launch in `nested_start`, the `NENV=(…)` assignment, and a new check after it; one line in the header comment.
- Modify: `README.md` — the `e2e-box.sh` paragraph that begins "`e2e-box.sh` runs in the same kind of nested niri" (around line 386).

**Interfaces:**
- Consumes: `ok`, `bad`, `nested`, `NENV`, `$SB` from `tests/lib/nested-niri.sh`.
- Produces: nothing new for other files. `NENV` keeps its shape — `env`, then `-u` options, then assignments — so `NENV+=(XDG_DATA_HOME=…)` in `tests/e2e-panel.sh:175` keeps working.

**Background for the implementer:** herdr is a terminal multiplexer. A shell in a herdr pane has `HERDR_SESSION`, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_ENV`, `HERDR_BIN_PATH` and maybe more (`env | grep ^HERDR_` lists them). `niritasks task add` and `task status active` read them to pick a tag and link the pane. The nested niri's only workspace is `e2e`, so with an inherited named session `niritasks task add` inside it fails with "does not match any named workspace", and `task status active` could rename the real pane's agent. To reproduce, you must run from a herdr pane of a named session; if `env | grep ^HERDR_` prints nothing in your shell, fake it with `HERDR_SESSION=niri-tasks HERDR_PANE_ID=fake` in front of each command below.

- [ ] **Step 1: Write the failing check**

In `tests/lib/nested-niri.sh`, `nested_start`, directly after the `NENV=(…)` assignment (the block ending `NOTIFY_LOG="$SB/notifications" PATH="$SB/bin:$PATH")`) and before the `for _ in $(seq 1 25); do WIN=$(nested_window …` loop, add:

```bash
    # Started from a herdr pane, the run must behave as from a plain
    # terminal: an inherited HERDR_SESSION names a session the nested niri
    # has no workspace for, and HERDR_PANE_ID points at your real pane.
    if grep -q '^HERDR_' "$SB/nested.env" || nested env | grep -q '^HERDR_'; then
        bad "HERDR_* reached the nested niri: $( { grep -o '^HERDR_[A-Z_]*' "$SB/nested.env"; nested env | grep -o '^HERDR_[A-Z_]*'; } | sort -u | tr '\n' ' ')"
    else
        ok "no HERDR_* reaches the nested niri or what runs in it"
    fi
```

The `nested env` half covers `NENV`, which the spec's done-when checks by hand; one check proves both halves.

- [ ] **Step 2: Run it to see it fail**

From a herdr pane (or with the fake variables from the background note):

```bash
bash -c '. tests/lib/nested-niri.sh; nested_start; summary; [ "$fail" -eq 0 ]'
```

Expected: a line `FAIL  HERDR_* reached the nested niri: HERDR_PANE_ID HERDR_SESSION …`, then `failed: 1`, exit status 1. The nested window appears briefly on your last workspace and goes away on exit.

- [ ] **Step 3: Unset every HERDR_* in both places**

In `nested_start`, at the start of the function body (before the `local spawn_env=""` line), add:

```bash
    # Every HERDR_* you have, as env -u options: a herdr pane's session,
    # socket and pane are yours, not the nested niri's. By prefix, not a
    # list, so a variable herdr adds later is dropped too.
    local herdr_unset=() var
    while read -r var; do
        herdr_unset+=(-u "$var")
    done < <(compgen -e | grep '^HERDR_')
```

Change the niri launch from:

```bash
    env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        vblank_mode=0 __GL_SYNC_TO_VBLANK=0 niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
```

to:

```bash
    env -u NIRI_SOCKET "${herdr_unset[@]}" XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        vblank_mode=0 __GL_SYNC_TO_VBLANK=0 niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
```

Change the `NENV` comment and assignment from:

```bash
    # -u DISPLAY and GDK_BACKEND: if the nested niri dies, GTK must not fall
    # back to X11 and open a box on the real desktop. env -u comes first.
    NENV=(env -u DISPLAY XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$n_wayland"
```

to:

```bash
    # -u DISPLAY and GDK_BACKEND: if the nested niri dies, GTK must not fall
    # back to X11 and open a box on the real desktop. -u HERDR_*: as for the
    # nested niri. env -u comes first.
    NENV=(env -u DISPLAY "${herdr_unset[@]}" XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$n_wayland"
```

(the rest of the `NENV=(…)` lines stay as they are). With no `HERDR_*` set, `herdr_unset` is empty and `"${herdr_unset[@]}"` expands to nothing, which bash 5 allows under `set -u`.

The nested niri's environment is what its spawns, and so the daemon's children, inherit; the daemon itself is started through `NENV` by `nested_daemon_start`. Both paths are now covered.

- [ ] **Step 4: Run it to see it pass**

```bash
bash -c '. tests/lib/nested-niri.sh; nested_start; summary; [ "$fail" -eq 0 ]'
```

Expected: `PASS  no HERDR_* reaches the nested niri or what runs in it`, `failed: 0`, exit status 0.

Also from a shell with no `HERDR_*` (proves the empty array is fine):

```bash
env $(compgen -e | grep '^HERDR_' | sed 's/^/-u /') bash -c '. tests/lib/nested-niri.sh; nested_start; summary; [ "$fail" -eq 0 ]'
```

Expected: the same PASS, exit status 0.

- [ ] **Step 5: Say so in the header comment and the README**

In the header comment of `tests/lib/nested-niri.sh`, after the paragraph ending `nested_service_untouched proves it at the end of a run.`, add a paragraph:

```bash
#
# Started from a herdr pane, a run behaves as from a plain terminal: no
# HERDR_* you have reaches the nested niri, its spawns or anything run
# through `nested`, so a task added in it is never filed under your herdr
# session or linked to your pane.
```

In `README.md`, at the end of the paragraph beginning "`e2e-box.sh` runs in the same kind of nested niri", after "Your own `niri-tasks` daemon is never stopped.", add the sentence:

```markdown
Run from a herdr pane, both nested tests drop every `HERDR_*` variable before
starting the nested niri, so they behave as from a plain terminal and never
touch your herdr session or pane.
```

Rewrap that paragraph to the surrounding line width (about 80 columns) if needed.

- [ ] **Step 6: Run the done-when checks**

From a pane of a named herdr session (`echo $HERDR_SESSION` prints a name), with a fresh build:

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh
```

Expected: `e2e-panel.sh` and `e2e-box.sh` both pass, each printing `PASS  no HERDR_* reaches the nested niri or what runs in it`, and no box test fails with "does not match any named workspace".

Then the by-hand check that `nested env` is clean:

```bash
bash -c '. tests/lib/nested-niri.sh; nested_start >/dev/null; nested env | grep ^HERDR_; echo "grep exit: $?"'
```

Expected: no `HERDR_` lines, then `grep exit: 1`.

- [ ] **Step 7: Commit**

```bash
git add tests/lib/nested-niri.sh README.md
git commit -m "fix(e2e): unset HERDR_* in the nested niri

Run from a herdr pane, the nested niri, its spawns, its daemon and
every nested command inherited the pane's HERDR_SESSION and
HERDR_PANE_ID. task add then failed with no workspace matching the
session, and task status active could rename the real pane's agent.
Every HERDR_* is now dropped by prefix from the niri launch and NENV,
and nested_start checks none got through.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
