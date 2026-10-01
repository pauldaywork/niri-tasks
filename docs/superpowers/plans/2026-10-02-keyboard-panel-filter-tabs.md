# Keyboard Panel Filter Tabs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While the task panel has the keyboard, a row of tabs above the cards narrows them to All, Active, Planned or To refine, each tab showing its count, picked with 1–4, `[` `]` or a click.

**Architecture:** What each tab matches is a pure `Filter` enum in `src/panel/model.rs`, beside the cards it filters, with each `Card` now carrying whether its task is `+planned` (its `Status` can hide that behind Active or Blocked). The keys that pick tabs are new `KeyAction` variants in `src/panel/keys.rs`. `src/panel/surface.rs` puts a tab strip above the scroller inside the overlay, visible only in keyboard mode, applies the filter before the cap, shows a "No … tasks" line for an empty tab, and counts the strip's height in the surface size, the scroller's cap, the input region and the blur.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK 4.22), gtk4-layer-shell 0.8, bash + Pillow + wtype for the e2e test in a nested niri.

**Spec:** Taskwarrior task `3e93db09-caac-40a3-9766-dd4ead5ace7d`. Read it with `task rc.json.array=on 3e93db09-caac-40a3-9766-dd4ead5ace7d export`. Its description and notes are the spec.

## Global Constraints

- Only the keyboard panel (Mod+Alt+Ctrl+T, centred) gets tabs. The hover slide-out, the peek and "+N more" stay exactly as they are.
- Four tabs, left to right: **All · Active · Planned · To refine**. Active = started (`Status::Active`). Planned = has `+planned` (`task::PLANNED_TAG`). To refine = no `+planned`. A tab is a filter, not a status: a started planned task shows under both Active and Planned.
- Each label carries its count, e.g. `Planned 3`.
- Keys: `1`–`4` pick a tab; `[` and `]` step to the one either side, stopping at the ends like `keys::step`. Clicking a tab works too.
- Tabs stay out of the focus chain: Up/Down/Left/Right/Tab still move only between cards and buttons.
- The panel opens on All every time. A refresh while open keeps the current tab, and focus falls back to the first card if the focused one left the filter.
- A tab with nothing in it shows a single "No … tasks" line and the panel keeps the keyboard. Only a workspace with no tasks at all takes `render()`'s hide-and-release path.
- The tab strip counts toward `shown_height`, the input region and the blur. The tab resets to All in `release_keyboard`.
- Update the `surface.rs` module doc; add "Filter tab" to `CONTEXT.md`.
- Done when: Mod+Alt+Ctrl+T shows the four tabs with counts above the centred cards; 1–4, `[` `]` and clicks switch them and the cards, focus and blur follow; an empty tab keeps the panel open; Escape and reopening start on All; hover is unchanged; `cargo test` and `tests/e2e-panel.sh` pass.
- Out of scope: tabs on the hover panel; telling quick-refined and grilled tasks apart; filtering the fuzzel picker.
- House style: every item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Decisions made while planning (flag any you disagree with)

1. **`Card` gains `planned: bool`.** `Status` is one value and Active and Blocked outrank Planned, so a started or blocked planned task's card no longer says it is planned. The Planned and To refine tabs need the tag itself. Active needs nothing new: `Status::Active` is exactly "started".
2. **The filter applies before the cap.** `model::cap(&filter.pick(all), CAP)`, so "+N more" counts the rest *of that tab*. Counts on the tabs are over every card, uncapped.
3. **The empty-tab lines:** All "No tasks" (unreachable while the keyboard is held, but total), Active "No active tasks", Planned "No planned tasks", To refine "No tasks to refine" ("No to refine tasks" does not read).
4. **The strip is one rounded bar the full card width**, in a card's fill, outline and font, its four tabs left-aligned inside, parted by the action row's faint lines. Unpicked tabs are dimmed text; the picked one is full `TEXT` on a faint white fill (`CURRENT_TAB`, `rgba(255, 255, 255, 0.12)`). One bar means one blur rectangle with the card's rounded corners.
5. **Geometry:** the overlay's child becomes a vertical box holding the strip and the scroller. The strip has `RING_PX` margin top, start and end and `GAP_PX - RING_PX` below, so its measured height (margins included) is the bar plus one card gap, called `tabs_h`, and the first card lands exactly `tabs_h` below where it lands today. Without the keyboard the strip is hidden and `tabs_h` is 0, so the hover layout is unchanged.
6. **The strip never scrolls.** `shown_height(cards_h, tabs_h, screen_h)` caps the *scroller* at the screen less its margins *and* the strip.
7. **Switching tab keeps `expanded`** ("+N more" opened) until the keyboard is given back, as today. Switching re-renders through the same focus-restore path a refresh takes, so the focused card keeps focus if the new tab has it and the first card gets it otherwise.
8. **The empty line is not a card.** It is a `gtk::Label` with class `filter-empty`, not `task-card`, and `key()` collects only `task-card` children, so Up/Down/Left/Right on an empty tab do nothing rather than index an empty slot list.
9. **The All reset also runs on `render()`'s hide-and-release path**, which drops the keyboard without going through `release_keyboard`. Otherwise the last task going while on Planned would reopen the next panel on Planned.
10. **The e2e test checks tabs with wtype in the nested niri:** `3` on a workspace with no planned tasks keeps the panel centred and shorter; `1` brings back the exact first keyboard frame; `[` at All changes nothing; `]` `]` reaches the same frame as `3`; Escape from a tab gives back the exact tucked frame; reopening gives back the exact first keyboard frame. Clicks on tabs cannot be driven (no pointer), so they join the README's hand checks.

## File map

| File | Change |
|---|---|
| `src/panel/model.rs` | `Card.planned`; new `Filter` enum: `TABS`, `label`, `tab_label`, `empty_text`, `matches`, `pick`, `counts`. |
| `src/panel/keys.rs` | `KeyAction::Filter(Filter)`, `PrevFilter`, `NextFilter`; `1`–`4`, `[`, `]`. |
| `src/panel/style.rs` | `CURRENT_TAB`; `.filter-tabs` and `.filter-empty` CSS. |
| `src/panel/surface.rs` | Tab strip, `filter` state, `pick()`, `update_tabs()`, `empty_line()`, filtered render, `tabs_h` in size/region/blur, key handling, resets, module doc. |
| `tests/e2e-panel.sh` | Tab checks in the keyboard section. |
| `README.md` | Keybind row, Task panel section, hand checks, e2e description. |
| `CONTEXT.md` | "Filter tab" entry. |

---

### Task 1: Filter tabs as data

**Files:**
- Modify: `src/panel/model.rs`

**Interfaces:**
- Consumes: `crate::task::Task::is_planned()` (exists, `src/task.rs:61`).
- Produces (Tasks 2 and 3 use these exact names):
  - `Card` gains `pub planned: bool`.
  - `pub enum Filter { All, Active, Planned, ToRefine }`: `Debug, Clone, Copy, PartialEq, Eq`.
  - `pub const Filter::TABS: [Filter; 4]` = `[All, Active, Planned, ToRefine]`.
  - `pub fn Filter::label(self) -> &'static str`
  - `pub fn Filter::tab_label(self, count: usize) -> String`
  - `pub fn Filter::empty_text(self) -> &'static str`
  - `pub fn Filter::matches(self, card: &Card) -> bool`
  - `pub fn Filter::pick(self, cards: &[Card]) -> Vec<Card>`
  - `pub fn Filter::counts(cards: &[Card]) -> [usize; 4]`, in `TABS` order.

- [ ] **Step 1: Write the failing tests**

Append these tests inside `mod tests` in `src/panel/model.rs` (the helpers `task`, `planned` and `texts` already exist there):

```rust
    #[test]
    fn a_card_knows_its_task_is_planned_whatever_its_status() {
        let got = cards(&[planned("started", true), planned("waits", false), task("plain", 1.0, false)], &["waits".into()]);
        assert_eq!(got[0].status, Status::Active);
        assert!(got[0].planned, "a started planned task is still planned");
        let waits = got.iter().find(|c| c.text == "waits").unwrap();
        assert_eq!(waits.status, Status::Blocked);
        assert!(waits.planned, "a blocked planned task is still planned");
        assert!(!got.iter().find(|c| c.text == "plain").unwrap().planned);
    }

    #[test]
    fn the_more_card_is_not_planned() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| planned(&format!("t{i}"), false)).collect();
        assert!(!cap(&cards(&many, &[]), CAP).last().unwrap().planned);
    }

    #[test]
    fn the_tabs_run_all_active_planned_to_refine() {
        let labels: Vec<&str> = Filter::TABS.iter().map(|f| f.label()).collect();
        assert_eq!(labels, vec!["All", "Active", "Planned", "To refine"]);
    }

    #[test]
    fn a_tab_label_carries_its_count() {
        assert_eq!(Filter::Planned.tab_label(3), "Planned 3");
        assert_eq!(Filter::ToRefine.tab_label(0), "To refine 0");
    }

    #[test]
    fn an_empty_tab_says_what_it_has_none_of() {
        assert_eq!(Filter::All.empty_text(), "No tasks");
        assert_eq!(Filter::Active.empty_text(), "No active tasks");
        assert_eq!(Filter::Planned.empty_text(), "No planned tasks");
        assert_eq!(Filter::ToRefine.empty_text(), "No tasks to refine");
    }

    /// A filter, not a status: a started planned task is under Active and
    /// Planned both, and not under To refine.
    #[test]
    fn each_tab_picks_its_tasks_in_order() {
        let all = cards(
            &[
                task("plain", 9.0, false),
                planned("started-planned", true),
                task("started", 5.0, true),
                planned("planned", false),
            ],
            &[],
        );
        assert_eq!(texts(&Filter::All.pick(&all)), texts(&all));
        assert_eq!(texts(&Filter::Active.pick(&all)), vec!["started", "started-planned"]);
        assert_eq!(texts(&Filter::Planned.pick(&all)), vec!["started-planned", "planned"]);
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["started", "plain"]);
    }

    #[test]
    fn counts_are_per_tab_in_tab_order() {
        let all = cards(
            &[task("plain", 9.0, false), planned("started-planned", true), planned("planned", false)],
            &[],
        );
        assert_eq!(Filter::counts(&all), [3, 1, 2, 1]);
        assert_eq!(Filter::counts(&[]), [0, 0, 0, 0]);
    }
```

Note the expected order in `each_tab_picks_its_tasks_in_order`: `cards()` puts active tasks first, then by urgency. The two active tasks are `started` (urgency 5.0) and `started-planned` (urgency 1.0 from the `planned` helper), so `started` comes first; then `plain` (9.0), then `planned` (1.0).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel::model`
Expected: compile errors — no field `planned` on `Card`, cannot find type `Filter`.

- [ ] **Step 3: Add `planned` to `Card`**

In `src/panel/model.rs`, add the field to `Card` after `uuid`:

```rust
    /// The task carries `+planned`. Apart from `status`, which shows a
    /// started or blocked planned task as Active or Blocked: the Planned and
    /// To refine tabs go by the tag, whatever the card's icon says.
    pub planned: bool,
```

In `cards()`, add `planned: t.is_planned(),` to the `Card { … }` literal after `uuid: Some(t.uuid.clone()),`. In `cap()`, add `planned: false,` to the "+N more" card's literal after `uuid: None,`.

- [ ] **Step 4: Add `Filter`**

In `src/panel/model.rs`, after `impl Card { … }` and before `pub fn cards`, add:

```rust
/// A filter tab on the keyboard's panel: which of the cards it shows.
///
/// A filter, not a status. A started planned task shows under both Active
/// and Planned, where its card's icon can only say one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    /// Started tasks.
    Active,
    /// Tasks carrying `+planned`, from Refine or Grill me.
    Planned,
    /// Tasks without `+planned`: the ones still worth refining.
    ToRefine,
}

impl Filter {
    /// The tabs left to right, which is also the order 1 to 4 pick them in.
    pub const TABS: [Filter; 4] = [Filter::All, Filter::Active, Filter::Planned, Filter::ToRefine];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Planned => "Planned",
            Filter::ToRefine => "To refine",
        }
    }

    /// The tab's label with how many tasks are under it, so a tab worth
    /// opening shows before it is opened.
    pub fn tab_label(self, count: usize) -> String {
        format!("{} {count}", self.label())
    }

    /// The one line a tab with nothing under it shows in place of cards.
    pub fn empty_text(self) -> &'static str {
        match self {
            Filter::All => "No tasks",
            Filter::Active => "No active tasks",
            Filter::Planned => "No planned tasks",
            Filter::ToRefine => "No tasks to refine",
        }
    }

    /// Whether this tab shows the card. Meant for the uncapped cards: the
    /// "+N more" card stands for no one task, so it is made after filtering.
    pub fn matches(self, card: &Card) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => card.status == Status::Active,
            Filter::Planned => card.planned,
            Filter::ToRefine => !card.planned,
        }
    }

    /// The cards this tab shows, in the order `cards` put them.
    pub fn pick(self, cards: &[Card]) -> Vec<Card> {
        cards.iter().filter(|c| self.matches(c)).cloned().collect()
    }

    /// How many of these cards each tab shows, in `TABS` order.
    pub fn counts(cards: &[Card]) -> [usize; 4] {
        Filter::TABS.map(|f| cards.iter().filter(|c| f.matches(c)).count())
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib panel::model`
Expected: PASS, every test including the existing ones.

Then: `cargo build`
Expected: builds. (`planned` was added to every `Card` literal; `grep -rn "Card {" src` should show only the two in `model.rs`.)

- [ ] **Step 6: Commit**

```bash
git add src/panel/model.rs
git commit -m "$(cat <<'EOF'
Give the panel's cards a filter for each tab: All, Active, Planned and To refine

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Keys for the tabs

**Files:**
- Modify: `src/panel/keys.rs`
- Modify: `src/panel/surface.rs` (one match arm, to keep it compiling; Task 3 wires it)

**Interfaces:**
- Consumes: `super::model::Filter` and `Filter::TABS` from Task 1.
- Produces (Task 3 uses these):
  - `KeyAction::Filter(Filter)`: pick this tab.
  - `KeyAction::PrevFilter`, `KeyAction::NextFilter`: the tab either side.
  - `key_action`: `1`/`2`/`3`/`4` → `Filter(Filter::TABS[0..4])`, `bracketleft` → `PrevFilter`, `bracketright` → `NextFilter`.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` in `src/panel/keys.rs`:

```rust
    /// 1 to 4 are the tabs left to right.
    #[test]
    fn numbers_pick_the_tabs_in_order() {
        let numbers = [gdk::Key::_1, gdk::Key::_2, gdk::Key::_3, gdk::Key::_4];
        for (key, filter) in numbers.into_iter().zip(Filter::TABS) {
            assert_eq!(key_action(key), KeyAction::Filter(filter), "{key:?}");
        }
    }

    #[test]
    fn brackets_step_between_tabs() {
        assert_eq!(key_action(gdk::Key::bracketleft), KeyAction::PrevFilter);
        assert_eq!(key_action(gdk::Key::bracketright), KeyAction::NextFilter);
    }

    /// Past the four tabs a number is nothing, not a fifth tab.
    #[test]
    fn other_numbers_pass_through() {
        for key in [gdk::Key::_0, gdk::Key::_5, gdk::Key::_9] {
            assert_eq!(key_action(key), KeyAction::Ignore, "{key:?}");
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel::keys`
Expected: compile errors — no variant `Filter`, `PrevFilter`, `NextFilter`; cannot find `Filter`.

- [ ] **Step 3: Add the variants and keys**

In `src/panel/keys.rs`, change the imports to:

```rust
use super::actions::Action;
use super::model::Filter;
use gtk4::gdk;
```

Add to `enum KeyAction`, after `Run(Action),`:

```rust
    /// 1 to 4: show this filter tab's cards.
    Filter(Filter),
    /// [ and ]: the tab either side, stopping at the ends as the cards do.
    PrevFilter,
    NextFilter,
```

Add to the `match` in `key_action`, before `_ => KeyAction::Ignore,`:

```rust
        gdk::Key::_1 => KeyAction::Filter(Filter::TABS[0]),
        gdk::Key::_2 => KeyAction::Filter(Filter::TABS[1]),
        gdk::Key::_3 => KeyAction::Filter(Filter::TABS[2]),
        gdk::Key::_4 => KeyAction::Filter(Filter::TABS[3]),
        gdk::Key::bracketleft => KeyAction::PrevFilter,
        gdk::Key::bracketright => KeyAction::NextFilter,
```

In the module doc comment of `keys.rs`, nothing changes.

- [ ] **Step 4: Keep `surface.rs` compiling**

In `src/panel/surface.rs`, `Panel::key()` ends its `match action` with `KeyAction::Release | KeyAction::Ignore => {}`. Change that arm to:

```rust
            // The tabs are Task 3's; until then their keys do nothing.
            KeyAction::Release
            | KeyAction::Ignore
            | KeyAction::Filter(_)
            | KeyAction::PrevFilter
            | KeyAction::NextFilter => {}
```

(Task 3 replaces this arm, so the comment does not survive.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib panel::keys && cargo build`
Expected: PASS, and it builds.

- [ ] **Step 6: Commit**

```bash
git add src/panel/keys.rs src/panel/surface.rs
git commit -m "$(cat <<'EOF'
Map 1 to 4, [ and ] to the panel's filter tabs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: The tab strip on the keyboard's panel

**Files:**
- Modify: `src/panel/style.rs`
- Modify: `src/panel/surface.rs`

**Interfaces:**
- Consumes: `Card.planned`, `Filter` and everything on it (Task 1); `KeyAction::{Filter, PrevFilter, NextFilter}` (Task 2); `keys::step(index, len, forward)`; `style::{GAP_PX, RADIUS_PX, CARD_WIDTH_PX}`.
- Produces: CSS classes `filter-tabs` (the strip), `current` (the picked tab's button), `filter-empty` (the empty tab's line) — Task 4's README and e2e describe them only by what they look like. `shown_height(cards_h, tabs_h, screen_h)` (signature change, private).

#### Style

- [ ] **Step 1: Write the failing style tests**

Append inside `mod tests` in `src/panel/style.rs`:

```rust
    /// The tab strip and an empty tab's line sit among the cards, so they
    /// wear a card's fill, rounding, font and outline.
    #[test]
    fn the_tabs_and_the_empty_line_look_like_cards() {
        let css = css();
        let rule = css
            .split('}')
            .find(|r| r.contains(".task-panel .task-card,") && r.contains("background-color"))
            .expect("no shared card rule");
        assert!(rule.contains(".task-panel .filter-tabs"), "the strip lacks the card look");
        assert!(rule.contains(".task-panel .filter-empty"), "the empty line lacks the card look");
    }

    /// The picked tab is full text on a faint fill; the rest are dimmed.
    #[test]
    fn the_picked_tab_stands_out() {
        let css = css();
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs button.current {{ color: {TEXT}; background-color: {CURRENT_TAB}; }}"
        )));
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs button {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); }}"
        )));
    }

    /// The tabs are buttons, so the theme's look is reset from them as from
    /// the action row's, hover and press included.
    #[test]
    fn the_tabs_lose_the_themes_button_look() {
        let css = css();
        assert!(css.contains(".task-panel .filter-tabs button {\n"));
        assert!(css.contains(".task-panel .filter-tabs button:hover"));
        assert!(css.contains(".task-panel .filter-tabs button:active"));
        let line = ".task-panel .filter-tabs button:not(:first-child) { border-left: 1px solid rgba(255, 255, 255, 0.12); }";
        let at = css.find(line).expect("missing the line between tabs");
        let reset = css.find(".filter-tabs button:active").unwrap();
        assert!(at > reset, "the press reset's `border: none` would win over the line");
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib panel::style`
Expected: compile error, cannot find `CURRENT_TAB`.

- [ ] **Step 3: Add the CSS**

In `src/panel/style.rs`, after `FOCUSED_BACKGROUND`, add:

```rust
/// The fill of the filter tab that is picked: a faint white, so it reads as
/// lit against the strip without taking a colour from the action buttons.
pub const CURRENT_TAB: &str = "rgba(255, 255, 255, 0.12)";
```

In `css()`, change the opening card rule's selector from `.task-panel .task-card {{` to three lines:

```
.task-panel .task-card,
.task-panel .filter-tabs,
.task-panel .filter-empty {{
```

Change the button-reset rule's selector from

```
.task-panel .card-body,
.task-panel .card-actions button {{
```

to

```
.task-panel .card-body,
.task-panel .card-actions button,
.task-panel .filter-tabs button {{
```

Change the hover-and-press reset's selector list to end:

```
.task-panel .card-actions button:hover,
.task-panel .card-actions button:active,
.task-panel .filter-tabs button:hover,
.task-panel .filter-tabs button:active {{
```

Then, directly after the existing line
`.task-panel .card-actions button:not(:first-child) {{ border-left: 1px solid {SEPARATOR}; }}`, add:

```
/* The filter tabs: dimmed, apart from the one that is picked, and parted
   like the action row's buttons. After the resets, which clear borders and
   fills. */
.task-panel .filter-tabs button {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); }}
.task-panel .filter-tabs button:not(:first-child) {{ border-left: 1px solid {SEPARATOR}; }}
.task-panel .filter-tabs button.current {{ color: {TEXT}; background-color: {CURRENT_TAB}; }}
/* An empty tab's one line, padded like a card's text and dimmed like
   "+N more". */
.task-panel .filter-empty {{ padding: {PADDING_PX}px; color: alpha({TEXT}, 0.55); }}
```

Add a sentence to `style.rs`'s module doc, after the paragraph that ends "carries the fill, the rounding and the outline.":

```rust
//! While the panel has the keyboard, the filter tabs above the cards are one
//! bar in a card's look, and a tab with nothing in it shows one line in a
//! card's look too.
```

- [ ] **Step 4: Run the style tests**

Run: `cargo test --lib panel::style`
Expected: PASS, including the existing `every_rule_is_scoped_to_the_panel` and `hover_and_press_keep_each_buttons_own_look`.

#### Geometry helpers

- [ ] **Step 5: Write the failing surface tests**

In `src/panel/surface.rs`'s `mod tests`, change the two existing `shown_height` tests to pass `0` tabs:

```rust
    #[test]
    fn a_short_column_shows_whole() {
        assert_eq!(shown_height(400, 0, 1080), 400);
    }

    #[test]
    fn a_tall_column_stops_at_the_screens_margins() {
        // 1080 less the shadow room and the edge gap, top and bottom.
        assert_eq!(shown_height(5000, 0, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX));
    }
```

and append:

```rust
    /// The tab strip does not scroll, so the cards get the screen less it.
    #[test]
    fn the_tabs_take_their_height_off_the_cards() {
        assert_eq!(shown_height(400, 50, 1080), 400, "a short column still shows whole");
        assert_eq!(shown_height(5000, 50, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX) - 50);
    }

    /// The strip's bottom margin is the card gap less the ring the column
    /// keeps above the first card; less than none would not build.
    #[test]
    fn the_gap_under_the_tabs_leaves_room_for_the_ring() {
        assert!(GAP_PX >= RING_PX);
    }
```

- [ ] **Step 6: Run them to verify they fail**

Run: `cargo test --lib panel::surface`
Expected: compile error, `shown_height` takes 2 arguments but 3 were supplied.

- [ ] **Step 7: Change `shown_height`**

Replace `shown_height` at the bottom of `src/panel/surface.rs` (above `scroll_to_show`) with:

```rust
/// How tall the column of cards is on screen: all of it, or as much as fits
/// inside the screen's margins under the `tabs_h` the filter tabs take, with
/// the rest scrolled. Only the keyboard's wrapped cards, or "+N more" opened
/// onto a long list, get that tall.
fn shown_height(cards_h: i32, tabs_h: i32, screen_h: i32) -> i32 {
    cards_h.min(screen_h - 2 * (SHADOW_PX + EDGE_GAP_PX) - tabs_h).max(0)
}
```

Change the one caller in `render()` to `shown_height(cards_h, 0, self.monitor.geometry().height())` for now (Step 10 passes the real `tabs_h`).

Change the style import at the top of `surface.rs` to also bring in `GAP_PX`, which it already has: `use super::style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX};` — unchanged. Change the model import to `use super::model::{self, Card, Filter, Status};`.

Run: `cargo test --lib panel::surface`
Expected: PASS.

#### The strip, the filter and the empty line

- [ ] **Step 8: Add the state**

In `struct Panel`, after `scroller`, add:

```rust
    /// The filter tabs above the scroller, shown only while the panel has the
    /// keyboard. It does not scroll with the cards.
    tabs: gtk4::Box,
    /// One button per filter tab, in `Filter::TABS` order, for their labels
    /// and which one is picked.
    tab_buttons: Vec<gtk4::Button>,
```

and after `keyboard: Cell<bool>,`:

```rust
    /// The filter tab the keyboard's panel shows. All whenever the panel
    /// takes the keyboard; kept across a refresh while it has it.
    filter: Cell<Filter>,
```

In `struct Slide`, after `cards_h`, add:

```rust
    /// The filter tabs' height, with the gap under them: what the cards sit
    /// below. 0 without the keyboard, which has no tabs.
    tabs_h: Cell<i32>,
```

- [ ] **Step 9: Build the strip in `new()`**

In `Panel::new`, replace the line `overlay.add_overlay(&scroller);` with:

```rust
        // The filter tabs, over the scroller rather than in it, so they stay
        // put while the cards scroll. Ring room on three sides, as the column
        // keeps, and under them the card gap less the ring the column keeps
        // above the first card: the first card then sits a card gap below.
        let tabs = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        tabs.add_css_class("filter-tabs");
        tabs.set_margin_top(RING_PX);
        tabs.set_margin_start(RING_PX);
        tabs.set_margin_end(RING_PX);
        tabs.set_margin_bottom(GAP_PX - RING_PX);
        // Clips the picked tab's fill to the bar's rounded corners.
        tabs.set_overflow(gtk4::Overflow::Hidden);
        tabs.set_visible(false);
        let tab_buttons: Vec<gtk4::Button> = Filter::TABS
            .iter()
            .map(|_| {
                let button = gtk4::Button::new();
                // Out of the focus chain: the arrows and Tab stay between the
                // cards and their buttons, and a click leaves the focus where
                // it was.
                button.set_focusable(false);
                button.set_focus_on_click(false);
                tabs.append(&button);
                button
            })
            .collect();
        scroller.set_vexpand(true);
        let front = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        front.append(&tabs);
        front.append(&scroller);
        overlay.add_overlay(&front);
```

In the `Panel { … }` literal, add `tabs,` and `tab_buttons,` after `scroller,`; `filter: Cell::new(Filter::All),` after `keyboard: Cell::new(false),`; and `tabs_h: Cell::new(0),` after `cards_h: Cell::new(0),` in the `Slide` literal.

Change the overlay's `connect_get_child_position` rectangle's height from `slide.cards_h.get() + 2 * RING_PX` to:

```rust
                    slide.tabs_h.get() + slide.cards_h.get() + 2 * RING_PX,
```

After that `connect_get_child_position` block, add:

```rust
        for (button, filter) in panel.tab_buttons.iter().zip(Filter::TABS) {
            let weak = Rc::downgrade(&panel);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.pick(filter);
                }
            });
        }
```

- [ ] **Step 10: Filter in `render()`**

Replace `render()`'s beginning, from `fn render(self: &Rc<Self>) {` up to and including the `for card in &cards { … }` loop, with:

```rust
    fn render(self: &Rc<Self>) {
        if self.all.borrow().is_empty() {
            // The last task went while the panel had the keyboard: back to
            // the right edge, so the next card shows as a peek there, and the
            // next Mod+Alt+Ctrl+T opens on All, as after Escape.
            if self.keyboard.replace(false) {
                self.window.set_keyboard_mode(KeyboardMode::None);
                self.filter.set(Filter::All);
            }
            // Only hide something that is up; hiding a never-mapped layer
            // surface leaves it deaf to a later present().
            if self.window.is_visible() {
                self.window.set_visible(false);
            }
            self.cancel_grace();
            self.jump_to(TUCKED_X, 0);
            return;
        }

        let keyboard = self.keyboard.get();
        // Only the keyboard's panel has tabs; the peek and the hover show
        // every card. Filtered before the cap, so "+N more" is the rest of
        // this tab.
        let filter = if keyboard { self.filter.get() } else { Filter::All };
        let picked = filter.pick(&self.all.borrow());
        let cards = if self.expanded.get() {
            picked
        } else {
            model::cap(&picked, model::CAP)
        };

        // A refresh while the keyboard is on the panel keeps focus on the same
        // task, and on the same button of it: each card is named after its
        // task's uuid, and each button after its action.
        let focused = GtkWindowExt::focus(&self.window)
            .and_then(|w| Some((self.card_of(&w)?.widget_name(), w.widget_name())));
        // The buttons go with the widgets they were armed on.
        self.armed.replace(None);
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        for card in &cards {
            self.column.append(&self.card_widget(card));
        }
        if cards.is_empty() {
            // A tab with nothing under it says so and keeps the keyboard:
            // only a workspace with no tasks at all hides the panel, above.
            self.column.append(&empty_line(filter));
        }
```

(The old `if cards.is_empty() { … return; }` block is now the `self.all.borrow().is_empty()` block at the top; delete the old one.)

Leave the `heights` measuring loop as it is: it measures every column child, so the empty line gets its blur too.

Replace the block from `// The column's margins are in what it measures` through `self.slide.cards_h.set(shown);` and `let height = shown + 2 * SHADOW_PX;` with:

```rust
        self.tabs.set_visible(keyboard);
        self.update_tabs();
        // Measured, like the cards, once in the window; margins included,
        // so this is the bar and the gap under it.
        let tabs_h = if keyboard {
            self.tabs.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX).1
        } else {
            0
        };

        // The column's margins are in what it measures, and in the width it
        // is measured for; the cards' height is without them.
        let (_, with_ring, _, _) =
            self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX);
        let cards_h = with_ring - 2 * RING_PX;
        let shown = shown_height(cards_h, tabs_h, self.monitor.geometry().height());
        self.slide.tabs_h.set(tabs_h);
        self.slide.cards_h.set(shown);
        let height = tabs_h + shown + 2 * SHADOW_PX;
```

The rest of `render()` (size requests, `present()`, region, blur, focus restore) stays. Its focus restore already falls back to the first column child when the focused card is not in this tab; on an empty tab that child is the empty line, and `focus_card` finds no slots in it and focuses nothing, which is right.

- [ ] **Step 11: Add `pick`, `update_tabs` and `empty_line`**

Add these methods to `impl Panel`, after `expand()`:

```rust
    /// Show the cards under this filter tab. Focus stays on the card it was
    /// on when the tab has it, and goes to the first card otherwise, as on a
    /// refresh. Nothing without the keyboard, whose panel has no tabs.
    fn pick(self: &Rc<Self>, filter: Filter) {
        if !self.keyboard.get() || self.filter.replace(filter) == filter {
            return;
        }
        self.render();
    }

    /// Each tab's label with its count over every card, and which one is
    /// picked.
    fn update_tabs(&self) {
        let counts = Filter::counts(&self.all.borrow());
        for ((button, filter), count) in self.tab_buttons.iter().zip(Filter::TABS).zip(counts) {
            button.set_label(&filter.tab_label(count));
            if filter == self.filter.get() {
                button.add_css_class("current");
            } else {
                button.remove_css_class("current");
            }
        }
    }
```

And add this free function after `card_label`:

```rust
/// The one line a filter tab with nothing under it shows where its cards
/// would be, in a card's look so it reads as part of the panel. Not a card:
/// it has no task and nothing to focus, and the keys pass over it.
fn empty_line(filter: Filter) -> gtk4::Label {
    let line = gtk4::Label::new(Some(filter.empty_text()));
    line.add_css_class("filter-empty");
    line.set_xalign(0.0);
    line.set_size_request(CARD_WIDTH_PX, -1);
    line
}
```

- [ ] **Step 12: Reset to All on giving the keyboard back**

In `release_keyboard`, after `self.expanded.set(false);`, add:

```rust
        // The next Mod+Alt+Ctrl+T opens on All.
        self.filter.set(Filter::All);
```

#### Keys, region and blur

- [ ] **Step 13: Handle the tab keys in `key()`**

In `Panel::key`, replace

```rust
        if action == KeyAction::Release {
            self.release_keyboard();
            return;
        }
```

with

```rust
        match action {
            KeyAction::Release => return self.release_keyboard(),
            KeyAction::Filter(filter) => return self.pick(filter),
            KeyAction::PrevFilter | KeyAction::NextFilter => {
                let at = Filter::TABS.iter().position(|f| *f == self.filter.get()).unwrap_or(0);
                let to = keys::step(at, Filter::TABS.len(), action == KeyAction::NextFilter);
                return self.pick(Filter::TABS[to]);
            }
            _ => {}
        }
```

Change the card-collecting loop just below it to skip the empty line:

```rust
        while let Some(c) = child {
            child = c.next_sibling();
            // Not an empty tab's line, which has nothing to focus.
            if c.has_css_class("task-card") {
                cards.push(c);
            }
        }
```

Replace the temporary arm Task 2 added at the end of the `match action` with:

```rust
            KeyAction::Release
            | KeyAction::Ignore
            | KeyAction::Filter(_)
            | KeyAction::PrevFilter
            | KeyAction::NextFilter => {}
```

Extend `key()`'s doc comment with one sentence at the end: `1 to 4, [ and ] pick a filter tab instead, whatever has focus.`

- [ ] **Step 14: Count the strip in the input region**

In `set_region`, replace the `let rect = …` line with:

```rust
        // From the top of the tabs, when there are any, to the bottom of the
        // cards on screen.
        let height = self.slide.tabs_h.get() + self.slide.cards_h.get();
        let rect = cairo::RectangleInt::new(x, SHADOW_PX, width, height);
```

- [ ] **Step 15: Count the strip in the blur**

Replace the body of `update_blur` from `// The cards as laid out in the column` to the end with:

```rust
        let tabs_h = self.slide.tabs_h.get();
        let mut rects = Vec::new();
        if tabs_h > 0 {
            // The tab bar, which does not scroll; not the card gap under it.
            rects.extend(blur::card_region((x, SHADOW_PX, width, tabs_h - GAP_PX), RADIUS_PX, on_screen));
        }
        // The cards as laid out in the column under the tabs, moved up by
        // however far it is scrolled, and cut to the part of the column on
        // screen.
        let scrolled = self.scroller.vadjustment().value().round() as i32;
        let top = SHADOW_PX + tabs_h;
        let mut y = top - scrolled;
        let mut cards = Vec::new();
        for &h in self.heights.borrow().iter() {
            cards.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        rects.extend(blur::clip_rows(&cards, top, top + self.slide.cards_h.get()));
        blur.set(&rects);
```

- [ ] **Step 16: Update the module doc**

In `surface.rs`'s module doc, after the paragraph ending "…puts the panel back on the right edge as a peek." and before "Wrapped, the cards can stand taller than the screen…", add:

```rust
//! Above the cards, a bar of filter tabs, All, Active, Planned and To refine,
//! each with its count, narrows them to the tasks it names. 1 to 4 pick one,
//! [ and ] step along them stopping at the ends, and a click picks one too.
//! The tabs never take focus, so the arrows and Tab still move only between
//! cards and buttons. The panel takes the keyboard on All every time; a
//! refresh keeps the tab, and focus falls back to the first card when the
//! one it was on has left it. A tab with nothing under it shows one line
//! saying so and keeps the keyboard: only a workspace with no tasks at all
//! hides the panel. The hover and the peek have no tabs.
//!
```

And change the next paragraph's first sentence to: `Wrapped, the cards can stand taller than the screen, so the column sits in a scroller under the tabs, capped at the screen's height less its margins and the tabs.`

- [ ] **Step 17: Run every test and build**

Run: `cargo test && cargo build --release`
Expected: all pass; release build succeeds with no new warnings (`cargo build --release 2>&1 | grep -c warning` matches the count before this task, normally 0).

- [ ] **Step 18: Check it by hand against the worktree's build**

The keybind's `niritasks task panel` reaches whichever daemon is listening, so run this worktree's:

```bash
systemctl --user stop niri-tasks
./target/release/niritasks daemon &
```

On a workspace with a few tasks (at least one `+planned`, one started), press Mod+Alt+Ctrl+T and check: four tabs with counts above the centred cards, All picked; `2`/`3`/`4` change the cards and the picked tab, with focus on the first card when the focused one left; `[` at All and `]` at To refine do nothing; clicking a tab switches it and leaves the darkened card where it was; a tab with nothing under it shows its one line and Up/Down/Escape still work; Escape, then Mod+Alt+Ctrl+T again, opens on All; hovering the peek shows no tabs. Then:

```bash
kill %1
systemctl --user start niri-tasks
```

- [ ] **Step 19: Commit**

```bash
git add src/panel/style.rs src/panel/surface.rs
git commit -m "$(cat <<'EOF'
Show filter tabs above the keyboard panel's cards, picked with 1 to 4, [ ] or a click

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: The e2e test and the docs

**Files:**
- Modify: `tests/e2e-panel.sh`
- Modify: `README.md`
- Modify: `CONTEXT.md`

**Interfaces:**
- Consumes: the keys from Task 2 and the panel from Task 3. The test's three tasks (`ship it`, `write the glossary`, the long one) are none started and none `+planned`, so Planned (`3`) is empty and All, To refine show all three.

- [ ] **Step 1: Keep the first keyboard frame's height**

In `tests/e2e-panel.sh`, in the keyboard section, change

```bash
read -r x0 x1 _ _ < <(measure keyboard)
```

to

```bash
read -r x0 x1 y0 y1 < <(measure keyboard)
keyboard_h=$((y1 - y0))
```

- [ ] **Step 2: Add the tab checks**

In the `if command -v wtype >/dev/null; then` branch, before the `"${NENV[@]}" wtype -k Down` line, insert:

```bash
    # 3 is the Planned tab, and none of these tasks is planned: the panel
    # keeps the keyboard in the middle, with the tabs and one line saying so,
    # shorter than the three cards it had.
    "${NENV[@]}" wtype 3
    sleep 1
    shot tab_planned || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure tab_planned)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ] &&
        [ "$((y1 - y0))" -lt "$keyboard_h" ]; then
        ok "an empty tab keeps the panel in the middle, shorter (${keyboard_h}px to $((y1 - y0))px)"
    else
        bad "on the empty Planned tab the panel covers columns ${x0}-${x1}, rows ${y0}-${y1},
      against ${keyboard_h}px tall on All — 0-0 means it gave up the keyboard"
    fi

    # 1 is All again, with focus back on the first card: the first frame.
    "${NENV[@]}" wtype 1
    sleep 1
    shot tab_all || { summary; exit 1; }
    if same keyboard tab_all; then
        ok "1 brings back All, focus on the first card, as the panel opened"
    else
        bad "after 3 then 1 the panel is not as it opened on All"
    fi

    # [ stops at All; ] twice reaches Planned, as 3 did.
    "${NENV[@]}" wtype -k bracketleft
    sleep 1
    shot tab_left || { summary; exit 1; }
    if same keyboard tab_left; then
        ok "[ on the first tab stays there"
    else
        bad "[ on All changed the panel; it should stop at the end"
    fi
    "${NENV[@]}" wtype -k bracketright
    "${NENV[@]}" wtype -k bracketright
    sleep 1
    shot tab_right || { summary; exit 1; }
    if same tab_planned tab_right; then
        ok "] twice steps from All to Planned"
    else
        bad "] twice from All is not the Planned tab 3 showed"
    fi
    "${NENV[@]}" wtype 1
    sleep 1
```

Then replace the existing Escape block (from `"${NENV[@]}" wtype -k Escape` to the `fi` closing its `if same three released`) with:

```bash
    # Escape from a tab other than All: the hover panel has no tabs, so it
    # comes back exactly as before, and the next keyboard opens on All.
    "${NENV[@]}" wtype 3
    sleep 1
    "${NENV[@]}" wtype -k Escape
    settle
    shot released || { summary; exit 1; }
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi

    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot reopened || { summary; exit 1; }
    if same keyboard reopened; then
        ok "taking the keyboard again opens on All, whatever tab it was left on"
    else
        bad "the keyboard reopened on something other than All with the first card focused"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
```

Change the `skip` line's text to `"the panel's cards in the middle of the screen, their buttons, the filter tabs, and Escape back (needs wtype)"`.

In the header comment, change the sentence `with wtype, Down and Escape are pressed in the nested niri, never on your desktop.` to `with wtype, Down, the filter tabs' keys and Escape are pressed in the nested niri, never on your desktop.`

- [ ] **Step 3: Run the e2e test**

Run: `cargo build --release && NIRITASKS=$PWD/target/release/niritasks bash tests/e2e-panel.sh`

`NIRITASKS` must point at this worktree's build: `tests/lib/nested-niri.sh` defaults it to `niritasks` on `$PATH`, the *installed* binary, which has no tabs. Keep off the workspace the nested niri parks on until it finishes.

Expected: every check PASS, including the five new ones; none SKIP if `wtype` is installed. On a FAIL, rerun with `NIRITASKS_E2E_KEEP=1` and look at the frames in the printed sandbox's `shots/`.

- [ ] **Step 4: Update the README**

In the keybind table row for `Mod+Alt+Ctrl+T`, after `Enter opens the full menu (note, grill me, update status, move to another workspace).` insert ` Tabs above the cards, picked with 1–4 or [ and ], narrow them to All, Active, Planned or To refine.`

In the "Task panel" section, after the paragraph ending `Hovering never shows the buttons — only the keyboard does.`, add a paragraph:

```markdown
Above the cards, four tabs narrow them while the panel has the keyboard:
**All**, **Active** (started), **Planned** (refined or grilled into a plan)
and **To refine** (not planned yet), each with how many tasks it holds. `1`
to `4` pick one, `[` and `]` step to the one either side, and a click picks
one too; the arrows and Tab still move only between cards and buttons. A
started planned task is under both Active and Planned. A tab with nothing
under it says so and keeps the keyboard. The panel opens on All every time.
```

In the hand checks paragraph (the one starting `It cannot move the pointer, so the hover is checked by hand`), before `Escape tucks it away to the one-line peek.` insert: `a click on a filter tab switches the cards and leaves the focus where it was, and a started planned task shows under both Active and Planned;`

In the e2e description paragraph (starting `` `tests/e2e-panel.sh` is the third``), change `the keyboard's cards in the middle of the screen, and Escape putting them back,` to `the keyboard's cards in the middle of the screen, its filter tabs (an empty one keeping the keyboard, the keys stopping at the ends, reopening on All), and Escape putting them back,`.

- [ ] **Step 5: Add "Filter tab" to CONTEXT.md**

In `CONTEXT.md`, under `### On screen`, after the **Action row** entry, add:

```markdown
**Filter tab**:
One of the four tabs above the task cards while the task panel has the
keyboard — All, Active, Planned and To refine — narrowing the cards to the
tasks it names and showing how many there are. A filter, not a status: a
started planned task is under both Active and Planned, and To refine is
every task that is not a planned task.
_Avoid_: tab alone (Tab is also a key), category, view, status
```

- [ ] **Step 6: Run the whole suite**

Run: `cargo test && cargo build --release && NIRITASKS=$PWD/target/release/niritasks bash tests/all.sh`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
Check the panel's filter tabs in the nested niri, and describe them

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
