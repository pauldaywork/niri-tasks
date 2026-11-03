# Draw e2e-panel's frames in the panel's real font Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** e2e-panel's pixel checks measure the panel in the font it really draws in, Iosevka Term Extended plus the JetBrainsMono Nerd Font glyphs, on the host and in the container.

**Architecture:** `tests/e2e-panel.sh` gives the nested session `XDG_DATA_HOME="$SB/share"` (for the Ideas tab), and fontconfig reads user fonts only from `$XDG_DATA_HOME/fonts`, so today the frames draw in Noto Sans and other fallbacks. The fix links `$SB/share/fonts` to the real fonts folder before the daemon starts, then checks with `fc-match`, under the nested env, that the panel's font and its action glyphs resolve to the real fonts; a miss fails the run before any shot. The font-dependent expected heights are then re-measured, and the Dockerfile comment, `check-image.sh` and the README stop describing the fallback.

**Tech Stack:** bash, fontconfig (`fc-match`), Python Pillow (already used by the suite), Docker (`tests/container.sh`).

**Spec:** Taskwarrior task `355002ca-b11c-41f0-a2a3-dcf0e59b906f` (its description and notes; read with `task rc.json.array=on 355002ca export`). No separate spec file.

## Global Constraints

- Link `"$SB/share/fonts"` → `"${XDG_DATA_HOME:-$HOME/.local/share}/fonts"` before the daemon starts. Not `XDG_DATA_DIRS` (fontconfig never reads it), not a `FONTCONFIG_FILE` override.
- Before the first shot, `fc-match 'Iosevka Term Extended'` under the nested env must name Iosevka Term; if not, that is a `bad` line and the run ends there.
- Done when: the font check passes and all of e2e-panel's checks pass on the host and in `tests/container.sh`, both drawing in Iosevka Term Extended.
- Out of scope: changing the panel's font or style (`src/panel/style.rs`), and the font setup of `tests/e2e-box.sh` and `tests/e2e-tag.sh`.
- Run e2e-panel against this worktree's build, never the installed binary: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`.
- While a host run goes, keep off the last workspace of the monitor it starts from (the nested window is parked there).
- Commits: Conventional Commits, `test(e2e): …` / `test(container): …` as in recent history, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Background the implementer needs

- `tests/lib/nested-niri.sh` makes the sandbox `$SB` (a `mktemp -d`), and `nested_start` fills the array `NENV` (an `env …` prefix) that runs a command inside the nested session. `NENV` does not change `HOME`. It defines `ok`, `bad`, `skip`, `die`, `summary`; `bad` counts a failure and carries on, so a fatal check is `bad …; summary; exit 1` (see the tag check at `tests/e2e-panel.sh:199-205`).
- Cleanup is `rm -rf "$SB"`. `rm -rf` removes a symlink, not what it points at, so the real fonts folder is safe. With `NIRITASKS_E2E_KEEP=1` the sandbox (link included) is left on disk with its frames in `$SB/shots/`.
- On this machine:
  `fc-match -f '%{family[0]}' 'Iosevka Term Extended'` → `Iosevka Term`;
  `fc-match -f '%{family[0]}' 'Iosevka Term Extended:charset=f141'` → `JetBrainsMonoNL Nerd Font` (the action buttons' Font Awesome glyphs, `src/actions.rs`);
  `XDG_DATA_HOME=/nonexistent fc-match 'Iosevka Term Extended'` → `NotoSans-Regular.ttf` (the bug).
  `tests/container/parity.sh:21-22` and `tests/container/check-image.sh:65-68` use the same queries.
- Which expectations depend on the font. In `tests/e2e-panel.sh`:
  - `PEEK=30`, `RING=3`, `CARD=760`, `SURFACE=784`, `NOTES_MAX=240`, `PADDING=12` (lines 64-69) mirror constants in `src/panel/layout.rs` and `src/panel/style.rs`. They are not font metrics; a check on them failing in the new font is a finding to investigate, not a number to edit.
  - `SHORT_NOTES_GROWTH=50` (line 70, used at 558-566): the id line and one note line plus a padding — font height.
  - `clear_strip_h=39` (line 771, "the button's 31px and the 8px gap"): the Clear all button's height is font height.
  - Inequalities that may move: `-lt 60` (line 488), `pair_h + 12` (line 522). Leave them unless they fail.
- Image inputs (`tests/container/image.sh`): the Dockerfile, `tests/container/dconf/00-machine`, `Cargo.toml`, `Cargo.lock`. Editing the Dockerfile comment rebuilds the image on the next `tests/container.sh` run (several minutes, needs the network). `check-image.sh` is not an input.

## File Structure

- Modify `tests/e2e-panel.sh` — the fonts link (beside the `XDG_DATA_HOME` line, ~194-196), the font check (before `nested_daemon_start`, ~208), and re-measured constants and their comments (`SHORT_NOTES_GROWTH` at 70 and its comment at 558-560; `clear_strip_h` at 766-771).
- Modify `tests/container/Dockerfile:91-98` — the font comment.
- Modify `tests/container/check-image.sh:69-70` — the two checks that assert the fallback.
- Modify `README.md:724-726` — the sentence saying the panel test's `XDG_DATA_HOME` hides the fonts.

---

### Task 1: Link the fonts into the nested session and re-measure on the host

**Files:**
- Modify: `tests/e2e-panel.sh:194-208` (link and check), `:70`, `:558-560`, `:766-771` (re-measured values), any other expectation the run shows moved
- Test: `tests/e2e-panel.sh` itself (it is the test)

**Interfaces:**
- Consumes: `NENV`, `SB`, `bad`, `ok`, `summary` from `tests/lib/nested-niri.sh`.
- Produces: `$SB/share/fonts` as a symlink to the real fonts folder; the log lines `PASS  the nested session draws in Iosevka Term …` that Task 2 looks for in the container log.

- [ ] **Step 1: Write the failing check**

In `tests/e2e-panel.sh`, directly after the existing line
`NENV+=(XDG_DATA_HOME="$SB/share")` (line 196) and before `TAG=$(…)`, add:

```bash
# The panel draws in Iosevka Term Extended (src/panel/style.rs), and its
# action buttons' glyphs come from JetBrainsMonoNL Nerd Font. Fontconfig
# finds a user's fonts only in $XDG_DATA_HOME/fonts, so without the link
# below the frames would draw in Noto Sans and the expectations would
# measure a fallback. Checked before the first shot, so a missing font
# fails here, by name, not as heights that are off.
font=$("${NENV[@]}" fc-match -f '%{family[0]}' 'Iosevka Term Extended' 2>/dev/null)
glyphs=$("${NENV[@]}" fc-match -f '%{family[0]}' 'Iosevka Term Extended:charset=f141' 2>/dev/null)
if [ "$font" = "Iosevka Term" ] && [ "$glyphs" = "JetBrainsMonoNL Nerd Font" ]; then
    ok "the nested session draws in Iosevka Term, its action glyphs in ${glyphs}"
else
    bad "under the nested session's XDG_DATA_HOME the panel's font is '${font}' and its
      action glyphs' '${glyphs}', expected 'Iosevka Term' and 'JetBrainsMonoNL Nerd Font' —
      are they in ${XDG_DATA_HOME:-$HOME/.local/share}/fonts?"
    summary; exit 1
fi
```

- [ ] **Step 2: Run it and see it fail**

```bash
cargo build
NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
```

Expected: `FAIL  under the nested session's XDG_DATA_HOME the panel's font is 'Noto Sans' …`, then the summary and a non-zero exit, before any shot.

- [ ] **Step 3: Add the link**

Directly above the font check (after `NENV+=(XDG_DATA_HOME="$SB/share")`), add:

```bash
# Fontconfig reads a user's fonts from $XDG_DATA_HOME/fonts and nowhere
# else of theirs, so the sandbox's share gets a link to the real folder.
# rm -rf of the sandbox removes the link, never the fonts.
mkdir -p "$SB/share"
ln -s "${XDG_DATA_HOME:-$HOME/.local/share}/fonts" "$SB/share/fonts"
```

Also extend the comment above `NENV+=(XDG_DATA_HOME=…)` so it no longer reads as if the sandbox share holds only the Ideas files — append: `Its fonts are the real ones, linked in below.`

- [ ] **Step 4: Run it and collect what moved**

```bash
NIRITASKS_E2E_KEEP=1 NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tee "$HOME/.local/share/niri-tasks/reviews/e2e-panel-real-font-host.log"
```

Expected: `PASS  the nested session draws in Iosevka Term, its action glyphs in JetBrainsMonoNL Nerd Font`. Then some height checks likely FAIL; each FAIL line prints the measured number. Note the kept sandbox path the run prints at the end (`the run's files are kept in …`). Record every FAIL line.

- [ ] **Step 5: Re-measure each moved expectation from the frames, not just the FAIL line**

For each FAIL:

- **`one short note grew the card Npx, expected 50px`** — Open the kept frames `$SB/shots/keyboard_down.png` and `$SB/shots/notes_shown.png` with the Read tool and look: the extra height must be exactly the dimmed id line and the one note line, with the padding over them and no blank space under the note (blank space means short notes are taking the scroller's minimum height — the bug this check guards; stop and report it rather than raising the number). If it looks right, set `SHORT_NOTES_GROWTH=N` and rewrite the comment at lines 558-560 to say where the number now comes from:

  ```bash
      # One short note must not cost the card more than its lines: the id line
      # and the note, a padding over them, make up SHORT_NOTES_GROWTH, measured
      # in Iosevka Term Extended. (1efd4c6, before the notes were put in a
      # scroller, measured 50 in the fallback font.)
  ```

- **`All's three cards are …, Xpx off two one-line cards`** — the Clear all strip changed height. The new `clear_strip_h` is `39 - X` (from `extra = keyboard_h - waiting_h - pair_h + clear_strip_h`, which should be 0). Confirm on the kept Waiting-tab frame `$SB/shots/tab_waiting.png` that the Clear all button plus the 8px gap above it is that tall, then set `clear_strip_h` and change the comment's "the button's 31px" to the button's new height.

- **Any check on `PEEK_X`, `CARD_X`, `SURFACE_LEFT/RIGHT` or `PADDING + NOTES_MAX`** — these are layout constants, not font metrics. Do not edit the numbers. Look at the frames, find out why the font moved them, and report it to the user before going on.

- **`-lt 60` (line 488) or `pair_h + 12` (line 522)** failing — read the check's comment, measure the frame, and change the threshold only if the comment's reasoning still holds in the new font; say so in the commit body.

- [ ] **Step 6: Run it until it passes, twice**

```bash
NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
```

Expected both times: `failed: 0`, exit 0, and the font PASS line first. Two runs, because the expectations are exact and must not depend on the run.

- [ ] **Step 7: Commit**

```bash
git add tests/e2e-panel.sh
git commit -m "test(e2e): draw the panel's frames in its real font

e2e-panel gives the nested session an XDG_DATA_HOME of its own, and
fontconfig finds a user's fonts only under it, so the checks measured
the panel in Noto Sans. Link the real fonts folder into the sandbox,
fail the run before the first shot when the font or the action glyphs
do not resolve, and re-measure the heights that the real font moved.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(Name the constants that moved, and their old and new values, in the body.)

---

### Task 2: Stop describing the fallback, and pass in the container

**Files:**
- Modify: `tests/container/Dockerfile:91-98`
- Modify: `tests/container/check-image.sh:69-70`
- Modify: `README.md:724-726`

**Interfaces:**
- Consumes: Task 1's link and font check in `tests/e2e-panel.sh`, and its PASS line text `the nested session draws in Iosevka Term`.
- Produces: nothing later tasks use.

- [ ] **Step 1: Change the image check to assert the link, not the fallback**

In `tests/container/check-image.sh`, replace lines 69-70:

```bash
check "another XDG_DATA_HOME hides them" sh -c 'XDG_DATA_HOME=/nonexistent fc-match -f "%{file}" "Iosevka Term Extended" | grep -q "/NotoSans-Regular.ttf$"'
check "and leaves the check mark to Liberation" sh -c 'XDG_DATA_HOME=/nonexistent fc-match -f "%{file}" "Iosevka Term Extended:charset=f00c" | grep -q "/LiberationSerif-Italic.ttf$"'
```

with one check that does what e2e-panel does:

```bash
check "e2e-panel's fonts link finds them" sh -c 'd=$(mktemp -d) && ln -s ~/.local/share/fonts "$d/fonts" && XDG_DATA_HOME="$d" fc-match -f "%{file}" "Iosevka Term Extended" | grep -q "/IosevkaTerm-Extended.ttf$"; r=$?; rm -rf "$d"; exit $r'
```

- [ ] **Step 2: Rewrite the Dockerfile's font comment**

Replace `tests/container/Dockerfile` lines 91-98 (from `# The machine's own fonts, where it has them:` through `# Fonts in /usr/local/share/fonts stay in reach and draw another height.`) with:

```dockerfile
# The machine's own fonts, where it has them: in ~/.local/share/fonts, not
# a system folder. The panel and the box draw in Iosevka Term Extended, by
# name (src/panel/style.rs), and the action buttons' Font Awesome glyphs
# (src/actions.rs) fall back to JetBrainsMonoNL Nerd Font. The folder
# matters: e2e-panel.sh gives the nested session an XDG_DATA_HOME of its
# own and links this folder into it, as on the machine, so the panel's
# checks draw in these fonts and fail at once when they are not there.
```

Keep the next line, `# The machine's versions, from the same release zips.`, and the `RUN` below it unchanged.

- [ ] **Step 3: Rewrite the README sentence**

In `README.md`, replace

```
worktrunk and Claude Code in `~/.local/bin`, Iosevka Term (in
`~/.local/share/fonts`, as here, because the panel test's own
`XDG_DATA_HOME` hides that folder and its results depend on the font), the GTK
```

with

```
worktrunk and Claude Code in `~/.local/bin`, Iosevka Term and JetBrainsMono
Nerd Font (in `~/.local/share/fonts`, as here, which the panel test links into
its own `XDG_DATA_HOME` so its checks draw in the panel's font), the GTK
```

Then `grep -n -i 'fallback\|noto' README.md tests/e2e-panel.sh tests/container/Dockerfile` and fix any other line that still says the panel's checks draw in a fallback font (e2e-box and e2e-tag lines are out of scope).

- [ ] **Step 4: Check the image**

```bash
docker run --rm -i "$(bash tests/container/image.sh)" bash -s < tests/container/check-image.sh
```

`image.sh` builds the image when the Dockerfile changed (several minutes, needs the network). Expected: every check passes, including `e2e-panel's fonts link finds them`.

- [ ] **Step 5: Run e2e-panel in the container**

```bash
bash tests/container.sh bash tests/e2e-panel.sh
grep -n 'the nested session draws in Iosevka Term\|FAIL\|failed:' target/container-tests/latest.log
```

Expected: the font PASS line, `failed: 0`, exit 0 — with the same constants as the host. If a height check fails here but passed on the host, do not add a per-environment number: that is a parity gap between the image and the machine. Compare the kept frames in `target/container-tests/<stamp>-artifacts/` with the host's and report it to the user.

- [ ] **Step 6: Run the whole container suite**

```bash
bash tests/container.sh
tail -n 20 target/container-tests/latest.log
```

Expected: exit 0, no suite failed.

- [ ] **Step 7: Commit**

```bash
git add tests/container/Dockerfile tests/container/check-image.sh README.md
git commit -m "test(container): check the panel test's fonts link, not the fallback

e2e-panel now links the real fonts into its XDG_DATA_HOME, so the
image's check that another XDG_DATA_HOME falls back to Noto Sans, and
the Dockerfile and README lines explaining that fallback, no longer
describe what the panel's checks draw in.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
