# Leave Started Tasks off To refine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A started task shows under the Active tab only, never under To refine, the way a started planned task already leaves Planned.

**Architecture:** The keyboard panel's filter tabs are decided by one function, `Filter::matches` in `src/panel/model.rs`. To refine currently matches every unplanned card; it gains a `card.status != Status::Active` condition, mirroring Planned's. `Filter::shown` builds the tab bar from `matches`, so To refine hides by itself when the only unplanned tasks are started. Doc comments, `CONTEXT.md` and `README.md` say what the tab holds, so they change with it.

**Tech Stack:** Rust (`cargo test`), unit tests in `#[cfg(test)] mod tests` at the bottom of `src/panel/model.rs`.

**Spec:** Taskwarrior task `6014d9eb-562d-4027-8ac9-eaed1edf5eb6` — read it with `task rc.json.array=on 6014d9eb-562d-4027-8ac9-eaed1edf5eb6 export`. Its annotations:
- Goal: A started task shows under Active only, never under To refine, as a started planned task already leaves Planned.
- Decided: To refine is unplanned tasks that are not started: `!card.planned && card.status != Status::Active`; blocked and pending unplanned tasks stay.
- Decided: Refine stays offered on an active card; only the tab's filter changes.
- Done when: A started unplanned task appears under Active and All but not To refine, To refine hides when only started tasks are unplanned, and `cargo test` passes.
- Out of scope: Changing the Active, Planned or other tabs, or what Ctrl+Enter does on an active card.

## Global Constraints

- To refine's rule is exactly `!card.planned && card.status != Status::Active`; blocked and pending unplanned tasks stay under it.
- Do not touch the Active, Planned, Waiting, Finished or All arms of `Filter::matches`, the card buttons (Refine stays on an active card), or Ctrl+Enter.
- Commit messages: Conventional Commits, `fix(panel): …`, subject ≤ 72 chars, imperative, lowercase, no full stop; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

### Task 1: To refine skips started tasks

**Files:**
- Modify: `src/panel/model.rs:107-127` (the `Filter` doc comments), `src/panel/model.rs:176` (the `ToRefine` arm of `matches`)
- Test: `src/panel/model.rs` — `each_tab_picks_its_tasks_in_order` (~line 663) and a new test after it
- Modify: `CONTEXT.md:111-117` (the **Filter tab** entry)
- Modify: `README.md:101` (the To refine gloss)

**Interfaces:**
- Consumes: existing test helpers in `src/panel/model.rs`'s `mod tests`: `cards(&Listing, &[]) -> Vec<Card>`, `todo(Vec<Task>) -> Listing`, `task(uuid: &str, day: u32, active: bool) -> Task`, `planned(uuid: &str, active: bool) -> Task`, `texts(cards: &[Card]) -> Vec<&str>`, and `Filter::{pick, shown}`.
- Produces: nothing new; `Filter::matches` keeps its signature `fn matches(self, card: &Card) -> bool`.

- [ ] **Step 1: Flip the existing test to the new rule**

In `src/panel/model.rs`, replace the doc comment and last assertion of `each_tab_picks_its_tasks_in_order`:

```rust
    /// A filter, not a status. A started task is under Active alone, planned
    /// or not: Planned is for picking what to start, and To refine for what
    /// to refine before starting, and it is started.
    #[test]
    fn each_tab_picks_its_tasks_in_order() {
        let all = cards(
            &todo(vec![
                task("plain", 9, false),
                planned("started-planned", true),
                task("started", 5, true),
                planned("planned", false),
            ]),
            &[],
        );
        assert_eq!(texts(&Filter::All.pick(&all)), texts(&all));
        assert_eq!(texts(&Filter::Active.pick(&all)), vec!["started", "started-planned"]);
        assert_eq!(texts(&Filter::Planned.pick(&all)), vec!["planned"]);
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["plain"]);
    }
```

- [ ] **Step 2: Add a test that To refine hides when only started tasks are unplanned**

Directly after `each_tab_picks_its_tasks_in_order`, add:

```rust
    /// A started task alone leaves no tab to refine: To refine is shown only
    /// while an unstarted unplanned task is under it.
    #[test]
    fn to_refine_hides_when_every_unplanned_task_is_started() {
        let all = cards(&todo(vec![task("started", 5, true), planned("planned", false)]), &[]);
        assert_eq!(Filter::shown(&all), vec![Filter::All, Filter::Active, Filter::Planned]);
    }
```

- [ ] **Step 3: Run the two tests to see them fail**

Run: `cargo test --lib panel::model::tests::each_tab_picks_its_tasks_in_order panel::model::tests::to_refine_hides_when_every_unplanned_task_is_started`

If cargo rejects two filters, run them one at a time: `cargo test --lib each_tab_picks_its_tasks_in_order` and `cargo test --lib to_refine_hides_when_every_unplanned_task_is_started`.

Expected: both FAIL — the first with `left: ["started", "plain"]` vs `right: ["plain"]`, the second with `ToRefine` in the shown tabs.

- [ ] **Step 4: Change the filter**

In `Filter::matches`, `src/panel/model.rs:176`, replace

```rust
            Filter::ToRefine => !card.planned,
```

with

```rust
            Filter::ToRefine => !card.planned && card.status != Status::Active,
```

- [ ] **Step 5: Update the `Filter` doc comments**

Replace the enum's doc comment (`src/panel/model.rs:107-113`) with:

```rust
/// A filter tab on the keyboard's panel: which of the cards it shows.
///
/// A filter, not a status: Planned and To refine go by the `+planned` tag,
/// which a card's icon can hide behind ▶ or the lock. A started task is
/// under Active and neither Planned nor To refine, which are for picking what
/// to start or refine next. A waiting task is under Waiting and no other tab,
/// and a finished one under Finished: it is parked or done, and All is what
/// the hover shows.
```

and the `ToRefine` variant's comment with:

```rust
    /// Tasks without `+planned`, and not started: the ones still worth refining.
    ToRefine,
```

- [ ] **Step 6: Update CONTEXT.md and README.md**

In `CONTEXT.md`'s **Filter tab** entry, replace

```
status: a started planned task is under Active and not Planned, To refine is
every task that is not a planned task, and a waiting task is under Waiting
alone.
```

with

```
status: a started task is under Active and neither Planned nor To refine, To
refine is every unstarted task that is not a planned task, and a waiting task
is under Waiting alone.
```

(Keep the line wrapping consistent with the surrounding paragraph, about 80 columns; rewrap the following lines of that paragraph only if needed.)

In `README.md:101`, replace `**To refine** (not planned yet)` with `**To refine** (neither planned nor started yet)`.

- [ ] **Step 7: Run the whole suite**

Run: `cargo test`
Expected: all tests PASS, including the two from Steps 1–2. The `src/panel/state.rs` tab tests (`a_tab_shows_only_while_it_has_tasks`, `the_tab_falls_back_to_all_when_it_empties`) still expect `ToRefine` because each also has an unstarted pending card `"p"`; if any other test fails, read it before changing it — only a test that expected a started task under To refine should change.

Also run: `cargo clippy --all-targets -- -D warnings` (expected: clean).

- [ ] **Step 8: Commit**

```bash
git add src/panel/model.rs CONTEXT.md README.md
git commit -m "$(cat <<'EOF'
fix(panel): leave started tasks off the To refine tab

A started task now shows under Active alone, as a started planned task
already left Planned. To refine is for picking what to refine before
starting, so it hides when its only unplanned tasks are started.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
