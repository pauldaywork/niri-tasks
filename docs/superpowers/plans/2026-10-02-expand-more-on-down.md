# Expand "+N more" on Down — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the keyboard panel (Mod+Alt+Ctrl+T), pressing Down onto the "+N more" card shows the hidden cards straight away, focused on the first one it hid, with no Enter needed.

**Architecture:** One branch changes: `KeyAction::NextCard` in `Panel::key` (`src/panel/surface.rs`). When the card `keys::step` lands on is "+N more" (widget name `"more"`), it calls the existing `Panel::expand()` instead of `focus_card`. `expand()` already renders every card and focuses the first one that was hidden; the window's focus handler then shows that card's action row. The mouse path (the body's `connect_clicked` calling `expand()`) is untouched. The check is a new section of the nested-niri e2e test, `tests/e2e-panel.sh`, written first so it fails against today's code.

**Tech Stack:** Rust 2021, gtk4-rs, gtk4-layer-shell; bash + Pillow + wtype for the e2e test in a nested niri.

**Spec:** Taskwarrior task `664018db-9a2d-4dcd-b5ae-7e5469213753`. Read it with `task rc.json.array=on 664018db-9a2d-4dcd-b5ae-7e5469213753 export`. Its description and notes are the spec.

## Global Constraints

- Only the keyboard changes. In the NextCard branch, if the target card is "+N more" (widget name `"more"`), call `expand()` instead of `focus_card`. A mouse click on the card still expands it, as now.
- Focus lands on the first card that was hidden (what `expand()` already does), so the next Down keeps going down the list.
- An expanded list stays expanded until the keyboard is given back or the panel tucks away. Switching tab keeps it, as today. (Already true: `release_keyboard` and `tuck` clear `expanded`; `pick` does not.)
- Steps: change the NextCard branch in `Panel::key`. Update its doc comment and the panel module docs. Change the README's manual check "Enter on "+N more" shows the rest" (README.md:334, the spec's "line 319" has since moved) to say that Down reaching it shows the rest.
- Done when: on a tab with more than 8 tasks, Down from the 8th card shows every card and focuses the 9th. Clicking "+N more" on the hover panel still expands it. `cargo test` passes.
- Out of scope: hover/peek behaviour, the cap of 8 (`model::CAP`), and folding the list back up.
- House style: doc comments say *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Background the implementer needs

- `src/panel/surface.rs`, `Panel::key` (around line 808): collects the column's `task-card` children into `cards`, finds the focused one's index `at`, and for `PrevCard | NextCard` does `keys::step(at, cards.len(), forward)` (stops at the ends) then `focus_card(&cards[to], None)`.
- The "+N more" card is built by `card_widget` (around line 691) with `widget.set_widget_name(card.uuid.as_deref().unwrap_or("more"))` — every other card is named after its task's uuid. It is always the last card, so Down from the card above is the only key that reaches it. In keyboard mode it has no action row (it has no uuid).
- `Panel::expand()` (around line 499) sets `expanded`, calls `render()`, then focuses the `model::CAP`th (9th, index 8) child of the column. The focus handler (around line 405) calls `show_focused_row()`, so the 9th card's buttons show.
- Nothing in the Rust can drive a GTK keypress in a unit test; the panel's key handling is checked by `tests/e2e-panel.sh`, which presses keys with `wtype` inside a nested niri and measures the screen. It needs a niri session, `python3-pil`, taskwarrior and `wtype`. Run it from the worktree root: `bash tests/e2e-panel.sh`. `NIRITASKS_E2E_KEEP=1` keeps the frames. While it runs, keep off the workspace the nested window is parked on.
- In `tests/e2e-panel.sh`, at the end of the `if command -v wtype` branch (just before its `else`), the panel has been given back with Escape (the `parked` shot) and the workspace has two tasks on All plus one waiting ("ship it", off All). Helpers: `add "<desc>"`, `settle`, `shot <label>`, `measure <label> [against]` → `x0 x1 y0 y1` of what differs (against the empty baseline by default), `same <a> <b>`, `ok`, `bad`, `"${NENV[@]}" wtype -k Down`.

## File Structure

- Modify `src/panel/surface.rs` — the NextCard branch in `Panel::key`; the doc comments on `Panel::key`, the `expanded` field and the module's "The keyboard" section.
- Modify `src/panel/keys.rs` — the doc comment on `KeyAction::NextCard`.
- Modify `tests/e2e-panel.sh` — a new keyboard section for a list past the cap, and the header comment's list of what wtype checks.
- Modify `README.md` — the keyboard paragraph (around line 59) and the manual-check paragraph (around lines 322-335).

---

### Task 1: Down onto "+N more" shows the rest

**Files:**
- Modify: `tests/e2e-panel.sh` (header comment lines 32-36; new section before the `else` of `if command -v wtype`, after the `parked` check, around line 435)
- Modify: `src/panel/surface.rs` (module doc around lines 27-40; `expanded` field doc around line 152; `Panel::key` doc and NextCard branch around lines 800-850)
- Modify: `src/panel/keys.rs:18-19`
- Modify: `README.md` (around lines 59-66 and 322-335)

**Interfaces:**
- Consumes: `Panel::expand(self: &Rc<Self>)`, `focus_card(&gtk4::Widget, Option<&str>)`, `keys::step(usize, usize, bool) -> usize`, all existing in `src/panel/surface.rs` / `src/panel/keys.rs`.
- Produces: no new names.

- [ ] **Step 1: Write the failing e2e check**

In `tests/e2e-panel.sh`, insert this section right after the `parked` check's closing `fi` and before the `else` that skips the keyboard checks without wtype:

```bash

    # Past the cap: ten tasks on All show eight cards and "+2 more". Down from
    # the eighth card onto "+2 more" shows every card at once, focused on the
    # ninth: the panel grows by the two cards it hid, where focusing "+2 more"
    # itself would lose the eighth card's buttons and grow nothing.
    for n in 1 2 3 4 5 6 7 8; do add "past the cap $n"; done
    settle
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    for _ in 1 2 3 4 5 6 7; do "${NENV[@]}" wtype -k Down; sleep 0.2; done
    sleep 1
    shot long_eighth || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_eighth)
    eighth_h=$((y1 - y0))
    "${NENV[@]}" wtype -k Down
    sleep 1
    shot long_more || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_more)
    more_h=$((y1 - y0))
    if [ "$eighth_h" -gt 0 ] && [ "$more_h" -gt "$eighth_h" ]; then
        ok "Down onto \"+2 more\" shows every card (${eighth_h}px to ${more_h}px)"
    else
        bad "Down from the eighth card took the panel from ${eighth_h}px to ${more_h}px —
      it should grow by the two cards \"+2 more\" hid; shorter means it focused
      \"+2 more\" instead of showing them"
    fi
    # The focus is on the ninth card, not the last, so one more Down moves the
    # buttons to the tenth: the screen changes, the panel as tall as before.
    "${NENV[@]}" wtype -k Down
    sleep 1
    shot long_tenth || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_tenth)
    if ! same long_more long_tenth && [ "$((y1 - y0))" -eq "$more_h" ]; then
        ok "and focuses the first card it hid, so Down goes on to the next"
    else
        bad "after showing every card, Down changed nothing or the panel's height
      (${more_h}px to $((y1 - y0))px) — the focus was not on the ninth card"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
```

Also update the header comment (lines 34-36), replacing:

```bash
# middle of the screen, and with wtype, Down (which moves the action row), the
# filter tabs' keys and Escape are pressed in the nested niri, never on your
# desktop.
```

with:

```bash
# middle of the screen, and with wtype, Down (which moves the action row, and
# shows the rest on reaching "+N more"), the filter tabs' keys and Escape are
# pressed in the nested niri, never on your desktop.
```

The later "nothing pending shows nothing" section already completes every `+$TAG` task with `rc.bulk=0`, so the ten tasks need no cleanup of their own.

- [ ] **Step 2: Run the e2e test to verify the new check fails**

Run: `cargo build && bash tests/e2e-panel.sh`
Expected: every earlier check `ok`; the new one reports `bad "Down from the eighth card took the panel from …px to …px"` with the second number *smaller* than the first (today Down focuses "+2 more", whose card has no buttons, and the eighth card's row hides). The follow-up check also fails (Down on the last card changes nothing). If `wtype` is missing the whole keyboard block is skipped — install it (`sudo apt install wtype`) rather than going on blind.

If the script uses a release build or a different binary, check `tests/lib/nested-niri.sh` for how `$NIRITASKS` is found and build that one.

- [ ] **Step 3: Change the NextCard branch**

In `src/panel/surface.rs`, `Panel::key`, replace:

```rust
            KeyAction::PrevCard | KeyAction::NextCard => {
                let to = keys::step(at, cards.len(), action == KeyAction::NextCard);
                focus_card(&cards[to], None);
            }
```

with:

```rust
            KeyAction::PrevCard | KeyAction::NextCard => {
                let to = keys::step(at, cards.len(), action == KeyAction::NextCard);
                // "+N more" is always last, so only Down reaches it, and it
                // shows what it stands for there and then rather than waiting
                // on an Enter. expand() focuses the first card it hid, so the
                // next Down carries on down the list.
                if cards[to].widget_name() == "more" {
                    self.expand();
                } else {
                    focus_card(&cards[to], None);
                }
            }
```

- [ ] **Step 4: Update the doc comments**

`Panel::key`'s doc comment, replace its first sentence:

```rust
    /// Act on a key while the panel has the keyboard. Up and Down land on the
    /// next card's body, Left, Right and Tab move along the focused card, and
```

with:

```rust
    /// Act on a key while the panel has the keyboard. Up and Down land on the
    /// next card's body, or, Down reaching "+N more", show every card in its
    /// place, focused on the first it hid. Left, Right and Tab move along the
    /// focused card, and
```

(Keep the rest of that comment as it is; re-wrap the lines so they stay under the surrounding width.)

The `expanded` field's doc, replace:

```rust
    /// The "+N more" card was clicked: show every card until the panel tucks
    /// away again.
```

with:

```rust
    /// The "+N more" card was clicked, or Down reached it: show every card
    /// until the panel tucks away or gives the keyboard back.
```

The module doc's "The keyboard" section, replace:

```rust
//! lose the focused button. Up and Down move between cards, Left, Right and
//! Tab along the focused one, and s, r, e, t and Delete press its Start,
```

with:

```rust
//! lose the focused button. Up and Down move between cards, Down onto "+N
//! more" showing the cards it stands for and landing on the first of them;
//! Left, Right and Tab move along the focused one, and s, r, e, t and Delete
//! press its Start,
```

and re-wrap the following lines of that paragraph to the same width.

In `src/panel/keys.rs`, replace:

```rust
    /// Down: the card below, landing on its body.
    NextCard,
```

with:

```rust
    /// Down: the card below, landing on its body. Reaching "+N more", it
    /// shows every card instead, landing on the first that was hidden.
    NextCard,
```

- [ ] **Step 5: Update the README**

In the keyboard paragraph (around line 59), replace:

```markdown
Up and Down move a darker fill and the buttons between cards; Left, Right and
```

with:

```markdown
Up and Down move a darker fill and the buttons between cards, and Down onto
"+N more" shows the rest in its place, carrying on to the first of them; Left, Right and
```

and re-wrap that paragraph to the README's ~80-column lines.

In the manual-check paragraph (around line 333-334), replace:

```markdown
Delete deletes, while moving away disarms it; Enter on a card opens the menu,
Enter on "+N more" shows the rest, and a list taller than the screen scrolls
```

with:

```markdown
Delete deletes, while moving away disarms it; Enter on a card opens the menu,
Down reaching "+N more" shows the rest, focused on the first card it hid, and a list taller than the screen scrolls
```

and re-wrap. In the hover half of the same manual check (around line 325), after "while a click on a card opens its actions on that monitor", add ", and a click on "+N more" shows the rest" so the unchanged mouse path stays on the checklist.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test`
Expected: all pass (no Rust test changes; this guards the build and the llms.txt/README command-reference tests).

Run: `cargo build && bash tests/e2e-panel.sh`
Expected: every check `ok`, including `Down onto "+2 more" shows every card (…px to …px)` and `and focuses the first card it hid, so Down goes on to the next`, and the summary reports no failures.

- [ ] **Step 7: Check the mouse path by hand**

The e2e cannot move the pointer. With the installed daemon running the new build (or in the nested niri left open with `NIRITASKS_E2E_KEEP=1` if that is easier), put 9+ tasks on a workspace, hover the peek, click "+N more": every card shows, as before. Report this as checked or say plainly it was not.

- [ ] **Step 8: Commit**

```bash
git add src/panel/surface.rs src/panel/keys.rs tests/e2e-panel.sh README.md
git commit -m "$(cat <<'EOF'
Show the rest of the list when Down reaches "+N more" on the keyboard panel

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
