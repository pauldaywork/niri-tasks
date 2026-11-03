# Size and Blur the Project List on General Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mod+Alt+W's project list has its blur under its rows on every workspace, including one where the panel was hidden when the list opened (general), as it already does where the panel was up.

**Architecture:** `Panel::fit` (`src/panel/surface.rs`) measures the tab bar, Clear all's strip and the footer only when `is_visible()` says they show. gtk4-rs's `is_visible()` is `gtk_widget_is_visible`, which is false when any ancestor is hidden, the window included. A render on a hidden panel runs `fit` before `present()` maps the window, so the bar and footer measure as 0px. The `Layout` is then built with `tabs_h: 0, footer_h: 0`, and the blur rectangles start the cards at the top of the surface, 37px above where GTK draws them. The rows sit on a 50px pitch, so the blur looks about 13px low, with sharp wallpaper inside each row. The fix reads each widget's own flag with `get_visible()` (`gtk_widget_get_visible`), in `fit` and in the three other places that compare a panel widget's visibility. A new `tests/e2e-panel.sh` check catches the bug: the nested niri draws the panel's blur grey, and the list opened on a hidden panel must be the same frame as the list drawn again once mapped.

**Tech Stack:** Rust, gtk4-rs 0.11, gtk4-layer-shell 0.8, niri 26.04 (`ext-background-effect-v1`), bash + Pillow for the e2e.

**Spec:** Taskwarrior task `8fb40487-1349-4b78-80b7-7e0153ec26b6`, its description and notes (`task rc.json.array=on 8fb40487 export`). No separate spec file.

## What was found (2026-11-03)

The spec's first step, "reproduce and find the cause", is done. The findings are recorded here so the executor does not redo them.

- **Reproduced** on eDP-1 (1920x1080, scale 1). `niritasks project open` was run on general, rep-counter and niri-tasks. On general, the sharp gap strips between rows fall inside the rows. On the other two they fall in the gaps. The size and centring of the list are the same on all three (the field at y 154, the footer at y 927). The visible fault is the blur's place, and the user's own good and bad screenshots show the same thing.
- **The daemon's measurements**, from temporary `eprintln!`s in `fit` (since removed):
  - general, panel hidden before Mod+Alt+W (no peek): `tabs_h: 0, bar_h: 0, footer_h: 0, cards_h: 800, win_h=0`. The first blur rectangle is at y 16, which is `SHADOW_PX`, the top of the surface.
  - rep-counter, panel up with a peeked card: `tabs_h: 37, bar_h: 29, footer_h: 35, cards_h: 728`.
  - The surface height is 832 both times, because the 800px cap takes up what the bar and footer lost. That is why the size looked right and only the blur moved.
- **Why general:** no task on general is peeked, so its panel is hidden when Mod+Alt+W arrives, and `render` calls `fit` before `present()`. The workspace name plays no part. Any workspace with nothing peeked does the same. So does Mod+Alt+Ctrl+T from a hidden panel, whose tab bar and footer go through the same `fit`.
- **Confirmed in gtk4-0.11** `src/auto/widget.rs`: `fn is_visible` calls `gtk_widget_is_visible`, and `fn get_visible` calls `gtk_widget_get_visible`.
- **No unit test** can cover this: `Layout` (`src/panel/layout.rs`) is right for what it is given, and the fault is in what GTK is asked. The test is the pixel e2e, which already opens the project list on a hidden panel ("Mod+Alt+W's project list, with no task").

## Global Constraints

- Fix it at the cause. Nothing special-cases the workspace name, a workspace index or general.
- Out of scope: the project list's content and behaviour, and any other styling.
- When it is done, the project list on general looks the same as on other workspaces (size, centring, blur), and Move to workspace and the task cards still look right.
- Commits: Conventional Commits, `fix(panel): …` / `test(panel): …`, subject ≤72 characters, body wrapped at 72, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run e2e suites against this worktree's build: `NIRITASKS=$PWD/target/debug/niritasks`. Without it they run the installed `niritasks`.
- Never `pkill -f` a pattern that appears in your own command line: it kills your own shell. Kill a daemon you started by its PID.

---

### Task 1: Measure the panel's bars by their own visibility

**Files:**
- Modify: `tests/e2e-panel.sh` (a grey-blur rule before `nested_start`, about line 192; a new check in the "Mod+Alt+W's project list, with no task" section, about lines 1051-1075)
- Modify: `src/panel/surface.rs:839-865` (`fit`), `:1311` (`sync`), `:1353` (`show_row`), `:1375` (`show_notes`)
- Modify: `README.md` (the `e2e-panel.sh` paragraph, about line 640)

**Interfaces:**
- Consumes: `NESTED_EXTRA_KDL` (from `tests/lib/nested-niri.sh`, appended to the nested niri's config by `nested_start`); the e2e helpers `shot`, `same`, `whereabouts`, `let_cursor_rest`, `ok`, `bad`, `NENV` already in `tests/e2e-panel.sh`.
- Produces: no new names. `fit`'s behaviour changes only for a render on a hidden window.

- [ ] **Step 1: Make the panel's blur show in the nested niri**

Over the nested niri's flat `#406080` background, a blur is just the background again, so a blur off the cards draws nothing to measure. Add a layer-rule that drops its saturation to 0, which turns it grey on the blue. In `tests/e2e-panel.sh`, immediately before the line `nested_start` (about line 192), insert:

```bash
# The panel's blur, drawn grey. The panel asks niri for blur behind its
# cards (src/panel/blur.rs), but over the flat background a blur is the
# background again, so blur in the wrong place would draw nothing to
# measure. With saturation 0 it is grey on the blue: blur off the cards
# shows in the gaps between them.
NESTED_EXTRA_KDL='layer-rule {
    match namespace="^niri-tasks-panel$"
    background-effect {
        saturation 0.0
    }
}'
```

- [ ] **Step 2: Write the failing check**

In the "Mod+Alt+W's project list, with no task" section, after the `if same baseline open_list; then … fi` block and before `"${NENV[@]}" wtype -k Escape`, insert:

```bash
    # Typed and taken back, the list draws again on a panel that is mapped
    # by now. Its first draw was measured with the window still hidden, as
    # on a workspace with nothing peeked. The field bar and the footer
    # measured 0px then, and the blur sat that far above the rows, grey in
    # the gaps between them (see NESTED_EXTRA_KDL). Both frames must match.
    "${NENV[@]}" wtype x
    sleep 1
    "${NENV[@]}" wtype -k BackSpace
    let_cursor_rest
    shot open_redrawn || { summary; exit 1; }
    if same open_list open_redrawn; then
        ok "the project list opened on a hidden panel has its blur under its rows, as when drawn again"
    else
        bad "the project list opened on a hidden panel differs from the same list drawn again ($(whereabouts open_redrawn open_list)): its blur is off its rows"
    fi
```

The next line is the section's existing `wtype -k Escape`. The field is empty again by then, so that Escape closes the list as before, and the existing "Escape closes the project list" check still applies.

Also add one sentence to the header comment's paragraph that starts "Because the screen is flat and fixed" (about lines 20-24), at its end:

```bash
# The panel's blur is drawn grey (NESTED_EXTRA_KDL below), so where it
# lies shows on the flat screen too.
```

- [ ] **Step 3: Run the e2e and see the new check fail**

Run:

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
```

Expected: every existing check still `ok`. The new one is `bad`, with something like:
`the project list opened on a hidden panel differs from the same list drawn again (columns 420-1180, rows …): its blur is off its rows`.

Interpreting the result:
- If the nested niri refuses the config, `nested_start` dies with niri's error in `$SB/niri.log`. Fix the KDL (niri 26.04's `background-effect` takes `saturation <float>`) and run again.
- If the new check passes now, the grey blur is not showing and the check proves nothing. Keep the frames (`NIRITASKS_E2E_KEEP=1`) and look at `open_list.png`. The rows should sit on grey, not blue. Do not go on to Step 4 until this check fails on the unfixed code.
- If an existing check breaks because of the grey blur, stop and report it with its message. Do not adjust its expected numbers to fit.

- [ ] **Step 4: Commit the check**

```bash
git add tests/e2e-panel.sh
git commit -m "$(cat <<'EOF'
test(panel): check the project list's blur on a hidden panel

The nested niri now draws the panel's blur grey, so blur that is off
the cards shows on its flat screen. The project list opened with no
task must be the same frame as the list drawn again once mapped. It
is not yet: the list's first draw measures its field bar and footer
as 0px, and its blur sits above its rows.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5: Measure by the widgets' own visibility in `fit`**

In `src/panel/surface.rs`, `fn fit`, replace the tab bar measurement:

```rust
        // Measured, like the cards, once in the window; margins included,
        // so this is the bar and the gap under it.
        let tabs_h = if self.tabs.is_visible() {
```

with:

```rust
        // Measured, like the cards, once in the window; margins included,
        // so this is the bar and the gap under it.
        //
        // get_visible(), the widget's own flag, here and below, never
        // is_visible(). is_visible() also asks every ancestor, the window
        // included, and a render on a hidden panel (a workspace with
        // nothing peeked) fits it before present() maps the window. The
        // bar and footer then measured 0px, and the blur sat that far
        // above the cards.
        let tabs_h = if self.tabs.get_visible() {
```

Then, in the same function, replace:

```rust
        let bar_shown = self.tabs.is_visible();
```
with:
```rust
        let bar_shown = self.tabs.get_visible();
```

and:

```rust
        let clear = if bar_shown && self.clear_strip.is_visible() {
```
with:
```rust
        let clear = if bar_shown && self.clear_strip.get_visible() {
```

and:

```rust
        let footer_h = if self.footer.is_visible() {
```
with:
```rust
        let footer_h = if self.footer.get_visible() {
```

- [ ] **Step 6: Do the same in the three other visibility comparisons**

Each compares a widget's visibility with what it should be. Each means the widget's own flag, and each misreads it while the window is hidden. In `src/panel/surface.rs`:

In `fn sync` (about line 1311), replace:
```rust
        if relabelled && self.clear_strip.is_visible() {
```
with:
```rust
        if relabelled && self.clear_strip.get_visible() {
```

In `fn show_row` (about line 1353), replace:
```rust
            if row.row.is_visible() != show {
```
with:
```rust
            if row.row.get_visible() != show {
```

In `fn show_notes` (about line 1375), replace:
```rust
                if notes.is_visible() != show {
```
with:
```rust
                if notes.get_visible() != show {
```

Leave `render`'s `if self.window.is_visible()` (about line 928) alone. A window has no ancestor, so there the two calls agree.

Then check that no widget `is_visible()` is left in the panel:

Run: `grep -n "is_visible()" src/panel/`
Expected: only `src/panel/surface.rs` with `self.window.is_visible()`.

- [ ] **Step 7: Run the unit tests**

Run: `cargo test`
Expected: all pass. Nothing in the unit tests depends on this.

- [ ] **Step 8: Run the e2e and see it pass**

Run:

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
```

Expected: every check `ok`, including `the project list opened on a hidden panel has its blur under its rows, as when drawn again`, and the run exits 0.

- [ ] **Step 9: Say in the README what the grey blur is for**

In `README.md`, in the paragraph that starts "`e2e-panel.sh` runs on a screen of its own", after the sentence ending "and your `niri-tasks` daemon keeps running throughout.", add:

```markdown
The panel's blur is drawn grey there (a layer-rule with saturation 0), so
blur off the cards shows on the flat screen.
```

- [ ] **Step 10: Commit the fix**

```bash
git add src/panel/surface.rs README.md
git commit -m "$(cat <<'EOF'
fix(panel): measure the bars by their own visibility

Mod+Alt+W on a workspace with nothing peeked, general among them,
opened the project list with its blur off its rows. The panel was
hidden, and fit() runs before present() maps it. is_visible() asks
every ancestor, the window included, so the field bar and footer
measured 0px and the blur started the rows 37px too high.
get_visible() reads the widget's own flag. The action row, notes and
Clear all checks made the same mistake and read the same flag now.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Check it by eye on the real desktop

The e2e proves the blur is where the rows are. The task's "done when" also asks that general look the same as other workspaces, and that Move to workspace and the task cards still look right. That needs the real wallpaper, so the worktree's build stands in for the installed daemon for a few minutes. This task has no commit.

**Files:** none.

**Interfaces:**
- Consumes: Task 1's build, `target/debug/niritasks`.

- [ ] **Step 1: Swap in this worktree's daemon**

```bash
cargo build
systemctl --user stop niri-tasks.service
./target/debug/niritasks daemon 2>/dev/null &
DAEMON_PID=$!
sleep 2
```

- [ ] **Step 2: Look at the project list on general and on another workspace**

On general, with nothing peeked, run `niritasks project open`, or press Mod+Alt+W there. Do the same on a workspace that has a peeked card. Take a screenshot of each with `niri msg action screenshot-screen -p false --path <scratchpad>/<name>.png` and close the list with Escape. Compare the two crops of the rows. The sharp wallpaper strips should be in the gaps between rows in both, with the field, rows and footer at the same places.

- [ ] **Step 3: Look at the task cards and Move to workspace**

On a workspace with tasks and nothing peeked, press Mod+Alt+Ctrl+T. The tab bar and every card should have its blur under it, and only the focused card shows its action row. Press `m` on a card and check the move list is blurred the same way, then Escape twice.

- [ ] **Step 4: Put the installed daemon back**

```bash
kill "$DAEMON_PID"
systemctl --user start niri-tasks.service
systemctl --user is-active niri-tasks.service
```

Expected: `active`.

- [ ] **Step 5: Show the user**

Show the user the screenshots from Step 2 side by side. Ask them to confirm general now looks like the other workspaces, before the branch is finished with the `finish-worktree` skill.
