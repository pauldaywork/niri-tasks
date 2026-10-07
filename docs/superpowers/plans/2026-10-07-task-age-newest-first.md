# Task Age, Newest First — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The task panel lists a workspace's tasks active, then up next, then by Taskwarrior priority, then newest first, and each task card shows how long ago its task was added (`5m`, `3h`, `2d`, `4w`) at its right end.

**Architecture:** `src/task.rs` reads `entry` and `priority` into `Task`, and gains `age(stamp, now)`, which turns a Taskwarrior stamp into a short age, beside `Annotation::date`. `model::cards` sorts on active, up next, blocked (for the rest only), priority and `entry`, and stops reading urgency. `Card` carries the task's `entry` stamp, not a ready-made age, so a card does not change every minute and `PanelState::set_cards` does not re-render (which would disarm a half-pressed Remove). The surface draws the age as its own label and rewrites every age label's text once a minute on a timer.

**Tech Stack:** Rust 2021, GTK4 (gtk4-rs) and its CSS, Taskwarrior 2.6 CLI. No new crates.

**Spec:** Taskwarrior task `d7488051-62d1-43cc-98ce-d10255f7947b`. Read it with `task rc.json.array=on d7488051-62d1-43cc-98ce-d10255f7947b export`. Its description and notes are the spec.

## Global Constraints

- Order: active, then up next, then priority H > M > L > none, then newest `entry` first. Blocked tasks go below unblocked ones in that last group (neither active nor up next). Active and up next tasks stay on top even when blocked.
- Due dates do not affect the order. Urgency no longer affects the panel's order.
- Each task card shows a short age at its right end, from the task's `entry` stamp: `5m`, `3h`, `2d`, `4w`.
- Only the task panel changes. The fuzzel picker (`src/rows.rs`) and `task::pending_for_tag` keep their urgency order.
- Out of scope: ordering by due date, changing Taskwarrior's urgency coefficients, and the fuzzel picker.
- Done when: the panel lists a workspace's tasks active, up next, priority, then newest, each card with its age. Model tests cover the order and the age format, and `tests/e2e-panel.sh` passes.
- House style: every new item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (`feat(panel): …`, imperative, lowercase, ≤72 chars) and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions made while planning (flag any you disagree with)

- **Blocked sits above priority in the last group.** "Blocked tasks go below unblocked ones in that last group" is read as: below up next, every unblocked task comes before every blocked one, and priority then newest orders each half. A blocked `H` task sits under an unblocked task with no priority, since it cannot be started yet. Inside the active and up next groups, blocked counts for nothing.
- **Newest first compares the stamps as text.** `20261006T120929Z` sorts the same as text and as a time, so the sort needs no parsing. A task with no `entry` (none in practice) sorts last.
- **Under a minute reads `0m`.** The spec names only m, h, d and w. A stamp in the future (clock skew) reads `0m` too. A stamp that does not parse shows no age at all. Weeks have no upper unit, so a year-old task reads `52w`.
- **Units cut over at whole units.** Under 1h → minutes, under 1d → hours, under 7d → days, then weeks. Each count is rounded down.
- **No date crate.** Parsing one fixed stamp shape into seconds takes Howard Hinnant's `days_from_civil`, which is about ten lines (http://howardhinnant.github.io/date_algorithms.html#days_from_civil). `chrono`, `time` or `jiff` would each add a dependency tree for that one call, and the crate has kept to a short dependency list so far.
- **Ages refresh once a minute without a re-render.** `Card` holds the stamp, so the daemon's cards compare equal minute to minute, and `set_cards` keeps the focus and anything armed. Each `Panel` runs a 60-second timer that rewrites its age labels in place.
- **The age is dimmed** like the action row's hint, `alpha(TEXT, 0.55)`, on every card, including active (green) and up next (yellow) ones.
- **The e2e test adds its tasks with a fixed old `entry`.** With `entry:now` a label could tick from `0m` to `1m` between two frames the test compares and fail it. With `entry:20260101T000000Z` the age only changes on a week boundary. Taskwarrior 2.6 accepts `entry:` on `add` (checked while planning).

## File Structure

- `src/task.rs` — `Task.entry`, `Task.priority`, `Task::priority_rank`, `age`, `now_secs`, and the private `stamp_secs` and `days_from_civil`. Tests for the age format and the stamp parse.
- `src/panel/model.rs` — `Card.entry`, `Card::age`, the new sort in `cards`. Tests per rule.
- `src/panel/state.rs` — test helper `card()` gains `entry`.
- `src/rows.rs` — test helper `task()` gains the two new `Task` fields. Its sort is unchanged.
- `src/panel/surface.rs` — `card_label` draws the age. `CardWidgets` keeps the label. `Panel::new` starts the minute timer.
- `src/panel/style.rs` — the `.card-age` rule and its test.
- `tests/e2e-panel.sh` — `add()` sets a fixed `entry`.
- `CONTEXT.md`, `README.md` — the order and the age.

---

### Task 1: Read entry and priority, and format an age

**Files:**
- Modify: `src/task.rs` (struct `Task` at :28-48, `impl Task` at :56-73, after `impl Annotation` ending at :103, tests from :613)
- Modify: `src/rows.rs:64-79` (test helper `task()`)
- Modify: `src/panel/model.rs:241-251` (test helper `task()`, only to add the two fields so it compiles; Task 2 rewrites it)

**Interfaces:**
- Produces:
  - `Task.entry: String` (Taskwarrior's stamp, empty when absent)
  - `Task.priority: Option<String>` (`"H"`, `"M"`, `"L"` or `None`)
  - `Task::priority_rank(&self) -> u8` (H 3, M 2, L 1, otherwise 0)
  - `pub fn age(stamp: &str, now: i64) -> Option<String>` in `crate::task`
  - `pub fn now_secs() -> i64` in `crate::task` (Unix seconds)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/task.rs`:

```rust
    /// `entry` and `priority` are what the panel sorts by; a task with no
    /// priority has the field absent rather than empty.
    #[test]
    fn parses_entry_and_priority() {
        let json = r#"[{"uuid":"a","description":"d","entry":"20261006T120929Z","priority":"H"},
                       {"uuid":"b","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].entry, "20261006T120929Z");
        assert_eq!(tasks[0].priority_rank(), 3);
        assert_eq!(tasks[1].entry, "");
        assert_eq!(tasks[1].priority_rank(), 0);
    }

    #[test]
    fn priority_ranks_h_over_m_over_l_over_none() {
        let rank = |p: Option<&str>| {
            let mut t: Task = serde_json::from_str(r#"{"uuid":"a","description":"d"}"#).unwrap();
            t.priority = p.map(String::from);
            t.priority_rank()
        };
        assert!(rank(Some("H")) > rank(Some("M")));
        assert!(rank(Some("M")) > rank(Some("L")));
        assert!(rank(Some("L")) > rank(None));
        assert_eq!(rank(Some("X")), rank(None), "a priority taskwarrior does not have counts as none");
    }

    /// Checked against Python's `calendar.timegm`, a leap day included.
    #[test]
    fn a_stamp_reads_as_unix_seconds() {
        assert_eq!(stamp_secs("19700101T000000Z"), Some(0));
        assert_eq!(stamp_secs("20261006T120929Z"), Some(1_791_288_569));
        assert_eq!(stamp_secs("20240229T000000Z"), Some(1_709_164_800));
        assert_eq!(stamp_secs("20240301T000000Z"), Some(1_709_164_800 + 86_400));
    }

    /// Taskwarrior's to define, so any other shape is no age rather than a
    /// wrong one, and a multi-byte character is no panic.
    #[test]
    fn a_stamp_of_another_shape_has_no_age() {
        for bad in ["", "2026-10-06", "20261006T120929", "20261306T120929Z", "2026100éT12092Z", "20261006X120929Z"] {
            assert_eq!(age(bad, 0), None, "{bad:?}");
        }
    }

    /// Each unit from its first second to its last, rounded down.
    #[test]
    fn ages_cut_over_at_whole_units() {
        let added = "20261006T120000Z";
        let at = |secs: i64| age(added, stamp_secs(added).unwrap() + secs).unwrap();
        assert_eq!(at(0), "0m");
        assert_eq!(at(59), "0m");
        assert_eq!(at(60), "1m");
        assert_eq!(at(3_599), "59m");
        assert_eq!(at(3_600), "1h");
        assert_eq!(at(86_399), "23h");
        assert_eq!(at(86_400), "1d");
        assert_eq!(at(7 * 86_400 - 1), "6d");
        assert_eq!(at(7 * 86_400), "1w");
        assert_eq!(at(365 * 86_400), "52w");
    }

    /// A clock behind the one that stamped the task reads as just added.
    #[test]
    fn a_stamp_from_the_future_is_just_added() {
        let added = "20261006T120000Z";
        assert_eq!(age(added, stamp_secs(added).unwrap() - 30).as_deref(), Some("0m"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib task::tests`
Expected: compile errors: no field `entry`, no method `priority_rank`, cannot find `stamp_secs`/`age`.

- [ ] **Step 3: Add the fields**

In `struct Task`, after `status`:

```rust
    /// When the task was added, as taskwarrior stamps it
    /// (`20261006T120929Z`). The task panel sorts newest first by it and shows
    /// its [`age`]. Every task has one; the default only keeps a hand-built
    /// task in a test parseable.
    #[serde(default)]
    pub entry: String,
    /// Taskwarrior's `priority`: `H`, `M` or `L`, absent when unset. The task
    /// panel sorts by it, above age.
    #[serde(default)]
    pub priority: Option<String>,
```

In `impl Task`, after `is_up_next`:

```rust
    /// The priority as a number to sort by, highest first: H over M over L
    /// over none. Anything else counts as none, as taskwarrior allows no
    /// other value.
    pub fn priority_rank(&self) -> u8 {
        match self.priority.as_deref() {
            Some("H") => 3,
            Some("M") => 2,
            Some("L") => 1,
            _ => 0,
        }
    }
```

- [ ] **Step 4: Add the age formatter**

After `impl Annotation { … }` (ends at :103), add:

```rust
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

/// How long ago a stamp was, as a task card shows it: `5m`, `3h`, `2d` or
/// `4w`, rounded down, from `now` in Unix seconds.
///
/// Under a minute, or a stamp ahead of `now` (another machine's clock), is
/// `0m`. A stamp not of taskwarrior's shape is `None`, so the card shows no
/// age rather than a wrong one — the same reasoning as [`Annotation::date`].
pub fn age(stamp: &str, now: i64) -> Option<String> {
    let secs = (now - stamp_secs(stamp)?).max(0);
    let (n, unit) = match secs {
        s if s < HOUR => (s / MINUTE, 'm'),
        s if s < DAY => (s / HOUR, 'h'),
        s if s < WEEK => (s / DAY, 'd'),
        s => (s / WEEK, 'w'),
    };
    Some(format!("{n}{unit}"))
}

/// The time now, in Unix seconds, for [`age`]. A clock before 1970 reads
/// as 1970 rather than failing a panel draw.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A taskwarrior stamp, `20261006T120929Z`, as Unix seconds. Always UTC: the
/// `Z` is part of the shape.
fn stamp_secs(stamp: &str) -> Option<i64> {
    let b = stamp.as_bytes();
    if b.len() != 16 || b[8] != b'T' || b[15] != b'Z' {
        return None;
    }
    // `get`, not indexing: a multi-byte character is None, not a panic.
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = stamp.get(r)?;
        s.bytes().all(|c| c.is_ascii_digit()).then(|| s.parse().ok())?
    };
    let (y, mo, d) = (num(0..4)?, num(4..6)?, num(6..8)?);
    let (h, mi, s) = (num(9..11)?, num(11..13)?, num(13..15)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some(days_from_civil(y, mo, d) * DAY + h * HOUR + mi * MINUTE + s)
}

/// Days from 1970-01-01 to a date in the proleptic Gregorian calendar.
/// Howard Hinnant's algorithm
/// (<http://howardhinnant.github.io/date_algorithms.html#days_from_civil>):
/// ten lines here rather than a date crate for one fixed stamp shape.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
```

- [ ] **Step 5: Fix the hand-built tasks in other tests**

In `src/rows.rs`, in the test helper `task()`, after `status: "pending".into(),` add:

```rust
            entry: String::new(),
            priority: None,
```

In `src/panel/model.rs`, in the test helper `task()`, after `status: "pending".into(),` add the same two lines. (Task 2 replaces this helper.)

- [ ] **Step 6: Run the tests**

Run: `cargo test`
Expected: all pass, the six new ones included.

- [ ] **Step 7: Commit**

```bash
git add src/task.rs src/rows.rs src/panel/model.rs
git commit -m "feat(task): read entry and priority, and format a task's age

The task panel will sort by priority, then newest first, and show each
task's age, so Task reads both fields. age() turns taskwarrior's stamp
into 5m, 3h, 2d or 4w without a date crate.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Sort the cards active, up next, priority, newest

**Files:**
- Modify: `src/panel/model.rs` (`Card` at :28-43, `cards` at :175-219, `cap` at :225-237, tests from :237)
- Modify: `src/panel/state.rs:517-519` (test helper `card()`)

**Interfaces:**
- Consumes: `Task.entry`, `Task::priority_rank()`, `crate::task::age(stamp, now)` from Task 1.
- Produces:
  - `Card.entry: String` (the task's stamp; empty on "+N more")
  - `Card::age(&self, now: i64) -> Option<String>`
  - `cards(tasks: &[Task], blocked: &[String]) -> Vec<Card>` keeps its signature.

- [ ] **Step 1: Rewrite the test helpers around the day a task was added**

The helpers took urgency, which no longer orders anything. They take the day of October 2026 the task was added instead, so a higher number is newer. In `mod tests` of `src/panel/model.rs`, replace `task` and `up_next`:

```rust
    /// A task added on this day of October 2026: a higher day is newer, so
    /// it sorts higher, as a higher urgency used to.
    fn task(uuid: &str, day: u32, active: bool) -> Task {
        Task {
            uuid: uuid.into(),
            description: uuid.into(),
            urgency: 0.0,
            start: active.then(|| "20260927T080000Z".to_string()),
            annotations: Vec::new(),
            tags: Vec::new(),
            status: "pending".into(),
            entry: format!("202610{day:02}T120000Z"),
            priority: None,
        }
    }

    fn up_next(uuid: &str, day: u32, active: bool) -> Task {
        let mut t = task(uuid, day, active);
        t.tags = vec![crate::task::UP_NEXT_TAG.into()];
        t
    }

    fn with_priority(mut t: Task, p: &str) -> Task {
        t.priority = Some(p.into());
        t
    }
```

In `planned()` and `waiting()`, change `task(uuid, 1.0, active)` / `task(uuid, 1.0, false)` to `task(uuid, 1, active)` / `task(uuid, 1, false)`.

Then in every remaining call to `task(…)` or `up_next(…)` in the module, change the second argument from a float to the whole day with the same order: `1.0` → `1`, `2.0` → `2`, `5.0` → `5`, `9.0` → `9`, `0.5` → `1`. Two calls compute it:
- `task(&format!("t{i}"), i as f64, false)` in `past_the_cap_the_rest_fold_into_one_card` → `task(&format!("t{i}"), i as u32 + 1, false)`; `t10` is still first.
- In `planned_does_not_change_the_order`, replace `low.urgency = 1.0;` with nothing (`planned()` is already day 1).

Rename the test `active_first_then_most_urgent` to `active_first_then_newest`. In `past_the_cap_the_rest_fold_into_one_card`, change the assertion message `"the cap keeps the most urgent"` to `"the cap keeps the newest"`.

Update `src/panel/state.rs:518` so the helper builds:

```rust
        Card { status, text: uuid.into(), uuid: Some(uuid.into()), planned: status == Status::Planned, up_next: false, entry: String::new() }
```

and the `card` closure in `a_cards_state_is_its_tasks` (model.rs) the same way, adding `entry: String::new()`.

- [ ] **Step 2: Add a failing test per rule**

Add to `mod tests` in `src/panel/model.rs`:

```rust
    /// Newest first below up next, whatever the urgency: taskwarrior's age
    /// coefficient lifts old tasks, which is what this replaces.
    #[test]
    fn newest_first_whatever_the_urgency() {
        let mut old = task("old", 1, false);
        old.urgency = 20.0;
        let got = cards(&[old, task("new", 9, false)], &[]);
        assert_eq!(texts(&got), vec!["new", "old"]);
    }

    #[test]
    fn priority_outranks_age_h_over_m_over_l_over_none() {
        let got = cards(
            &[
                task("none-new", 9, false),
                with_priority(task("l", 4, false), "L"),
                with_priority(task("h-old", 1, false), "H"),
                with_priority(task("m", 2, false), "M"),
                with_priority(task("h-new", 3, false), "H"),
            ],
            &[],
        );
        assert_eq!(texts(&got), vec!["h-new", "h-old", "m", "l", "none-new"]);
    }

    /// Up next outranks priority, as active does.
    #[test]
    fn active_then_up_next_outrank_priority() {
        let got = cards(
            &[with_priority(task("high", 9, false), "H"), up_next("next", 1, false), task("started", 1, true)],
            &[],
        );
        assert_eq!(texts(&got), vec!["started", "next", "high"]);
    }

    /// Below up next, a task that cannot be started yet waits under every one
    /// that can, high priority or new as it is; priority then age order each
    /// half.
    #[test]
    fn blocked_sinks_below_the_unblocked_rest() {
        let got = cards(
            &[
                with_priority(task("blocked-h", 9, false), "H"),
                task("blocked-new", 8, false),
                task("blocked-old", 2, false),
                task("free-old", 1, false),
                with_priority(task("free-l", 1, false), "L"),
            ],
            &["blocked-h".into(), "blocked-new".into(), "blocked-old".into()],
        );
        assert_eq!(texts(&got), vec!["free-l", "free-old", "blocked-h", "blocked-new", "blocked-old"]);
    }

    /// Being blocked does not move an active or up next task off the top.
    #[test]
    fn active_and_up_next_stay_on_top_when_blocked() {
        let got = cards(
            &[task("free", 9, false), up_next("next", 1, false), task("started", 1, true)],
            &["next".into(), "started".into()],
        );
        assert_eq!(texts(&got), vec!["started", "next", "free"]);
    }

    /// The card keeps the stamp, not the age, so it reads the same minute to
    /// minute and the panel need not re-render to age it.
    #[test]
    fn a_card_ages_from_its_tasks_stamp() {
        let got = cards(&[task("t", 6, false)], &[]);
        assert_eq!(got[0].entry, "20261006T120000Z");
        // 2026-10-06 12:00:00Z is 1_791_288_000 (Task 1's 12:09:29 less 569s).
        let later = 1_791_288_000 + 3 * 3_600;
        assert_eq!(got[0].age(later).as_deref(), Some("3h"));
    }

    /// "+N more" stands for no one task, so it has no age.
    #[test]
    fn the_more_card_has_no_age() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| task(&format!("t{i}"), i as u32 + 1, false)).collect();
        let more = cap(&cards(&many, &[]), CAP).pop().unwrap();
        assert_eq!(more.entry, "");
        assert_eq!(more.age(i64::MAX / 2), None);
    }
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --lib panel::model`
Expected: compile error, no field `entry` on `Card` / no method `age`. (Once those exist, the order tests fail on the old urgency sort.)

- [ ] **Step 4: Add `entry` and `age` to `Card`**

In `struct Card`, after `up_next`:

```rust
    /// When the task was added, taskwarrior's stamp, for the age at the
    /// card's right end. The stamp, not the age: the age changes every
    /// minute, and a changed card re-renders the panel, disarming a
    /// half-pressed Remove. Empty on "+N more".
    pub entry: String,
```

In `impl Card`, after `shows_up_next`:

```rust
    /// How long ago the task was added, `now` in Unix seconds. None on
    /// "+N more", and on a stamp taskwarrior wrote in some other shape.
    pub fn age(&self, now: i64) -> Option<String> {
        crate::task::age(&self.entry, now)
    }
```

In `cap`, the "+N more" card gets `entry: String::new(),` after `up_next: false,`. In `cards`, the `Card { … }` built per task gets `entry: t.entry.clone(),` after `up_next: t.is_up_next(),`.

- [ ] **Step 5: Change the sort**

Replace the doc comment and the sort at the top of `cards` with:

```rust
/// The task cards for one workspace tag, every one of them: the active tasks
/// first, then the up next ones, then the rest by priority, newest first,
/// with the blocked ones under the rest.
///
/// Active first even when something else is higher priority: it is the work
/// in progress, and the one card worth reading without hovering. Up next
/// comes right under it, being the one to do next. Being started or up next
/// outranks being blocked, since you picked it anyway. Below them, a task
/// that cannot be started yet goes under every one that can. Newest first,
/// not urgency: taskwarrior's age coefficient lifts old tasks, and a task
/// just added is the one most likely to matter. Due dates count for nothing
/// here; the picker keeps the urgency order.
pub fn cards(tasks: &[Task], blocked: &[String]) -> Vec<Card> {
    // Only below up next: an active or up next task stays on top blocked.
    let sinks = |t: &Task| !t.is_active() && !t.is_up_next() && blocked.contains(&t.uuid);
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| {
        b.is_active()
            .cmp(&a.is_active())
            .then(b.is_up_next().cmp(&a.is_up_next()))
            .then(sinks(a).cmp(&sinks(b)))
            .then(b.priority_rank().cmp(&a.priority_rank()))
            // The stamp sorts as text the way it does as a time.
            .then(b.entry.cmp(&a.entry))
    });
```

Leave the `sorted.iter().map(…)` part as it is apart from the `entry` field from Step 4.

- [ ] **Step 6: Run the tests**

Run: `cargo test`
Expected: all pass. If an older test now fails, its expectation relied on urgency; check that the day-for-urgency swap in Step 1 kept every float's order and fix the call, not the sort.

- [ ] **Step 7: Commit**

```bash
git add src/panel/model.rs src/panel/state.rs
git commit -m "feat(panel): list tasks by priority then newest first

Below the active and up next tasks the panel now orders by priority,
H over M over L over none, then newest first, with blocked tasks under
the rest. Urgency lifted old tasks through its age coefficient. Cards
carry the task's entry stamp for the age shown next.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Draw the age on each card

**Files:**
- Modify: `src/panel/surface.rs` (`CardWidgets` at :199-206, `Panel::new` tail at :330-342, `card_widget` at :681-726, `card_label` at :1003-1030)
- Modify: `src/panel/style.rs` (the CSS after the `.card-hint` rule, tests at the end)

**Interfaces:**
- Consumes: `Card::age(&self, now: i64) -> Option<String>`, `Card.entry` (Task 2), `crate::task::{age, now_secs}` (Task 1).
- Produces: CSS class `card-age` on the age label.

- [ ] **Step 1: Write the failing style test**

Add to `mod tests` in `src/panel/style.rs`:

```rust
    /// The age is a caption to the description, dimmed like the action
    /// row's hint, and parted from the text by a card's padding.
    #[test]
    fn the_age_is_dimmed() {
        assert!(css().contains(&format!(
            ".task-panel .card-age {{ color: alpha({TEXT}, 0.55); padding-left: {PADDING_PX}px; }}"
        )));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --lib panel::style`
Expected: FAIL in `the_age_is_dimmed`.

- [ ] **Step 3: Add the rule**

In `css()` in `src/panel/style.rs`, right after the `.card-hint` line:

```rust
/* How long ago the task was added, at the card's right end: a caption, so
   dimmed like the hint, on a green or yellow card too. */
.task-panel .card-age {{ color: alpha({TEXT}, 0.55); padding-left: {PADDING_PX}px; }}
```

Run: `cargo test --lib panel::style` — Expected: PASS.

- [ ] **Step 4: Draw the label**

In `src/panel/surface.rs`, change `card_label` to also return the age label, and draw it after the text:

```rust
/// The card's icon, description and age: one line cut off with "…" for the
/// peek and the hover, or all of it, wrapped, while the panel has the
/// keyboard. The age label comes back too, for the minute timer to update.
///
/// The icon is a label of its own beside the text, so wrapped lines start
/// under the first line's text rather than back under the icon. The age is
/// one too, at the right end, so the text's "…" stops short of it.
fn card_label(card: &Card, wrap: bool) -> (gtk4::Box, Option<gtk4::Label>) {
```

Keep the body as it is up to `row.append(&text);`, then replace the final `row` with:

```rust
    let age = card.age(crate::task::now_secs()).map(|a| {
        let age = gtk4::Label::new(Some(&a));
        age.add_css_class("card-age");
        // Level with the first line when the text wraps.
        age.set_valign(gtk4::Align::Start);
        row.append(&age);
        age
    });
    (row, age)
}
```

In `struct CardWidgets`, after `row`:

```rust
    /// The age at the right end, and the stamp it counts from, for the
    /// minute timer. None on "+N more".
    age: Option<(gtk4::Label, String)>,
```

In `card_widget`, replace

```rust
        let body = gtk4::Button::builder().child(&card_label(card, keyboard)).build();
```

with

```rust
        let (label, age) = card_label(card, keyboard);
        let body = gtk4::Button::builder().child(&label).build();
```

and its last line with

```rust
        let age = age.map(|label| (label, card.entry.clone()));
        CardWidgets { uuid: card.uuid.clone(), root, body, row, age }
```

- [ ] **Step 5: Refresh the ages once a minute**

Add a method to `impl Panel` (beside `render`):

```rust
    /// Rewrite each card's age where it stands. Not a render: the cards are
    /// the same, and a render would tear the column down and disarm a
    /// half-pressed Remove.
    fn refresh_ages(&self) {
        let now = crate::task::now_secs();
        for card in self.cards.borrow().iter() {
            if let Some((label, entry)) = &card.age {
                if let Some(age) = crate::task::age(entry, now) {
                    label.set_text(&age);
                }
            }
        }
    }
```

In `Panel::new`, after `panel.connect_surface();`:

```rust
        // The ages count up while the cards stay put. Weak, so a panel
        // dropped when its monitor goes ends its timer.
        {
            let weak = Rc::downgrade(&panel);
            glib::timeout_add_seconds_local(60, move || match weak.upgrade() {
                Some(p) => {
                    p.refresh_ages();
                    glib::ControlFlow::Continue
                }
                None => glib::ControlFlow::Break,
            });
        }
```

A label only changes when its text does, and every age under an hour moves each minute, so a once-a-minute timer is late by at most a minute.

- [ ] **Step 6: Build, test, and look at it**

Run: `cargo build && cargo test`
Expected: builds without warnings, all tests pass.

Then restart the daemon from this build and look at a workspace with tasks: every card has a dimmed age at its right end, a long description ends in "…" before the age, and with the keyboard (Mod+Alt+Ctrl+T) the age sits level with the first line of a wrapped description. Use the `run` skill if unsure how to launch it.

- [ ] **Step 7: Commit**

```bash
git add src/panel/surface.rs src/panel/style.rs
git commit -m "feat(panel): show each task's age at its card's right end

A dimmed 5m, 3h, 2d or 4w from the task's entry stamp. A minute timer
rewrites the labels in place, so the age keeps up without a re-render
that would disarm a half-pressed Remove.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Pin the e2e test's ages and update the docs

**Files:**
- Modify: `tests/e2e-panel.sh:75-77` (`add()`)
- Modify: `CONTEXT.md` (**Up next task** at :47-53, **Task card** at :64-68)
- Modify: `README.md:26-30`

**Interfaces:**
- Consumes: the order and the age from Tasks 2 and 3.

- [ ] **Step 1: Run the e2e test as it stands**

Run: `bash tests/e2e-panel.sh`
Expected: passes, or fails only where a frame comparison caught an age label ticking. Note which.

- [ ] **Step 2: Give the e2e tasks a fixed age**

Replace `add()` in `tests/e2e-panel.sh` with:

```bash
add() {  # description
    # A fixed, old entry: an age counted from now could tick from 0m to 1m
    # between two frames compared for sameness. This one only moves weekly.
    task rc.verbose=nothing rc.confirmation=no add "+$TAG" entry:20260101T000000Z -- "$1" >/dev/null 2>&1
}
```

(Keep the `add` on another tag at :213 as it is: it only checks nothing changes.)

- [ ] **Step 3: Run it**

Run: `bash tests/e2e-panel.sh`
Expected: PASS. All three tasks share one entry and no priority, so they keep the order the sort is stable on, which is the export's: the order they were added. If a check fails on which card is where, read what it expected before changing anything: the order rules are in Task 2 and are not to be bent for the test.

- [ ] **Step 4: Update CONTEXT.md**

In **Up next task**, replace

```
yellow and sits directly under the active tasks, unless it is active itself
(green) or waiting; the tag adds urgency, so the picker lifts it too. Up
next again takes the mark off.
```

with

```
yellow and sits directly under the active tasks, above any priority, unless
it is active itself (green) or waiting; the tag adds urgency, so the picker
lifts it too. Up next again takes the mark off.
```

Replace the **Task card** entry's definition with:

```
One task in the task panel, drawn like a notification: a status icon, a
one-line description and, at its right end, how long ago the task was added
(5m, 3h, 2d, 4w) — or, while the task panel has the keyboard, the whole
description wrapped, above its action row while it has focus. The cards run
active, then up next, then by Taskwarrior priority, newest first, with
blocked tasks under the rest.
```

- [ ] **Step 5: Update README.md**

In the **Task panel** section, replace

```
several, one per worktree being worked), each ▶ and text in green, then the rest most urgent first: ○, a dimmed lock for one blocked on
```

with

```
several, one per worktree being worked), each ▶ and text in green, then any
up next, then the rest by priority (H, M, L, none) and newest first, the
blocked ones last: ○, a dimmed lock for one blocked on
```

and add a sentence at the end of that paragraph, after the "+N more" sentence:

```
Each card ends in how long ago its task was added: 5m, 3h, 2d, 4w.
```

Read the paragraph afterwards to check it still runs as sentences. `llms.txt` needs nothing: it says only that up next sorts under the active tasks, which holds, and the picker's urgency order is unchanged.

- [ ] **Step 6: Run everything**

Run: `bash tests/all.sh`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add tests/e2e-panel.sh CONTEXT.md README.md
git commit -m "docs: describe the panel's priority and newest-first order

Says how the task panel orders its cards and that each shows its age.
The e2e test adds its tasks with a fixed entry so no age ticks between
two frames it compares.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
