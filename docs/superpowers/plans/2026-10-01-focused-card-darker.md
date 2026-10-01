# Focused Task Card: Darker, Not Bordered — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The task card the keyboard is on in the task panel is marked by a darker fill instead of a 1px white border.

**Architecture:** The panel's whole look is one stylesheet string, `css()` in `src/panel/style.rs`. Today its `.task-card:focus` rule draws `outline: {FOCUS}`. Replace that with a `background-color` from a new constant, `FOCUSED_BACKGROUND`, a stronger black than the card's normal `BACKGROUND`. The `FOCUS` constant stays: the task box (`src/taskbox/style.rs`) still borders its focused field and buttons with it, and that is out of scope. Only its doc comment changes, since the panel no longer uses it.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK CSS).

**Spec:** Taskwarrior task `338dc898-d71b-49c3-85cb-a2c98646172e`. Read it with `task rc.json.array=on 338dc898-d71b-49c3-85cb-a2c98646172e export`. Its description is the whole spec: "when a task in the task list is highlighted just make the background darker and don't add a border to it".

## Global Constraints

- "The task list" is the **task panel** (see `CONTEXT.md`), and "highlighted" is the card the keyboard is on (`:focus`). The fuzzel picker already marks its selected row with brighter text and no border (`fuzzel/picker.ini`, `selection=00000000`), so it needs no change.
- The focused card gets a darker background and no border and no outline.
- Out of scope: the task box's focus border (`src/taskbox/style.rs`), hover styling, and the active/blocked/"+N more" text colours.
- Keep the `:focus` (not `:focus-visible`) comment's reasoning: GTK clears focus-visible 3s after the last key press, and the highlight would vanish mid-pick. It applies to the fill just as it did to the border.
- House style: every constant gets a doc comment that says *why*, in the voice of the surrounding code. The module doc in `src/panel/style.rs` describes the look in prose and must stay true.
- Done when: `cargo test` passes, `tests/all.sh` passes (suites that cannot run on the machine SKIP), and by eye the focused card is darker and has no white edge.

## Decisions made while planning (flag any you disagree with)

1. **`rgba(0, 0, 0, 0.6)` for the focused fill.** The card's normal fill is `rgba(0, 0, 0, 0.375)`. 0.6 is clearly darker over the blurred wallpaper while the text (`#f0f0f0`) still reads well. If it looks too subtle or too heavy in the visual check (Step 6), adjust the alpha there and in the test together; that is a taste call, not a design change.
2. **Hover stays as it is.** The spec says "highlighted", and in the panel that is the keyboard's card. Hovering only slides the panel out; it does not pick a card. A focused card that is also hovered stays dark, because the `:focus` rule comes after the `:hover` rule at the same specificity.
3. **`FOCUS` is kept, not moved.** Moving it into `src/taskbox/style.rs` would be tidier, but it is a refactor of code the spec does not touch. Reword its doc comment instead.

---

### Task 1: Darken the focused card and drop its border

**Files:**
- Modify: `src/panel/style.rs` (module doc lines 9-11, the `FOCUS` doc comment at line 55, a new constant after it, the `:focus` rule at lines 79-81, and the tests at lines 89-123)
- Modify: `README.md:43-45` and `README.md:250-251` (they describe the white border)
- Modify: `src/panel/surface.rs:28-29` (module doc says `:focus` "draws its border")
- Test: `src/panel/style.rs` (`mod tests`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub const FOCUSED_BACKGROUND: &str = "rgba(0, 0, 0, 0.6)";` in `crate::panel::style`. Nothing else uses it.

- [ ] **Step 1: Write the failing tests**

In `src/panel/style.rs`, in `cards_carry_makos_shape_and_the_terminals_font_and_colours`, replace the line

```rust
            ".task-card:focus { outline: 1px solid #ffffff; outline-offset: -1px; }",
```

with

```rust
            ".task-card:focus { background-color: rgba(0, 0, 0, 0.6); }",
```

Then add this test after `every_rule_is_scoped_to_the_panel`, inside `mod tests`:

```rust
    /// The card the keyboard is on is picked out by its fill alone. A border
    /// or outline there is the look this replaced.
    #[test]
    fn the_focused_card_is_darker_and_unbordered() {
        let css = css();
        let focus = css
            .lines()
            .find(|l| l.contains(".task-card:focus"))
            .expect("no :focus rule");
        assert!(!focus.contains("outline"), "focused card has an outline: {focus}");
        assert!(!focus.contains("border"), "focused card has a border: {focus}");
        assert_ne!(FOCUSED_BACKGROUND, BACKGROUND, "focused card is not picked out");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel::style`
Expected: does not compile, `cannot find value FOCUSED_BACKGROUND in this scope`. (That is the failure; once the constant exists, the first test would also fail on the missing `background-color` rule and the new one on `outline`.)

- [ ] **Step 3: Add the constant and change the rule**

In `src/panel/style.rs`, replace

```rust
/// The border on the card the keyboard is on.
pub const FOCUS: &str = "1px solid #ffffff";
```

with

```rust
/// The border on whatever the keyboard is on in the task box: a field or a
/// button. The panel's cards used to wear it too; they are darkened instead,
/// with [`FOCUSED_BACKGROUND`].
pub const FOCUS: &str = "1px solid #ffffff";
/// The fill of the card the keyboard is on: the same black as [`BACKGROUND`],
/// stronger, so the card stands out without a border drawn round it.
pub const FOCUSED_BACKGROUND: &str = "rgba(0, 0, 0, 0.6)";
```

In `css()`, replace

```rust
/* :focus, not :focus-visible: GTK clears focus-visible 3s after the last key
   press (VISIBLE_FOCUS_DURATION), and the border would vanish mid-pick. */
.task-panel .task-card:focus {{ outline: {FOCUS}; outline-offset: -1px; }}
```

with

```rust
/* :focus, not :focus-visible: GTK clears focus-visible 3s after the last key
   press (VISIBLE_FOCUS_DURATION), and the darker fill would vanish mid-pick. */
.task-panel .task-card:focus {{ background-color: {FOCUSED_BACKGROUND}; }}
```

The base rule above it already sets `border: none` and `outline: none` for every card, so nothing else draws an edge.

In the module doc at the top of the file, replace

```rust
//! No card has a border, except the one the keyboard is on, which gets a 1px
//! white one. The active task is marked by colour alone: its ▶ and its text
//! are green.
```

with

```rust
//! No card has a border. The one the keyboard is on is filled a darker black
//! than the rest. The active task is marked by colour alone: its ▶ and its
//! text are green.
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib panel::style`
Expected: 3 tests pass: `cards_carry_makos_shape_and_the_terminals_font_and_colours`, `every_rule_is_scoped_to_the_panel`, `the_focused_card_is_darker_and_unbordered`.

Then run: `cargo test --lib taskbox::style`
Expected: both tests pass, which shows the task box still has its `1px solid #ffffff` focus border.

- [ ] **Step 5: Update the prose that describes the border**

In `src/panel/surface.rs`, lines 28-29 of the module doc say `Enter clicks the focused one, and `:focus` draws its border.` Change `draws its border` to `darkens it`. Read the surrounding sentence first and keep its line wrapping at the file's width.

In `README.md`, replace

```markdown
takes the keyboard, the arrow keys move a 1px white border between cards, and
Enter acts as a click. Escape tucks it away again.
```

with

```markdown
takes the keyboard, the arrow keys move a darker fill between cards, and
Enter acts as a click. Escape tucks it away again.
```

and replace

```markdown
slides the panel out with the first card bordered, the arrows move the border,
```

with

```markdown
slides the panel out with the first card darkened, the arrows move the darkening,
```

Then check nothing else still describes the panel's border:

Run: `grep -rn -i "white border\|card bordered\|draws its border\|move the border" README.md CONTEXT.md src`
Expected: no output.

- [ ] **Step 6: Build, run every suite, and look at it**

Run: `cargo build --release 2>&1 | tail -1 && NIRITASKS=./target/release/niritasks bash tests/all.sh`
Expected: every suite that can run passes; ones whose prerequisites are missing report SKIP. `e2e-panel.sh` and `e2e-box.sh` take over the screen and keyboard, so leave the machine alone while they run.

Then the visual check, which no suite makes (the panel e2e test measures where the cards are, not their colour). Put a few tasks on the focused workspace, run the built daemon in place of the service, and drive the panel from the keyboard:

```bash
BIN=./target/release/niritasks
WAS=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
systemctl --user stop niri-tasks.service 2>/dev/null; sleep 1
"$BIN" daemon & D=$!; sleep 2
```

Press `Mod+Alt+Ctrl+T`. Check by eye: the first card is darker than the others and has no white edge; Down/Up move the darker fill; with the pointer resting on the focused card it stays dark. Press Escape. Then stop it and restore the service:

```bash
kill "$D"; wait "$D" 2>/dev/null
[ "$WAS" = active ] && systemctl --user start niri-tasks.service
```

If the darkening is too faint or too heavy, change the alpha in `FOCUSED_BACKGROUND` and in the test string from Step 1 together, then rerun Step 4 and this check.

- [ ] **Step 7: Commit**

```bash
git add src/panel/style.rs src/panel/surface.rs README.md
git commit -m "$(cat <<'EOF'
Mark the panel's focused card with a darker fill, not a white border

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
