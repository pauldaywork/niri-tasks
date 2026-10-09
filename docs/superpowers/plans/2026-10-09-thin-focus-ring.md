# Thin the Window Focus Ring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the edge niri draws round every window slightly thinner (4px → 3px), so windows sit a little lighter on screen.

**Architecture:** The edge is the `focus-ring` in the `window-rule` of both window-rules profiles, `~/.config/niri/window-rules/normal.kdl` and `focus.kdl`. `window-rules-active.kdl` is a symlink to one of them (currently `focus.kdl`) and Mod+Alt+F (`window-rules/toggle.sh`) swaps it. Set `width 3` in both profiles so toggling never changes the width. Nothing in this repo's code changes.

**Tech Stack:** niri KDL config, `niri validate`, `niri msg`.

**Spec:** Taskwarrior task `6e69a516-f27f-4022-a665-f7e8db4d4a94` — read it with `task rc.json.array=on 6e69a516-f27f-4022-a665-f7e8db4d4a94 export`; its description and annotations are the spec.

## Global Constraints

- Ring width 4 → 3 ("slightly"); drop to 2 only if 3 is not visibly thinner.
- Change both profiles the same, so toggling Mod+Alt+F does not change the edge's width.
- Out of scope: the `border` blocks (all `off`; `normal.kdl:12-14`, `focus.kdl:11-13` also say `width 4` — leave them), ring colours (`#00000020` in the profiles, the colour-only `layout { focus-ring }` in `config.kdl:711`), and niri-tasks' panel and task box CSS (`src/panel/style.rs`, already 1px).
- `~/.config/niri` is not a git repository: there is nothing to commit there. Only this plan file is committed in the worktree.

---

### Task 1: Set the focus-ring width to 3 in both profiles

**Files:**
- Modify: `~/.config/niri/window-rules/normal.kdl:61` (inside `focus-ring {` at line 59)
- Modify: `~/.config/niri/window-rules/focus.kdl:50` (inside `focus-ring {` at line 48)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing other tasks use.

There is no automated test for a config value. The "failing test" is a check that shows the current state, and the pass is the same check plus `niri validate`.

- [ ] **Step 1: Show the current state (the check that should fail)**

Run:

```bash
cd ~/.config/niri/window-rules
for f in normal.kdl focus.kdl; do echo "== $f"; sed -n '/focus-ring {/,/}/p' "$f" | grep -n width; done
```

Expected: both print `width 4`. If either line numbers in the Files block no longer match, trust this output over the line numbers.

- [ ] **Step 2: Change the width inside the focus-ring blocks only**

The `border` blocks also contain `width 4` and must not change, so scope the substitution to the `focus-ring { … }` range:

```bash
cd ~/.config/niri/window-rules
sed -i '/focus-ring {/,/}/ s/^\(\s*\)width 4$/\1width 3/' normal.kdl focus.kdl
```

Each `focus-ring` block should now read:

```kdl
    focus-ring {
        on
        width 3
        active-color "#00000020"
        inactive-color "#00000020"
        urgent-color "#00000020"
    }
```

- [ ] **Step 3: Verify only the ring changed**

Run:

```bash
cd ~/.config/niri/window-rules
for f in normal.kdl focus.kdl; do echo "== $f"; sed -n '/focus-ring {/,/}/p' "$f" | grep width; sed -n '/border {/,/}/p' "$f" | grep width; done
```

Expected, for each file: `width 3` (ring) then `width 4` (border, untouched).

- [ ] **Step 4: Validate the config**

Run: `niri validate`
Expected: exits 0 and reports the config is valid. If it fails, fix the KDL before going on — a broken config makes niri keep the old one and log an error.

- [ ] **Step 5: Make sure niri has loaded it**

niri watches its config and normally reloads on save. Load it explicitly anyway; it is harmless if already loaded:

```bash
niri msg action load-config-file
```

Expected: no error output.

- [ ] **Step 6: Check both profiles on screen**

Look at a few windows: the faint dark ring should be visibly thinner than before. Then press Mod+Alt+F to switch profile (a "Window rules: normal" or "…: focus" notification appears), check the ring is the same 3px width, and press Mod+Alt+F again to return to the starting profile (`focus` at the time of writing; `readlink ~/.config/niri/window-rules-active.kdl` shows which).

If 3px is not visibly thinner than 4px, repeat Steps 2–5 with `width 3` → `width 2` in both files, per the spec's decision. Report which width you landed on.

- [ ] **Step 7: Commit the plan**

Only the plan lives in this repo; the config change is outside it.

```bash
git add docs/superpowers/plans/2026-10-09-thin-focus-ring.md
git commit -m "docs: plan thinning the window focus ring to 3px

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

**Done when:** every window's ring is 3px in both the normal and focus profiles, and `niri validate` passes.
