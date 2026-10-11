# Report Icon on a Planned Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A planned task whose notes link a refine report shows the Report button's icon in place of its filled dot, so the card says at a glance that a report is there to open.

**Architecture:** `Card::icon()` in `src/panel/model.rs` picks the status icon from `Card.status`. It gains one more arm: `Status::Planned` with `has_report` set returns Font Awesome's file-text, `\u{f15c}`, the glyph the Report button already wears in `src/actions.rs`. `Card.has_report` already exists, set by `cards()` from `Task::report_path()`, which reads the newest `Report: <path>` note and ignores `Report (earlier draft): ` notes. No new `Status`, no new CSS class, nothing in the filter tabs, the action row or the up-next yellow changes.

**Tech Stack:** Rust (`cargo test`), GTK4 panel, Nerd Font glyphs.

**Spec:** Taskwarrior task `dd260b16-5eaf-425c-827d-ae7887d724cd`, "feat(panel): swap the dot for a report icon". Read it with `task rc.json.array=on dd260b16-5eaf-425c-827d-ae7887d724cd export`; its notes are the spec and are quoted in **Global Constraints**.

## Global Constraints

- **The glyph:** Font Awesome's file-text, `U+F15C`, written `"\u{f15c}"`: the Report button's own icon in `src/actions.rs` (`Report => Facts { icon: "\u{f15c}", … }`), as a waiting card wears the Waiting button's icon.
- **What gives way:** the filled dot `●` only. `icon()` returns the report glyph when `self.status == Status::Planned && self.has_report`. An active card keeps `▶`; a blocked, processing, waiting or finished card keeps its icon, as those statuses already outrank `Planned` in `cards()`.
- **What counts as a report:** `Card.has_report`, already set in `cards()` from `t.report_path().is_some()`: the newest `Report: ` note, never a `Report (earlier draft): ` note. Nothing in `Task::report_path()` changes.
- **No new state:** no new `Status` variant, no new CSS class, no change to the filter tabs, the action row (`Card::state`, `TaskState`) or `shows_up_next()`.
- **Docs:** the README's icon sentence (around line 31) gains the report icon; CONTEXT.md's **Planned task** entry says filled dot, not clipboard-check; its **Refine report** entry says the card's dot becomes the report icon once linked.
- **Done when:** `cargo test` passes; on the panel a planned task with a `Report:` note shows the report glyph on the hover panel and in the peek, starting it turns it `▶` and stopping it brings the glyph back; README and CONTEXT.md match.
- **Out of scope:** a Status or filter tab for reported tasks; the icon's colour; the Report button and the report itself.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. The task's type is `feat`, scope `panel`; the docs commit is `docs`.
- **Tests stay green at every commit.**

## File Structure

- `src/panel/model.rs` (modify): `Card::icon()` gains the report arm; a `reported()` test helper and one test beside `planned_tasks_get_the_filled_dot`.
- `README.md` (modify): the panel's icon sentence, lines 29–32.
- `CONTEXT.md` (modify): the **Planned task** and **Refine report** entries, lines 49–62.

---

### Task 1: The report glyph on a planned card

**Files:**
- Modify: `src/panel/model.rs:73-91` (`Card::icon`)
- Test: `src/panel/model.rs` (the `tests` module: a helper after `processing()` near line 425, a test after `planned_tasks_get_the_filled_dot` near line 568)

**Interfaces:**
- Consumes: `Card.status: Status`, `Card.has_report: bool` (both exist), `crate::task::Annotation { entry, description }`, `crate::task::PROCESSING_TAG`, the test helpers `task(uuid, day, active)`, `planned(uuid, active)`, `todo(pending)` and `cards(&Listing, &[String])`.
- Produces: `Card::icon()` returns `"\u{f15c}"` for a planned card with `has_report`. No signature changes.

- [ ] **Step 1: Add the `reported` test helper**

In the `tests` module of `src/panel/model.rs`, directly after the `processing` helper (the `fn processing(uuid: &str) -> Task` block ending around line 425), add:

```rust
    /// A planned task whose notes link a refine report.
    fn reported(uuid: &str, active: bool) -> Task {
        let mut t = planned(uuid, active);
        t.annotations = vec![crate::task::Annotation {
            entry: "20261006T020000Z".into(),
            description: "Report: /r/a.html".into(),
        }];
        t
    }
```

- [ ] **Step 2: Write the failing test**

Directly after `planned_tasks_get_the_filled_dot` (which ends `assert_eq!(got[0].icon(), "●"); }` around line 568), add:

```rust
    /// A planned task whose notes link a report wears the Report button's
    /// glyph instead of the filled dot, so the card says a report is there
    /// to open. Only the dot gives way: started, blocked and processing
    /// outrank planned and keep their icons, and an earlier draft's note is
    /// not the plan's report.
    #[test]
    fn a_planned_task_with_a_report_wears_the_report_glyph() {
        let got = cards(&todo(vec![reported("p", false)]), &[]);
        assert_eq!(got[0].status, Status::Planned);
        assert_eq!(got[0].icon(), "\u{f15c}");
        assert_eq!(cards(&todo(vec![planned("q", false)]), &[])[0].icon(), "●", "no report, the dot stays");

        assert_eq!(cards(&todo(vec![reported("s", true)]), &[])[0].icon(), "▶");
        assert_eq!(cards(&todo(vec![reported("b", false)]), &["b".into()])[0].icon(), "\u{f023}");
        let mut busy = reported("h", false);
        busy.tags.push(crate::task::PROCESSING_TAG.into());
        assert_eq!(cards(&todo(vec![busy]), &[])[0].icon(), "\u{f252}");

        let mut drafted = planned("d", false);
        drafted.annotations = vec![crate::task::Annotation {
            entry: "20261006T010000Z".into(),
            description: "Report (earlier draft): /r/old.html".into(),
        }];
        assert_eq!(cards(&todo(vec![drafted]), &[])[0].icon(), "●", "an earlier draft's report is not the plan's");
    }
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test a_planned_task_with_a_report_wears_the_report_glyph`
Expected: FAIL on the second assertion, `left: "●"`, `right: "\u{f15c}"`.

- [ ] **Step 4: Add the report arm to `Card::icon`**

In `src/panel/model.rs`, `Card::icon()`, replace the `Planned` arm:

```rust
            // The pending circle filled in: still waiting, but worked up.
            Status::Planned => "●",
```

with:

```rust
            // Font Awesome's file-text, the Report button's own icon: a plan
            // with a report to open, as a waiting card wears Waiting's pause.
            Status::Planned if self.has_report => "\u{f15c}",
            // The pending circle filled in: still waiting, but worked up.
            Status::Planned => "●",
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test panel::model`
Expected: every test passes, `a_planned_task_with_a_report_wears_the_report_glyph` and `planned_tasks_get_the_filled_dot` among them.

Then run: `cargo test`
Expected: PASS, nothing else was touched.

- [ ] **Step 6: Commit**

```bash
git add src/panel/model.rs
git commit -m "feat(panel): show the report icon on a planned card with a report" -m "A planned task whose notes link a refine report now wears the Report
button's file-text glyph in place of its filled dot, so the card says at
a glance that a report is there to open, as a waiting card wears the
Waiting button's pause. Only the dot gives way: started, blocked and
processing already outrank planned and keep their icons, and an earlier
draft's note still does not count as the plan's report.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: Docs

**Files:**
- Modify: `README.md:29-32` (the panel's icon sentence)
- Modify: `CONTEXT.md:49-62` (the **Planned task** and **Refine report** entries)

**Interfaces:**
- Consumes: the behaviour from Task 1; the button is called **Open report** on a card with a report (`Action::label` in `src/actions.rs`).
- Produces: nothing code depends on.

- [ ] **Step 1: README's icon sentence**

In `README.md`, replace these lines (29–32):

```
blocked ones last: ○, a dimmed lock for one blocked on
another task, or ● for one already worked up into a plan with
Claude from a card's **Refine** or **Grill me**. Up to eight, then a "+N
more" card. Each card ends in how long ago its task was added: 5m, 3h, 2d, 4w.
```

with:

```
blocked ones last: ○, a dimmed lock for one blocked on
another task, ● for one already worked up into a plan with
Claude from a card's **Refine** or **Grill me**, or a report icon for a
planned one whose notes link a refine report, which its **Open report**
opens. Up to eight, then a "+N more" card. Each card ends in how long ago
its task was added: 5m, 3h, 2d, 4w.
```

- [ ] **Step 2: CONTEXT.md's Planned task entry**

In `CONTEXT.md`, replace:

```
and a clipboard-check icon on its task card.
```

with:

```
and a filled dot on its task card.
```

- [ ] **Step 3: CONTEXT.md's Refine report entry**

In `CONTEXT.md`, replace:

```
reviews folder and linked from the task's notes as `Report: <path>`; once
linked, the card's Report opens it, and `niritasks task report <uuid> --fresh`
builds another.
```

with:

```
reviews folder and linked from the task's notes as `Report: <path>`; once
linked, the card's dot becomes the report icon, its Report opens it, and
`niritasks task report <uuid> --fresh` builds another.
```

- [ ] **Step 4: Check the docs still read true and tests still pass**

Run: `grep -n "clipboard" README.md CONTEXT.md`
Expected: only README's `tests/e2e-panel.sh` row, which is about the system clipboard, not the icon.

Run: `cargo test`
Expected: PASS (README has command-block tests; the sentence edited here is prose, but confirm).

- [ ] **Step 5: Commit**

```bash
git add README.md CONTEXT.md
git commit -m "docs: describe the report icon on a planned card" -m "The README's icon sentence gains the report icon a planned card wears
once its notes link a refine report. CONTEXT.md's Planned task entry
said clipboard-check where the card has shown a filled dot since the
icon was changed; it now says filled dot, and the Refine report entry
says the dot becomes the report icon once the report is linked.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Final check: on the panel

Not a code task. Run it once both tasks are committed and the daemon is running the new build (`cargo build` then restart the `niri-tasks` daemon the way this machine does, or let the `finish-worktree` skill's install step do it). The panel watches taskwarrior's database, so plain `task` commands are enough.

- [ ] **Step 1: Make a scratch planned task with a report note**

```bash
UUID=$(niritasks task add "chore: scratch report icon check" | grep -oE '[0-9a-f]{8}-[0-9a-f-]{27}' | head -1)
task "$UUID" modify +planned
task "$UUID" annotate "Report: /tmp/scratch-report.html"
```

If `niritasks task add` does not print the uuid, find it with `task +planned rc.json.array=on export | python3 -c 'import json,sys; print([t["uuid"] for t in json.load(sys.stdin) if "scratch report icon" in t["description"]][0])'`.

- [ ] **Step 2: Look**

Expected: in the peek and on the hover panel the card shows the file-text glyph where a planned card shows `●`; the card is otherwise unchanged (still under the Planned tab, still its usual colour; the Report button reads **Open report**).

- [ ] **Step 3: Start and stop**

```bash
task "$UUID" start
```
Expected: the card turns `▶` and green.

```bash
task "$UUID" stop
```
Expected: the file-text glyph is back.

- [ ] **Step 4: Clean up**

```bash
task "$UUID" delete
```
