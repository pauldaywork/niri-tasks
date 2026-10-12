# Go to list document ages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a task's Go to list, show how old each linked document is (Open report, Planning doc, Design spec), dimmed and at the row's right end, as a task card shows its age.

**Architecture:** `Task` gains `report_link`/`plan_link`/`spec_link`, each a `Link { path, entry }`: the newest linking note's path and its `entry` stamp. The panel's `Card` carries those links in place of its three `has_*` bools. When Go to… is pressed, `PanelState::show_goto` turns them into a `goto::Since` (one stamp per document, falling back to the file's last change through a closure the surface supplies), which `GoToList` keeps; `GoToList::row_age(item, now)` gives the age, and `row_card` in `surface.rs` draws it with the existing `card-age` class.

**Tech Stack:** Rust 2021, gtk4-rs, Taskwarrior 3 JSON export. Tests: `cargo test`.

**Spec:** Taskwarrior task `5713483c-5633-46e3-b50f-027642c2dba1` — its description and notes (`task rc.json.array=on 5713483c export`). No separate spec file.

## Global Constraints

- Display only: nothing is saved to the task or its notes; the age is worked out each time the Go to list is drawn.
- The age counts from the linking note's existing `entry` date, falling back to the file's last change (resolved like `open::resolve`) when that date cannot be read.
- Reuse the `card-age` class and layout as they are, so it matches the task cards; no new style.
- Rows that are not documents (Refine, Grill me, Get started, Generate report, sessions, worktree, VS Code, Read aloud) show no age.
- Out of scope: ages on non-document rows; changing `card-age`'s look on the task cards.
- Age format is `task::age`'s: `5m`, `3h`, `2d`, `4w`.
- Comments follow the repo's voice: plain English, full sentences, what and why.
- Commits: Conventional Commits, `feat` for this task's user-facing commits, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

## File Structure

- `src/task.rs` — `Link` type and `Task::report_link`, `plan_link`, `spec_link`; `report_path` and friends become thin wrappers so their callers don't change.
- `src/open.rs` — `modified_secs(path)`: a file's last change in Unix seconds, beside `resolve`. `src/docs.rs` reuses it.
- `src/panel/model.rs` — `Card` carries `report`, `plan`, `spec: Option<Link>` in place of `has_report`, `has_plan`, `has_spec`.
- `src/panel/goto.rs` — `Since` (a stamp per document) and `Since::of`; `GoToList::new` takes one; `GoToList::row_age`.
- `src/panel/state.rs` — `show_goto` takes a `modified` closure and builds the list's `Since`.
- `src/panel/surface.rs` — `Effect::ListGoTo` supplies the closure; `row_card` draws an optional age.
- `CONTEXT.md` — the Go to list entry mentions the ages.

---

### Task 1: Read the date a document was linked

**Files:**
- Modify: `src/task.rs:151-183` (the `report_path`/`spec_path`/`plan_path`/`linked` block) and its tests (near `the_newest_spec_and_plan_notes_are_the_tasks`, ~line 1325)
- Modify: `src/open.rs` (after `resolve`, ~line 50) and its tests
- Modify: `src/docs.rs:94-100` (reuse `modified_secs`)

**Interfaces:**
- Produces:
  - `pub struct task::Link { pub path: String, pub entry: String }` — derives `Debug, Clone, PartialEq, Eq`.
  - `Task::report_link(&self) -> Option<Link>`, `Task::plan_link(&self) -> Option<Link>`, `Task::spec_link(&self) -> Option<Link>`.
  - `Task::report_path`, `plan_path`, `spec_path` keep their signatures (`Option<&str>`).
  - `pub fn open::modified_secs(path: &Path) -> Option<i64>`.

- [ ] **Step 1: Write the failing tests**

In `src/task.rs`'s `mod tests`, after `the_newest_spec_and_plan_notes_are_the_tasks`, add:

```rust
    /// Each link carries the note's own date beside its path: the newest
    /// note's, so a regenerated report's link is dated when it was made.
    #[test]
    fn a_link_carries_its_notes_date() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report: /r/first.html"},
                {"entry":"20261006T020000Z","description":"Spec: docs/superpowers/specs/a.md "},
                {"entry":"20261006T030000Z","description":"Plan: docs/superpowers/plans/a.md"},
                {"entry":"20261007T040000Z","description":"Report: /r/second.html"}
            ]}"#,
        )
        .unwrap();
        let link = |path: &str, entry: &str| Some(Link { path: path.into(), entry: entry.into() });
        assert_eq!(t.report_link(), link("/r/second.html", "20261007T040000Z"));
        assert_eq!(t.spec_link(), link("docs/superpowers/specs/a.md", "20261006T020000Z"));
        assert_eq!(t.plan_link(), link("docs/superpowers/plans/a.md", "20261006T030000Z"));
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!((bare.report_link(), bare.spec_link(), bare.plan_link()), (None, None, None));
    }
```

In `src/open.rs`'s `mod tests`, add:

```rust
    /// A file's last change in Unix seconds; nothing for a file not there.
    #[test]
    fn modified_secs_reads_a_files_last_change() {
        let dir = std::env::temp_dir().join(format!("niritasks-open-modified-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("plan.md");
        std::fs::write(&file, "# plan").unwrap();
        let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_791_331_200);
        std::fs::File::options().write(true).open(&file).unwrap().set_modified(when).unwrap();
        assert_eq!(modified_secs(&file), Some(1_791_331_200));
        assert_eq!(modified_secs(&dir.join("gone.md")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test a_link_carries_its_notes_date modified_secs_reads_a_files_last_change 2>&1 | tail -20`
Expected: compile errors — `cannot find struct Link`, `no method named report_link`, `cannot find function modified_secs`. (If `cargo test` rejects two filters, run them one at a time.)

- [ ] **Step 3: Implement `Link` and the `*_link` methods**

In `src/task.rs`, beside `Annotation` (after its struct, ~line 125), add:

```rust
/// A document a task's notes link: the path the newest `Report:`,
/// `Plan:` or `Spec:` note names, and that note's `entry` stamp, when it
/// was linked — what the Go to list counts the document's age from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub path: String,
    pub entry: String,
}
```

Replace the `report_path` … `linked` block (lines 151-183) with:

```rust
    /// The refine report linked from the task's notes: the path of the newest
    /// `Report: <path>` note, the last one, the mod appending them in the
    /// order the reports were made. None on a task with no such note; a
    /// `Report (earlier draft):` note is a report of a plan since changed,
    /// and never counts. A `Report:` note with nothing after it is skipped.
    pub fn report_path(&self) -> Option<&str> {
        self.linked(REPORT_NOTE).map(|(path, _)| path)
    }

    /// The design spec linked from the task's notes: the newest
    /// `Spec: <path>` note's path, read as [`Task::report_path`] reads a
    /// report's. None on a task with no such note.
    pub fn spec_path(&self) -> Option<&str> {
        self.linked(SPEC_NOTE).map(|(path, _)| path)
    }

    /// The implementation plan linked from the task's notes: the newest
    /// `Plan: <path>` note's path. None on a task with no such note.
    pub fn plan_path(&self) -> Option<&str> {
        self.linked(PLAN_NOTE).map(|(path, _)| path)
    }

    /// [`Task::report_path`] with the date its note was added.
    pub fn report_link(&self) -> Option<Link> {
        self.link(REPORT_NOTE)
    }

    /// [`Task::spec_path`] with the date its note was added.
    pub fn spec_link(&self) -> Option<Link> {
        self.link(SPEC_NOTE)
    }

    /// [`Task::plan_path`] with the date its note was added.
    pub fn plan_link(&self) -> Option<Link> {
        self.link(PLAN_NOTE)
    }

    /// The path the newest note starting with `prefix` links, trimmed, and
    /// that note. A note with nothing after the prefix is skipped.
    fn linked(&self, prefix: &str) -> Option<(&str, &Annotation)> {
        self.annotations.iter().rev().find_map(|a| {
            a.description
                .strip_prefix(prefix)
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| (p, a))
        })
    }

    /// [`Task::linked`] as an owned [`Link`].
    fn link(&self, prefix: &str) -> Option<Link> {
        self.linked(prefix).map(|(path, a)| Link { path: path.into(), entry: a.entry.clone() })
    }
```

- [ ] **Step 4: Implement `modified_secs`**

In `src/open.rs`, right after `resolve` (line 50), add:

```rust
/// When the file at `path` last changed, in Unix seconds: the Go to
/// list's age for a document whose note's date cannot be read. None when
/// it is not there or the filesystem keeps no such time.
pub fn modified_secs(path: &Path) -> Option<i64> {
    let modified = path.metadata().and_then(|m| m.modified()).ok()?;
    Some(modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64)
}
```

In `src/docs.rs` (lines ~94-100), replace the inline metadata chain inside `or_else` with the shared function:

```rust
        let Some(changed) = times.get(&path).copied().or_else(|| crate::open::modified_secs(&path)) else {
            continue;
        };
```

If `UNIX_EPOCH` is then unused in `docs.rs`, drop it from that file's `use` line (check with `cargo build` warnings).

- [ ] **Step 5: Run the tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all tests pass, and `cargo build 2>&1 | grep warning` shows no warning in `task.rs`, `open.rs` or `docs.rs`.

- [ ] **Step 6: Commit**

```bash
git add src/task.rs src/open.rs src/docs.rs
git commit -m "feat(task): read when a task's documents were linked

A task's report, plan and spec links now carry the date of the note
that links them, and open gains the file's last change for when that
date cannot be read: what the Go to list will show their ages from.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Carry a card's document links

**Files:**
- Modify: `src/panel/model.rs` — `Card` fields (lines 63-70), `Card::icon` (line 91), `Card::state` (lines 138-140), `cards` (lines 350-352), `cap` (lines 382-384), tests at ~556-583 and ~1012-1029
- Modify: `src/panel/state.rs:1049` (test helper `card`)

**Interfaces:**
- Consumes: `task::Link`, `Task::report_link`/`plan_link`/`spec_link` (Task 1).
- Produces: `Card.report: Option<Link>`, `Card.spec: Option<Link>`, `Card.plan: Option<Link>`; the fields `has_report`, `has_spec`, `has_plan` are gone from `Card` (they stay on `TaskState`).

- [ ] **Step 1: Update the model tests to the new fields (they fail to compile)**

In `src/panel/model.rs` tests:

- In the report test (~lines 556-560) replace the four asserts with:

```rust
        assert_eq!(
            by("reported").report,
            Some(crate::task::Link { path: "/r/a.html".into(), entry: "20261006T020000Z".into() })
        );
        assert_eq!(by("drafted").report, None, "an earlier draft's report is not the plan's");
        assert_eq!(by("plain").report, None);
        let many: Vec<Task> = (0..CAP + 1).map(|i| planned(&format!("t{i}"), false)).collect();
        assert_eq!(cap(&cards(&todo(many), &[]), CAP).last().unwrap().report, None);
```

- In `a_card_knows_its_tasks_spec_plan_and_claudes` replace `assert!(by("linked").has_spec && by("linked").has_plan);` and the `plain` line with:

```rust
        assert_eq!(by("linked").spec.as_ref().map(|l| l.entry.as_str()), Some("20261006T010000Z"));
        assert_eq!(by("linked").plan.as_ref().map(|l| l.path.as_str()), Some("docs/superpowers/plans/a.md"));
        assert!(by("plain").spec.is_none() && by("plain").plan.is_none());
```

  and `assert!(!more.has_spec && !more.has_plan);` with `assert!(more.spec.is_none() && more.plan.is_none());`. The `linked.has_spec && linked.has_plan` assert on the `TaskState` stays as it is.

- In `a_cards_state_is_its_tasks` (~line 1012) the closure's last parameter becomes a report link:

```rust
        let card = |status, planned, up_next, reported: bool| Card {
            status,
            text: "t".into(),
            uuid: Some("u".into()),
            id: 0,
            planned,
            up_next,
            since: String::new(),
            notes: Vec::new(),
            report: reported.then(|| crate::task::Link { path: "/r/a.html".into(), entry: "20261006T020000Z".into() }),
            spec: None,
            plan: None,
            processing: false,
        };
```

In `src/panel/state.rs:1049`, the test helper becomes:

```rust
        Card { status, text: uuid.into(), uuid: Some(uuid.into()), id: 0, planned: status == Status::Planned, up_next: false, since: String::new(), notes: Vec::new(), report: None, spec: None, plan: None, processing: false }
```

Search for any other `has_report`/`has_spec`/`has_plan` on a `Card` (not a `TaskState`): `grep -n "has_report\|has_spec\|has_plan" src/panel/model.rs src/panel/state.rs src/panel/surface.rs` and update the `Card` ones the same way.

- [ ] **Step 2: Run to see it fail**

Run: `cargo test --no-run 2>&1 | grep -E "^error" | head`
Expected: `struct Card has no field named report` (and `spec`, `plan`).

- [ ] **Step 3: Change `Card`**

In `src/panel/model.rs`, change the import to `use crate::task::{Link, Listing, Task};` and replace the three fields (lines 63-70) with:

```rust
    /// The refine report the task's notes link, `Report: <path>`, and when:
    /// its Go to list offers Open report, with its age, and Regenerate
    /// report. None on "+N more".
    pub report: Option<Link>,
    /// The design spec the task's notes link, `Spec: <path>`, and when, for
    /// its Go to list. None on "+N more".
    pub spec: Option<Link>,
    /// The implementation plan the task's notes link, `Plan: <path>`, and
    /// when, for its Go to list. None on "+N more".
    pub plan: Option<Link>,
```

In `Card::icon`: `Status::Planned if self.report.is_some() => "\u{f15c}",`

In `Card::state`:

```rust
            has_report: self.report.is_some(),
            has_spec: self.spec.is_some(),
            has_plan: self.plan.is_some(),
```

In `cards`:

```rust
            report: t.report_link(),
            spec: t.spec_link(),
            plan: t.plan_link(),
```

In `cap`'s "+N more" card: `report: None, spec: None, plan: None,` (one per line, as the file has them).

- [ ] **Step 4: Run the tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/panel/model.rs src/panel/state.rs
git commit -m "refactor(panel): carry a card's document links, not flags

A card keeps the report, plan and spec its task's notes link, with the
date of each linking note, in place of whether it has them, so the Go
to list can show how old each is.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Work out each document row's age

**Files:**
- Modify: `src/panel/goto.rs` — add `Since`, change `GoToList`, `GoToList::new`, add `row_age`, module doc, tests
- Modify: `src/panel/state.rs:736-750` (`show_goto`) and its tests (calls at ~1739, 1769, 1822, 1854, 1889, 1892)
- Modify: `src/panel/surface.rs:656-669` (`Effect::ListGoTo`)

**Interfaces:**
- Consumes: `task::Link` (Task 1), `Card.report`/`plan`/`spec` (Task 2), `open::resolve`, `open::modified_secs` (Task 1), `task::stamp_secs`, `task::stamp`, `task::age`.
- Produces:
  - `pub struct goto::Since { pub report: Option<String>, pub plan: Option<String>, pub spec: Option<String> }` — derives `Debug, Clone, Default, PartialEq, Eq`; each a taskwarrior stamp.
  - `Since::of(report: Option<&Link>, plan: Option<&Link>, spec: Option<&Link>, modified: &dyn Fn(&str) -> Option<i64>) -> Since`
  - `GoToList::new(uuid: &str, text: &str, state: TaskState, found: Found, since: Since) -> Option<GoToList>`
  - `GoToList::row_age(&self, item: Item, now: i64) -> Option<String>`
  - `PanelState::show_goto(&mut self, uuid: &str, found: Found, modified: &dyn Fn(&str) -> Option<i64>) -> Vec<Effect>`

- [ ] **Step 1: Write the failing tests in `goto.rs`**

Update the three existing `GoToList::new(...)` calls in `goto.rs`'s tests to pass `Since::default()` as a fifth argument. Then add:

```rust
    fn link(path: &str, entry: &str) -> crate::task::Link {
        crate::task::Link { path: path.into(), entry: entry.into() }
    }

    /// 2026-10-08T00:00:00Z, four days after the links below were made.
    const NOW: i64 = 1_791_417_600;

    /// Each document row shows how long ago its note linked it; no other
    /// row has an age, and a document not linked has none.
    #[test]
    fn document_rows_show_their_age_and_others_none() {
        let since = Since::of(
            Some(&link("/r/a.html", "20261004T000000Z")),
            Some(&link("docs/p.md", "20261007T000000Z")),
            None,
            &|_| panic!("every note's date reads"),
        );
        let list = GoToList::new("a", "task a", everything(), all_found(), since).unwrap();
        assert_eq!(list.row_age(OpenReport, NOW).as_deref(), Some("4d"));
        assert_eq!(list.row_age(Plan, NOW).as_deref(), Some("1d"));
        assert_eq!(list.row_age(Spec, NOW), None, "no spec linked");
        for item in Item::ALL.into_iter().filter(|i| !matches!(i, OpenReport | Plan | Spec)) {
            assert_eq!(list.row_age(item, NOW), None, "{item:?}");
        }
    }

    /// A regenerated report's newest note is the one its age counts from.
    #[test]
    fn a_regenerated_reports_age_counts_from_its_newest_note() {
        let t: crate::task::Task = serde_json::from_str(
            r#"{"uuid":"a","description":"d","annotations":[
                {"entry":"20261001T000000Z","description":"Report: /r/first.html"},
                {"entry":"20261007T120000Z","description":"Report: /r/second.html"}
            ]}"#,
        )
        .unwrap();
        let since = Since::of(t.report_link().as_ref(), None, None, &|_| None);
        let list = GoToList::new("a", "d", everything(), all_found(), since).unwrap();
        assert_eq!(list.row_age(OpenReport, NOW).as_deref(), Some("12h"));
    }

    /// A note's date that cannot be read gives way to the file's last
    /// change, found by the note's path; with neither, no age.
    #[test]
    fn an_unreadable_note_date_falls_back_to_the_files() {
        let modified = |path: &str| (path == "docs/p.md").then_some(NOW - 3 * 3600);
        let since = Since::of(Some(&link("/r/gone.html", "someday")), Some(&link("docs/p.md", "")), None, &modified);
        assert_eq!(since, Since { report: None, plan: Some("20261007T210000Z".into()), spec: None });
        let list = GoToList::new("a", "d", everything(), all_found(), since).unwrap();
        assert_eq!(list.row_age(Plan, NOW).as_deref(), Some("3h"));
        assert_eq!(list.row_age(OpenReport, NOW), None);
    }
```

`NOW` is checked (`date -u -d 2026-10-08 +%s` = `1791417600`); a first line in `document_rows_show_their_age_and_others_none` keeps it honest:

```rust
        assert_eq!(crate::task::stamp_secs("20261008T000000Z"), Some(NOW));
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test --no-run 2>&1 | grep -E "^error" | head`
Expected: `cannot find struct Since`, `this function takes 4 arguments but 5 were supplied`.

- [ ] **Step 3: Implement `Since`, the list's field and `row_age`**

In `src/panel/goto.rs`, after the `Found` struct, add:

```rust
/// When each document the task's notes link was made, as a taskwarrior
/// stamp, for the age at its row's right end: the linking note's date,
/// else the file's last change. Worked out as Go to… is pressed and never
/// saved. None for a document not linked, or with neither date.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Since {
    pub report: Option<String>,
    pub plan: Option<String>,
    pub spec: Option<String>,
}

impl Since {
    /// The stamps of the `report`, `plan` and `spec` a card links, asking
    /// `modified` for the Unix seconds a linked path last changed whenever
    /// a note's date is not taskwarrior's shape.
    pub fn of(
        report: Option<&Link>,
        plan: Option<&Link>,
        spec: Option<&Link>,
        modified: &dyn Fn(&str) -> Option<i64>,
    ) -> Since {
        let since = |link: Option<&Link>| {
            let link = link?;
            match task::stamp_secs(&link.entry) {
                Some(_) => Some(link.entry.clone()),
                None => modified(&link.path).map(task::stamp),
            }
        };
        Since { report: since(report), plan: since(plan), spec: since(spec) }
    }
}
```

Change the imports at the top to:

```rust
use super::style as c;
use crate::actions::{Action, TaskState};
use crate::task::{self, Link};
```

Add a field to `GoToList`, after `found`:

```rust
    /// When its documents were made, for their rows' ages.
    since: Since,
```

Replace `GoToList::new`:

```rust
    /// The list for the task `uuid`, described `text`, in `state`, with
    /// what was `found` and when its documents were made, the first row
    /// highlighted. None with no row: a processing task.
    pub fn new(uuid: &str, text: &str, state: TaskState, found: Found, since: Since) -> Option<GoToList> {
        let rows = Item::of(state, found);
        (!rows.is_empty()).then(|| GoToList { uuid: uuid.into(), text: text.into(), state, found, since, rows, at: 0 })
    }
```

After `row_label`, add:

```rust
    /// How long ago a document row's document was made, as a task card
    /// shows its age, `now` in Unix seconds: Open report, Planning doc and
    /// Design spec alone. None on every other row, and on a document with
    /// no date.
    pub fn row_age(&self, item: Item, now: i64) -> Option<String> {
        let since = match item {
            OpenReport => &self.since.report,
            Plan => &self.since.plan,
            Spec => &self.since.spec,
            _ => return None,
        };
        task::age(since.as_deref()?, now)
    }
```

In the module doc at the top of `goto.rs`, after "…what picking one runs." add the sentence: "A document's row shows how old it is, from the note linking it, else its file ([`Since`])."

- [ ] **Step 4: Run the goto tests**

Run: `cargo test --no-run 2>&1 | grep -E "^error" | head`
Expected: errors now only in `state.rs` (`GoToList::new` arity) and `surface.rs` once Step 5 lands. Continue to Steps 5-6 before running tests.

- [ ] **Step 5: Thread `Since` through `show_goto`**

In `src/panel/state.rs`, change line 21 to `use super::goto::{Found, GoToList, Since};`, and replace `show_goto` (lines ~736-750):

```rust
    /// What the surface found for Go to… on this task's card: its Go to
    /// list in place of the cards, the first row highlighted. `modified` is
    /// when a document's file last changed, for a document whose note's
    /// date cannot be read. Nothing without the keyboard, with a list
    /// already up, or for a card no longer there.
    pub fn show_goto(&mut self, uuid: &str, found: Found, modified: &dyn Fn(&str) -> Option<i64>) -> Vec<Effect> {
        if !self.keyboard || self.on_a_list() {
            return Vec::new();
        }
        let Some(card) = self.all.iter().find(|c| c.uuid.as_deref() == Some(uuid)) else { return Vec::new() };
        let since = Since::of(card.report.as_ref(), card.plan.as_ref(), card.spec.as_ref(), modified);
        let Some(list) = card.state(&self.agents).and_then(|state| GoToList::new(uuid, &card.text, state, found, since))
        else {
            return Vec::new();
        };
        self.mode = Mode::GoTo(list);
        vec![Effect::Render]
    }
```

Update each `show_goto(…)` call in `state.rs`'s tests (lines ~1739, 1769, 1822, 1854, 1889, 1892) to add `&|_| None` as the third argument, e.g. `state.show_goto("a", Found::default(), &|_| None)`.

Then add a state test beside `what_was_found_adds_its_rows`, built the way `going_from_a` builds its state but with a report linked on `a` by a note whose date cannot be read:

```rust
    /// A card's linked report reaches its Go to row's age, through the
    /// file's date when the note's cannot be read.
    #[test]
    fn a_linked_reports_date_reaches_its_row() {
        let mut a = card("a", Status::Active);
        a.planned = true;
        a.report = Some(crate::task::Link { path: "/r/a.html".into(), entry: "someday".into() });
        let mut state = PanelState::default();
        state.set_cards(&[a, card("b", Status::Pending)]);
        assert!(state.take_keyboard(vec![crate::names::refine_agent("a")]));
        assert_eq!(key(&mut state, KeyAction::Run(Action::GoTo)), vec![Effect::ListGoTo("a".into())]);
        let now = crate::task::stamp_secs("20261008T000000Z").unwrap();
        state.show_goto("a", Found::default(), &|path| (path == "/r/a.html").then_some(now - 120));
        let list = state.goto().unwrap();
        assert!(list.rows().contains(&Item::OpenReport));
        assert_eq!(list.row_age(Item::OpenReport, now).as_deref(), Some("2m"));
    }
```

- [ ] **Step 6: Supply the file date from the surface**

In `src/panel/surface.rs`, replace the `Effect::ListGoTo(uuid)` arm (lines ~656-669):

```rust
                Effect::ListGoTo(uuid) => {
                    // Looked at on every press: a worktree made, VS Code
                    // installed or a speech started since shows.
                    let ws = crate::workspace::Workspace::focused().ok();
                    let worktree = ws.as_ref().and_then(|ws| crate::work::worktree_path(ws, &uuid));
                    let folder = ws.as_ref().zip(crate::dirs::Dirs::from_env().ok()).map(|(ws, dirs)| ws.folder(&dirs));
                    let found = Found {
                        worktree: worktree.is_some(),
                        editor: crate::programs::on_path(crate::programs::EDITOR),
                        speaking: crate::speak::speaking_now(),
                    };
                    // A document whose note's date cannot be read is aged
                    // by its file, found where `task open` would open it.
                    let modified = |linked: &str| {
                        crate::open::modified_secs(&crate::open::resolve(linked, worktree.as_deref(), folder.as_deref()?))
                    };
                    let effects = self.state.borrow_mut().show_goto(&uuid, found, &modified);
                    self.apply(effects);
                }
```

- [ ] **Step 7: Run all tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass, including the four new tests. `cargo build 2>&1 | grep warning` shows nothing new.

- [ ] **Step 8: Commit**

```bash
git add src/panel/goto.rs src/panel/state.rs src/panel/surface.rs
git commit -m "feat(panel): work out the Go to list's document ages

The Go to list learns when its report, plan and spec were made: the
date of the note linking each, else the file's last change. Nothing is
saved; the dates are read each time Go to… is pressed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Draw the age on a document row

**Files:**
- Modify: `src/panel/surface.rs` — `draw_projects` (~line 932), `draw_goto` (~line 944), `row_card` (~lines 1090-1125)
- Modify: `CONTEXT.md:202-224` (Go to list entry)

**Interfaces:**
- Consumes: `GoToList::row_age(item, now)` (Task 3), `task::now_secs()`.
- Produces: `row_card(self: &Rc<Self>, at: usize, label: &str, age: Option<&str>, class: Option<&str>, pick: fn(&mut PanelState, usize) -> Vec<Effect>) -> (gtk4::Box, gtk4::Button)`.

The drawing is GTK and has no unit test in this repo; the age's value is already tested in Task 3. This task is checked by building and by looking at the panel.

- [ ] **Step 1: Give `row_card` an optional age**

Replace `row_card` in `src/panel/surface.rs` with:

```rust
    /// One row of either list, drawn as a task card is: a box holding a
    /// body button with the row's words and, on a Go to list's document,
    /// its age at the right end, as a card's. A click on it runs `pick` on
    /// the state with its index. Out of the focus chain: the project
    /// list's text field keeps the keyboard, the Go to list leaves it on
    /// the window, and the highlight (`sync`) says which row Enter takes.
    fn row_card(
        self: &Rc<Self>,
        at: usize,
        label: &str,
        age: Option<&str>,
        class: Option<&str>,
        pick: fn(&mut PanelState, usize) -> Vec<Effect>,
    ) -> (gtk4::Box, gtk4::Button) {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.add_css_class("task-card");
        if let Some(class) = class {
            root.add_css_class(class);
        }
        root.set_size_request(CARD_WIDTH_PX, -1);
        root.set_overflow(gtk4::Overflow::Hidden);
        // The words take the room, pushing the age to the right end, as
        // `card_label` lays out a card.
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let text = gtk4::Label::new(Some(label));
        text.set_xalign(0.0);
        text.set_hexpand(true);
        row.append(&text);
        if let Some(age) = age {
            let age = gtk4::Label::new(Some(age));
            age.add_css_class("card-age");
            row.append(&age);
        }
        let body = gtk4::Button::builder().child(&row).build();
        body.add_css_class("card-body");
        body.set_focusable(false);
        body.set_focus_on_click(false);
        let weak = Rc::downgrade(self);
        body.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                let effects = pick(&mut p.state.borrow_mut(), at);
                p.apply(effects);
            }
        });
        root.append(&body);
        (root, body)
    }
```

- [ ] **Step 2: Pass the ages in**

In `draw_projects`: `let card = self.row_card(at, &ProjectList::label(row), None, None, PanelState::on_folder);`

In `draw_goto`, replace the loop:

```rust
        let now = crate::task::now_secs();
        for (at, item) in list.rows().iter().enumerate() {
            let age = list.row_age(*item, now);
            let row = self.row_card(at, &list.row_label(*item), age.as_deref(), Some(item.group().class()), PanelState::on_goto_row);
            self.column.append(&row.0);
            rows.push(row);
        }
```

- [ ] **Step 3: Build and test**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" | head; cargo test 2>&1 | tail -3`
Expected: no errors or new warnings; all tests pass.

- [ ] **Step 4: Update `CONTEXT.md`**

In the **Go to list** entry, after "…reading aloud always." insert:

```
Each document's row shows how old it is at its right end, dimmed as a
card's age is: from the note linking it, else the file's last change;
worked out as the list is drawn, never saved.
```

Re-wrap the paragraph at the file's width if the lines run long.

- [ ] **Step 5: Look at it**

Use the `run` skill (or the repo's `try.sh`, per its header) to bring up the task panel from this worktree. Open Go to… (`o`) on a task whose notes link a report, plan or spec — this task itself has `Plan:` once the plan is linked. Check: each document row has its age, dimmed and right-aligned like a card's; Refine, sessions, worktree, VS Code and Read aloud have none; the project list (Move) rows have none. Confirm the task's notes are unchanged: `task rc.json.array=on <uuid> export` before and after shows the same annotations. Show the user a screenshot if the run skill can take one.

- [ ] **Step 6: Commit**

```bash
git add src/panel/surface.rs CONTEXT.md
git commit -m "feat(panel): show document ages on the Go to list

Open report, Planning doc and Design spec each show how old they are
at the row's right end, dimmed as a task card shows its age. Other
rows show none.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
