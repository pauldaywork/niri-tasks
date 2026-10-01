# Centre the Keyboard Task Panel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mod+Alt+Ctrl+T (`niritasks task panel`) shows the task panel in the middle of the focused monitor, while hover, clicks and the peek keep today's right-edge slide-out.

**Architecture:** The panel stays one gtk4-layer-shell surface per monitor (`src/panel/surface.rs`). While it holds the keyboard, it drops its `Edge::Right` anchor, so niri centres the unanchored surface on the monitor, and the cards jump, without sliding, to `CENTRED_X`, where their margins inside the surface are equal on both sides. The input region and the blur follow them. Giving the keyboard back re-anchors the surface on the right and snaps the cards to `TUCKED_X`. Hover enter does nothing while the panel is centred.

**Tech Stack:** Rust 2021, gtk4-rs 0.11, gtk4-layer-shell 0.8, bash and Pillow for the e2e test.

**Spec:** Taskwarrior task `6b57114f-af5c-4bc3-82f1-5fefcd980a5b`. Read it with `task rc.json.array=on 6b57114f-af5c-4bc3-82f1-5fefcd980a5b export`. Its description and notes are the spec.

## Global Constraints

- Only the keyboard path moves. `take_keyboard` centres the panel and `release_keyboard` returns it to the right-edge peek. Hover and the "+N more" click are unchanged.
- To centre it, drop the same surface's Right anchor while it holds the keyboard (the compositor centres an unanchored layer surface) and re-anchor it on release. Don't add a second window. The peek is gone while the panel is centred.
- No slide in the centre. The cards appear in place with equal margins on each side, and they snap back to the tucked peek on release. The input region and the blur follow them.
- Pressing the shortcut again while the panel is open does what it does today. Escape or any action closes it.
- Out of scope: making the shortcut close an open panel, changing hover behaviour, and the fuzzel fallback when there are no tasks.
- Done when: the shortcut shows the wrapped cards and their buttons centred on the focused monitor, holding the keyboard. Escape or a button returns to the right-edge peek. Hover and click behave exactly as before. `cargo test` and `tests/e2e-panel.sh` pass.
- House style: every item gets a doc comment that says *why*, in the voice of the surrounding code (plain sentences, no marketing, no "simply"). Commit messages are a plain-English sentence in the imperative, like `git log` shows, ending with the `Co-Authored-By` line.

## Background the implementer needs

- `SURFACE_WIDTH = SHADOW_PX + CARD_WIDTH_PX + EDGE_GAP_PX` = 16 + 760 + 8 = 784. The cards ride inside it at `slide.x`. `EXPANDED_X` = 16 (out, at the right edge) and `TUCKED_X` = 754 (the 30px peek).
- The only thing drawn outside a card is its 4px outline ring (`RING_PX`, `style::OUTLINE`). `SHADOW_PX` is room for that ring. So a card at `(784 - 760) / 2 = 12` has 12px each side, which is more than the ring needs. The surface keeps its width, and centring needs no resize.
- `set_region(x)` accepts input from `x` to the surface's right edge. `update_blur(x)` blurs `CARD_WIDTH_PX` from `x`. Both are already right at `x = CENTRED_X`, so neither changes. Only who calls them, and with what `x`, changes.
- `slide_to` animates `slide.x` from a tick callback. If a tick is already running when the panel is centred (it was hovered mid-slide), the tick has to land on the new position rather than drag the cards across. Setting `from`, `to` and `x` all to the target makes every remaining tick compute that same target.
- `niri msg action screenshot-screen` captures the focused monitor. `Request::Panel` (`src/daemon.rs:185`) takes the focused workspace's output, so in the e2e test the panel being measured and the screenshot are on the same monitor.

---

### Task 1: Centre the panel while it holds the keyboard

**Files:**
- Modify: `src/panel/surface.rs` (module doc lines 1–47; constants at 84–86; `motion.connect_enter` at 213–222; `take_keyboard` at 333–348; `release_keyboard` at 350–360; `render`'s no-cards path at 390–403; a new `jump_to` beside `slide_to`; tests at the bottom)

**Interfaces:**
- Consumes: nothing new.
- Produces: `const CENTRED_X: f64` and `fn jump_to(self: &Rc<Self>, x: f64)` in `surface.rs`, both private. Task 2 relies only on the on-screen behaviour: while the panel holds the keyboard, its cards are centred horizontally and vertically on the focused monitor and nothing shows on the right edge.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` at the bottom of `src/panel/surface.rs`:

```rust
    #[test]
    fn centred_cards_have_equal_margins_in_the_surface() {
        // The compositor centres the surface, so the cards are centred on the
        // screen only if they are centred inside it.
        let left = CENTRED_X as i32;
        let right = SURFACE_WIDTH - CARD_WIDTH_PX - left;
        assert_eq!(left, right);
    }

    #[test]
    fn centred_cards_leave_room_for_their_ring() {
        // Less than this and the surface cuts the outline ring off square.
        assert!(CENTRED_X as i32 >= RING_PX);
    }
```

- [ ] **Step 2: Run the tests to check they fail**

Run: `cargo test --lib panel::surface`
Expected: compile error, `cannot find value CENTRED_X in this scope`.

- [ ] **Step 3: Add the constant**

In `src/panel/surface.rs`, below `TUCKED_X`:

```rust
/// Where the cards sit while the panel has the keyboard and the surface is
/// unanchored, which niri centres on the monitor: the middle of the surface,
/// so the cards are in the middle of the screen. 784 less 760 leaves 12px each
/// side, enough for the ring.
const CENTRED_X: f64 = ((SURFACE_WIDTH - CARD_WIDTH_PX) / 2) as f64;
```

`SURFACE_WIDTH - CARD_WIDTH_PX` is 24, which is even, so the integer division loses nothing. The equal-margins test fails if a later change to the constants makes it odd.

- [ ] **Step 4: Run the tests to check they pass**

Run: `cargo test --lib panel::surface`
Expected: PASS, all surface tests, including the two new ones.

- [ ] **Step 5: Add `jump_to`**

Put it directly above `fn slide_to`:

```rust
    /// Put the cards at `x` at once, with no slide: into the middle when the
    /// panel takes the keyboard, and back to the peek when it gives it up. A
    /// slide already running ends here too, since every tick it has left
    /// works out `from + (to - from) * eased`, which is `x`.
    fn jump_to(&self, x: f64) {
        let slide = &self.slide;
        slide.x.set(x);
        slide.from.set(x);
        slide.to.set(x);
        if let Some(child) = self.window.child() {
            child.queue_allocate();
        }
        self.set_region(x);
        self.update_blur(x);
    }
```

- [ ] **Step 6: Centre in `take_keyboard`**

Replace the body of `take_keyboard` and its doc comment with:

```rust
    /// Take the keyboard in the middle of the monitor, every card wrapped with
    /// its buttons, focusing the first. False when there are no cards to take
    /// it for.
    ///
    /// Dropping the right anchor leaves the surface anchored to nothing, which
    /// niri centres on the monitor, both ways; the cards jump to the middle of
    /// the surface rather than slide across the screen. The peek goes with
    /// it: there is one surface, and it is in the middle now.
    pub fn take_keyboard(self: &Rc<Self>) -> bool {
        if self.all.borrow().is_empty() {
            return false;
        }
        self.keyboard.set(true);
        self.cancel_grace();
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.window.set_anchor(Edge::Right, false);
        self.render();
        // After render(), which measures the cards the region needs.
        self.jump_to(CENTRED_X);
        if let Some(first) = self.column.first_child() {
            focus_card(&first, None);
        }
        true
    }
```

- [ ] **Step 7: Return to the right edge in `release_keyboard`**

Replace `release_keyboard` with:

```rust
    /// Give the keyboard back, folding every card to its one line again and
    /// putting the panel back on the right edge, tucked away to its peek. The
    /// cards snap there rather than slide: they would have to cross half the
    /// screen.
    fn release_keyboard(self: &Rc<Self>) {
        if !self.keyboard.replace(false) {
            return;
        }
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.window.set_anchor(Edge::Right, true);
        self.expanded.set(false);
        self.render();
        self.jump_to(TUCKED_X);
    }
```

- [ ] **Step 8: Re-anchor in `render`'s no-cards path**

In `render`, the empty branch currently reads:

```rust
            if self.keyboard.replace(false) {
                self.window.set_keyboard_mode(KeyboardMode::None);
            }
```

Change it to:

```rust
            // The last task went while the panel had the keyboard: back to
            // the right edge, so the next card shows as a peek there.
            if self.keyboard.replace(false) {
                self.window.set_keyboard_mode(KeyboardMode::None);
                self.window.set_anchor(Edge::Right, true);
            }
```

Leave the lines after it (`cancel_grace`, `slide.x.set(TUCKED_X)`, `slide.to.set(TUCKED_X)`) as they are. They already put the cards at the peek without animating, and the surface is hidden.

- [ ] **Step 9: Keep hover from sliding the centred cards**

`motion.connect_enter` in `Panel::new` currently cancels the grace and slides to `EXPANDED_X`. Centred cards are at `x = 12`, so the slide would drag them 4px and then hold them at `EXPANDED_X` for the rest of the session. Replace that handler with:

```rust
        {
            let weak = Rc::downgrade(&panel);
            motion.connect_enter(move |_, _, _| {
                let Some(p) = weak.upgrade() else { return };
                // The keyboard's panel is in the middle and stays put.
                if p.keyboard.get() {
                    return;
                }
                p.cancel_grace();
                p.slide_to(EXPANDED_X);
            });
        }
```

`connect_leave` already returns early while `keyboard` is set, so it needs no change.

- [ ] **Step 10: Update the module doc**

In the `//!` doc at the top of `surface.rs`, change the first two lines to:

```rust
//! One monitor's task panel: a layer surface on the right edge, tucked away to
//! a peek until the pointer comes over it, or in the middle of the screen
//! while it has the keyboard.
```

In `## The keyboard`, replace the first sentence ("Mod+Alt+Ctrl+T slides the panel out and hands it the keyboard, and every card lays itself out again: …") with:

```rust
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! and every card lays itself out again: its whole description, wrapped, above
//! a row of buttons for the menu's most-used actions.
```

Then replace "Escape, or anything that runs, hands the keyboard back and folds the cards to one line again." with:

```rust
//! Escape, or anything that runs, hands the keyboard back, folds the cards to
//! one line again, and puts the panel back on the right edge as a peek.
```

Add this paragraph after the paragraph that ends "…stops at the view's edges.":

```rust
//! To sit in the middle, the surface drops its right anchor while it has the
//! keyboard, and niri centres a layer surface anchored to nothing. The cards
//! jump to the middle of the surface, equal margins either side, rather than
//! slide; the input region and blur follow them there and back. One surface
//! means no peek on the right edge while it is centred, and the pointer
//! coming over the centred cards does not slide them.
```

Rewrap all the `//!` lines you touched to the file's width, under 80 columns, the way the surrounding lines are wrapped.

- [ ] **Step 11: Build, test, lint**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: every test passes and there are no clippy warnings.

- [ ] **Step 12: Check it by hand on the real compositor**

Install the build and restart the daemon the way README's "Development" section says (`bash install.sh`). Then, on a named workspace with two or three tasks:

1. Press Mod+Alt+Ctrl+T. The wrapped cards with their buttons appear in the middle of the focused monitor, with no slide and no peek on the right edge. The arrow keys and letters work as before.
2. Move the pointer over the centred cards and off again. They do not move.
3. Press Escape. The panel is back at the right edge as the one-line peek, with no slide across the screen.
4. Press Mod+Alt+Ctrl+T, then `e`. The task box opens, and the panel is back at the right-edge peek.
5. Hover the peek. It slides out and, after about 0.4s off it, back. Click a card and its menu opens on that monitor, as before.

If step 1 shows the panel still on the right, gtk4-layer-shell did not commit the anchor change on a mapped surface. Stop and report it rather than working around it.

- [ ] **Step 13: Commit**

```bash
git add src/panel/surface.rs
git commit -m "Centre the task panel on the monitor while it has the keyboard

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Measure the centred panel in the e2e test, and document it

**Files:**
- Modify: `tests/e2e-panel.sh` (header comment lines 17–20; `measure()` at 117–153; the keyboard section at 201–243)
- Modify: `README.md` (Task panel, lines 43–44 and 58–60; the keybinding table line 19; Testing, lines 263–268)
- Modify: `CONTEXT.md` ("Task panel", lines 41–45)
- Modify: `niri/niri-tasks.kdl` (comment at lines 75–78)

**Interfaces:**
- Consumes: Task 1's behaviour. While the panel has the keyboard, its cards are centred on the focused monitor and the right edge shows nothing. After Escape, the one-line peek is back on the right edge.
- Produces: `measure <label> [right|centre]` in `e2e-panel.sh`. `right` (the default) prints `<width> <height>`, as today. `centre` prints `<width> <height> <left> <right>`, where `left` and `right` are the empty columns between the panel and the edges of the measured box.

- [ ] **Step 1: Let `measure` look at the middle of the screen**

Replace `measure()` and its comment in `tests/e2e-panel.sh` with:

```bash
# Compare a frame against the baseline and print "<width> <height>" of the
# panel: the span of columns, and of rows, that changed over most of a run.
# With "centre" it looks at the middle of the screen instead of the right edge,
# and adds "<left> <right>": the unchanged columns either side of the panel in
# that box, which match when the panel is centred.
#
# Counting changed pixels does not work: the cards are translucent, so much of
# them differs from the wallpaper by only a few levels, while a window
# redrawing under the strip differs by a lot. What separates them is shape. A
# card is a solid block, so every column through the peek changes down most of
# a card's height and every row through it changes across most of the peek,
# while noise changes a handful of pixels. Columns and rows over MIN_RUN are
# panel and nothing else.
measure() {
    python3 - "$SB/shots/baseline.png" "$SB/shots/$1.png" "${2:-right}" <<'PY'
import sys
from PIL import Image, ImageChops

SENSITIVITY = 6   # levels of difference that count as changed at all
MIN_RUN = 20      # changed pixels in a line before it is panel rather than noise

base = Image.open(sys.argv[1]).convert("RGB")
frame = Image.open(sys.argv[2]).convert("RGB")
region = sys.argv[3]
w, h = base.size
if region == "centre":
    # The middle half of the screen, both ways, and symmetric about its
    # centre, so a centred panel leaves equal gaps either side. Clear of the
    # screenshot notifications, which stack down from the top right corner.
    box = (w // 4, h // 4, w - w // 4, h - h // 4)
else:
    # The right edge, well wider than the peek so an overlong one is seen, and
    # a band across the middle, where a vertically centred panel sits.
    #
    # The band is kept narrow on purpose. Every screenshot this takes makes
    # niri post a "Screenshot captured" notification, and they stack down from
    # the top right — into the very strip being measured, by the third shot, if
    # it runs much above the middle. Three cards fit in this band with room to
    # spare.
    box = (w - 300, int(h * 0.36), w, int(h * 0.64))
mask = (ImageChops.difference(base.crop(box), frame.crop(box))
        .convert("L")
        .point(lambda p: 255 if p > SENSITIVITY else 0))
px = mask.load()
cw, ch = mask.size
cols = [x for x in range(cw) if sum(1 for y in range(ch) if px[x, y]) >= MIN_RUN]
rows = [y for y in range(ch) if sum(1 for x in range(cw) if px[x, y]) >= MIN_RUN]
width = (cols[-1] - cols[0] + 1) if cols else 0
height = (rows[-1] - rows[0] + 1) if rows else 0
if region == "centre":
    left = cols[0] if cols else 0
    right = (cw - 1 - cols[-1]) if cols else 0
    print(width, height, left, right)
else:
    print(width, height)
PY
}
```

The existing callers (`read -r one_w one_h < <(measure one)` and the rest) pass no second argument, so they still get exactly two fields.

- [ ] **Step 2: Measure the keyboard's panel in the middle**

Replace the keyboard section, from `# ─── the keyboard: every card out, wrapped, with its buttons ───` through the end of its `wtype` `if … fi` block, with:

```bash
# ─── the keyboard: every card wrapped, with its buttons, mid-screen ──────────
# `task panel` is the keybind's command: it asks this sandbox's daemon to hand
# its panel the keyboard. The panel leaves the right edge for the middle of the
# screen, so the right edge goes back to the baseline and the middle changes,
# by the same amount either side. Each card grows an action row, so the three
# of them stand taller than their one-line peek.
#
# A panel wider than the middle half of the screen (a small or scaled monitor)
# fills the box and leaves no gap either side, which still reads as centred.
"$NIRITASKS" task panel >/dev/null 2>&1
settle
shot keyboard || exit 1
read -r key_edge_w _ < <(measure keyboard)
read -r key_w key_h key_l key_r < <(measure keyboard centre)
if [ "$key_edge_w" -eq 0 ]; then
    ok "the keyboard takes the panel off the right edge"
else
    bad "the right edge still shows ${key_edge_w}px of panel with the keyboard —
      the surface kept its right anchor"
fi
off=$(( key_l > key_r ? key_l - key_r : key_r - key_l ))
if [ "$key_w" -gt 0 ] && [ "$off" -le 10 ]; then
    ok "and shows it in the middle of the screen (${key_w}px wide, ${key_l}px | ${key_r}px either side)"
else
    bad "the middle of the screen shows ${key_w}px of panel, ${key_l}px from the
      left of the box and ${key_r}px from its right — 0 wide means it is not
      there; uneven gaps mean it is not centred"
fi
if [ "$key_h" -ge $((three_h + 40)) ]; then
    ok "and each card grows its buttons (${key_h}px tall vs ${three_h}px)"
else
    bad "the keyboard's cards are ${key_h}px tall against ${three_h}px tucked away —
      the action rows are missing"
fi

# Escape needs a key pressed on the panel, which only wtype can do here. The
# next section gives the keyboard back regardless: a panel with no tasks lets
# it go.
if command -v wtype >/dev/null; then
    wtype -k Escape
    settle
    shot released || exit 1
    read -r rel_w rel_h < <(measure released)
    read -r rel_mid_w _ < <(measure released centre)
    delta=$(( rel_h > three_h ? rel_h - three_h : three_h - rel_h ))
    if [ "$rel_w" -le $((PEEK + 30)) ] && [ "$delta" -le 6 ]; then
        ok "Escape puts it back to the one-line peek (${rel_w}x${rel_h}px)"
    else
        bad "after Escape the right edge shows ${rel_w}x${rel_h}px, expected the
      ${three_w}x${three_h}px peek it started from"
    fi
    [ "$rel_mid_w" -eq 0 ] && ok "and leaves the middle of the screen clear" \
        || bad "after Escape the middle of the screen still shows ${rel_mid_w}px of panel"
else
    echo "  SKIP  Escape back to the peek (needs wtype)"
fi
```

- [ ] **Step 3: Update the script's header comment**

In the header of `tests/e2e-panel.sh`, replace:

```bash
# are checked by hand (README, "Testing"). The keyboard it can: `task panel`
# slides the panel out, and with wtype installed, Escape tucks it away again.
```

with:

```bash
# are checked by hand (README, "Testing"). The keyboard it can: `task panel`
# moves the panel to the middle of the screen, and with wtype installed, Escape
# puts it back on the right edge.
```

Also add "the middle of the screen" to the stillness note. Change "It needs the right edge of the screen to hold still" to "It needs the right edge and the middle of the screen to hold still". The stillness check compares only the right edge, so add this line after `read -r noise _ < <(measure stillness)`:

```bash
read -r mid_noise _ < <(measure stillness centre)
noise=$((noise + mid_noise))
```

Change the `bad` message there from "the right edge is not static" to "the right edge or the middle of the screen is not static".

- [ ] **Step 4: Run the e2e test**

Install the Task 1 build first, so the daemon the script starts is the new binary (README, "Development": `bash install.sh`). Park the pointer in a corner away from both the right edge and the middle. Keep the workspace in front of you holding still (an empty named workspace is best).

Run: `bash tests/e2e-panel.sh`
Expected: `failed: 0`. The keyboard section prints "the keyboard takes the panel off the right edge", "and shows it in the middle of the screen (…)", "and each card grows its buttons (…)", and, with wtype, "Escape puts it back to the one-line peek (…)" and "and leaves the middle of the screen clear".

If a check fails, rerun with `NIRITASKS_E2E_KEEP=1` and open the kept `keyboard.png` and `released.png` before changing anything.

- [ ] **Step 5: Update README.md**

In the keybinding table (line 19), change the start of the `Mod+Alt+Ctrl+T` row from "Hand the task panel the keyboard: every card shows its whole description" to "Hand the task panel the keyboard, in the middle of the screen: every card shows its whole description". Leave the rest of the row as it is.

In "## Task panel", replace:

```markdown
`Mod+Alt+Ctrl+T` slides the panel out and hands it the keyboard. Every card
opens up to its whole description, wrapped, above a row of buttons:
```

with:

```markdown
`Mod+Alt+Ctrl+T` hands the panel the keyboard and moves it to the middle of the
screen, leaving no peek on the right edge while it is there. Every card opens up
to its whole description, wrapped, above a row of buttons:
```

and replace:

```markdown
status and Move to workspace. Every button gives the keyboard back as it runs,
and Escape tucks the panel away, folding the cards back to one line. A list
```

with:

```markdown
status and Move to workspace. Every button gives the keyboard back as it runs,
and Escape puts the panel back on the right edge, tucked away with the cards
folded back to one line. A list
```

In "Testing", replace "`Mod+Alt+Ctrl+T` slides the panel out with every card wrapped and the first darkened" with "`Mod+Alt+Ctrl+T` shows the panel in the middle of the screen, without a slide, with every card wrapped and the first darkened, and the pointer passing over it does not move it". After "…Enter on "+N more" shows the rest, and a" the paragraph goes on unchanged.

- [ ] **Step 6: Update CONTEXT.md and niri-tasks.kdl**

In `CONTEXT.md`, replace the "Task panel" definition's body with:

```markdown
The list of task cards on a monitor's right edge, showing the pending tasks of
that monitor's active workspace. It is tucked away to a peek until hovered, or
until Mod+Alt+Ctrl+T hands it the keyboard, which moves it to the middle of the
screen until the keyboard is given back.
```

Keep the `_Avoid_` line as it is.

In `niri/niri-tasks.kdl`, replace:

```kdl
    // Mod+Alt+T writes a new task; its Ctrl companion, Mod+Alt+Ctrl+T, slides out
    // the task panel to pick one with the arrow keys, and Enter offers edit / note
    // / delete / complete / set-active on it. With no tasks on the workspace (or
    // no daemon) it opens the fuzzel list instead.
```

with:

```kdl
    // Mod+Alt+T writes a new task; its Ctrl companion, Mod+Alt+Ctrl+T, shows the
    // task panel in the middle of the screen to pick one with the arrow keys, and
    // Enter offers edit / note / delete / complete / set-active on it. With no
    // tasks on the workspace (or no daemon) it opens the fuzzel list instead.
```

- [ ] **Step 7: Check that no doc still says the keyboard slides the panel out**

Run: `grep -rn "slides the panel out\|slides out$\|slides out the task panel\|tucks it away again" README.md CONTEXT.md niri/ src/ tests/`
Expected: no line that describes the keyboard path. Lines about hover sliding the cards out stay as they are.

- [ ] **Step 8: Commit**

```bash
git add tests/e2e-panel.sh README.md CONTEXT.md niri/niri-tasks.kdl
git commit -m "Measure the keyboard's task panel in the middle of the screen, and document it there

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
