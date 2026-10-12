# Open a `~` or Relative Report Note Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A hand-written `Report: ~/...` or `Report: name.html` note opens the file from the card and `niritasks task report`, and is listed by `niritasks task docs`, as an absolute one is.

**Architecture:** One pure function, `refine::report_file(path, &Dirs) -> PathBuf`, beside `refine::reports_dir`, turns a report note's text into a file path: `~` or `~/...` under the home folder, a relative path under the reviews folder, and an absolute path as written. The `task report` command in `src/main.rs` and `docs::reports` in `src/docs.rs` both call it before checking `is_file`, so the card's Open report (which runs `task report`) and `task docs` look in the same place. `Task::report_path` and `Task::report_notes` keep returning the note's text as written, and the card's `has_report` still only checks that a note is there.

**Tech Stack:** Rust (edition 2021), `cargo test`. The tests use `Dirs::at` and scratch folders under `std::env::temp_dir()`.

**Spec:** the description and notes of Taskwarrior task `e1aaecb7-e6b5-4aa1-ba9c-b70925104c68` (read it with `task rc.json.array=on e1aaecb7-e6b5-4aa1-ba9c-b70925104c68 export`).

## Global Constraints

- Resolve, do not refuse: `~` or `~/...` expands to `Dirs::home`; a relative path is taken under `refine::reports_dir`; an absolute path is used as written.
- Resolution lives in `refine.rs` beside `reports_dir` as a pure fn taking `&Dirs`, so it is unit tested with `Dirs::at`.
- `Task::report_path` and `Task::report_notes` keep returning the raw text; the card's `has_report` (`src/actions.rs:106`) stays note-presence only. Do not touch them.
- `docs::reports` resolves each note with the same fn, taking `&Dirs` from `docs::list`.
- A resolved path that is not a file still falls through with the "Report file gone" notification.
- Out of scope: `~user/...`, `$HOME` or other shell forms; changing what the mod writes (`claude/refine-mod/hooks/report.ts`); `task open-report`; the panel resolving paths; `refine::report_wrote` (it compares note text the mod wrote, which is always absolute).
- Commits: Conventional Commits, `fix(report): …` or `docs: …`, subject ≤ 72 chars, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

## File Structure

- `src/refine.rs`: add `pub fn report_file` right after `reports_dir` (line ~95), and its test next to `reports_go_in_the_reviews_folder` (line ~924).
- `src/main.rs`: change the `TaskCommand::Report` arm (lines ~481–510) to resolve the note's path before `is_file` and `open_document`.
- `src/docs.rs`: `reports` takes `&Dirs` and resolves each note; `list` passes its `dirs`; update the existing `reports` test and add one for the `~` and relative forms.
- `llms.txt` rows for `task report` (line 46) and `task docs` (line 48), and the README's **Open report** row (line 96): say that a `~` or relative path counts.

---

### Task 1: Resolve a report note's path, and use it in `task report`

**Files:**
- Modify: `src/refine.rs:92-97` (add the fn after `reports_dir`)
- Modify: `src/main.rs:481-510` (the `TaskCommand::Report` arm)
- Test: `src/refine.rs`, in `mod tests`, after `reports_go_in_the_reviews_folder`

**Interfaces:**
- Consumes: `crate::dirs::Dirs` (`Dirs::at(&Path)`, `Dirs::home() -> &Path`, `Dirs::with_data(&Path)`); `refine::reports_dir(&Dirs) -> PathBuf`.
- Produces: `pub fn report_file(path: &str, dirs: &Dirs) -> PathBuf` in `niri_tasks::refine`. Task 2 calls it as `refine::report_file(&n.path, dirs)`.

- [ ] **Step 1: Write the failing test**

In `src/refine.rs`, inside `mod tests`, right after the `reports_go_in_the_reviews_folder` test, add:

```rust
    /// A report note's path as typed: `~` is the home folder, a relative
    /// path is in the reviews folder, and an absolute one is as written.
    #[test]
    fn a_report_note_is_found_from_home_the_reviews_folder_or_as_written() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(report_file("~", &dirs), PathBuf::from("/home/x"));
        assert_eq!(
            report_file("~/.local/share/niri-tasks/reviews/r.html", &dirs),
            PathBuf::from("/home/x/.local/share/niri-tasks/reviews/r.html")
        );
        assert_eq!(report_file("r.html", &dirs), PathBuf::from("/home/x/.local/share/niri-tasks/reviews/r.html"));
        assert_eq!(report_file("old/r.html", &dirs), PathBuf::from("/home/x/.local/share/niri-tasks/reviews/old/r.html"));
        assert_eq!(
            report_file("r.html", &dirs.clone().with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/reviews/r.html")
        );
        assert_eq!(report_file("/r/r.html", &dirs), PathBuf::from("/r/r.html"));
    }
```

- [ ] **Step 2: Run the test to see it fail**

Run: `cargo test --lib refine::tests::a_report_note_is_found`
Expected: a compile error, `cannot find function `report_file` in this scope`.

- [ ] **Step 3: Write the fn**

In `src/refine.rs`, right after `pub fn reports_dir`, add:

```rust
/// The file a `Report: <path>` note names. The mod always writes an absolute
/// path, which is used as written; a hand-written `~` or `~/...` is under the
/// home folder, and a relative path is in [`reports_dir`].
pub fn report_file(path: &str, dirs: &Dirs) -> PathBuf {
    if path == "~" {
        return dirs.home().to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return dirs.home().join(rest);
    }
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        reports_dir(dirs).join(p)
    }
}
```

`Path` and `PathBuf` are already imported at the top of `refine.rs`.

- [ ] **Step 4: Run the test to see it pass**

Run: `cargo test --lib refine::tests::a_report_note_is_found`
Expected: PASS.

- [ ] **Step 5: Resolve the note in `task report`**

In `src/main.rs`, in the `TaskCommand::Report { uuid, fresh }` arm, replace this block:

```rust
            let mut gone = None;
            if !fresh {
                if let Some(path) = t.report_path() {
                    if Path::new(path).is_file() {
                        open::open_document(Path::new(path), Workspace::of_caller()?.name())?;
                        notify::tasks(&format!("Opened the report: {}", t.description));
                        return Ok(());
                    }
                    gone = Some(path.to_string());
                }
            }
```

with:

```rust
            let mut gone = None;
            if !fresh {
                if let Some(path) = t.report_path() {
                    // A note typed by hand may say `~/...` or just the file's
                    // name in the reviews folder.
                    let file = refine::report_file(path, &dirs::Dirs::from_env()?);
                    if file.is_file() {
                        open::open_document(&file, Workspace::of_caller()?.name())?;
                        notify::tasks(&format!("Opened the report: {}", t.description));
                        return Ok(());
                    }
                    gone = Some(path.to_string());
                }
            }
```

Leave the rest of the arm alone: a resolved path that is not a file sets `gone`, and the `Report file gone, building a fresh one` notification stays as it is. `dirs` and `refine` are already in the `use niri_tasks::{…}` list at the top of `main.rs`. If `Path` is no longer used anywhere in `main.rs`, `cargo build` warns; then remove it from `use std::path::{Path, PathBuf};` (check with `grep -n 'Path::' src/main.rs` first: it is used elsewhere today).

- [ ] **Step 6: Build and run every test**

Run: `cargo build && cargo test`
Expected: no warnings from this change; all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/refine.rs src/main.rs
git commit -m "fix(report): open a report note with a ~ or relative path

A hand-typed Report: note naming ~/... or just a file in the reviews
folder was checked as written, so task report and the card's Open
report built a fresh report instead of opening it. Resolve the note
first: ~ under home, relative under the reviews folder, absolute as
written.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: List a `~` or relative report in `task docs`, and document both commands

**Files:**
- Modify: `src/docs.rs:7` (imports), `src/docs.rs:33-51` (`reports`), `src/docs.rs:185-187` (`list`)
- Test: `src/docs.rs`, `mod tests`: update `reports_prefer_the_twin_and_skip_what_is_gone` and add a new test after it
- Modify: `llms.txt:46`, `llms.txt:48`, `README.md:96`

**Interfaces:**
- Consumes: `niri_tasks::refine::report_file(path: &str, dirs: &Dirs) -> PathBuf` from Task 1; `refine::reports_dir(&Dirs) -> PathBuf`.
- Produces: `pub fn reports(t: &Task, dirs: &Dirs) -> Vec<Doc>` (was `reports(t: &Task)`). Its only callers are `docs::list` and the tests in `docs.rs`; check with `grep -rn 'reports(&' src`.

- [ ] **Step 1: Write the failing test, and update the existing one**

In `src/docs.rs`, `mod tests`, change the existing call in `reports_prefer_the_twin_and_skip_what_is_gone` from

```rust
            reports(&t),
```

to

```rust
            reports(&t, &Dirs::at(&dir)),
```

(its notes are absolute, so the home it is given does not matter). Then add, right after that test:

```rust
    /// A note typed as `~/...` or as a bare name in the reviews folder is
    /// found there, twin first, as an absolute one is.
    #[test]
    fn a_report_note_may_be_under_home_or_in_the_reviews_folder() {
        let home = scratch("reports-relative");
        let dirs = Dirs::at(&home);
        let reviews = crate::refine::reports_dir(&dirs);
        file_at(&reviews.join("x.html"), "<html>", 1);
        file_at(&reviews.join("x.md"), "# x", 1);
        file_at(&reviews.join("y.html"), "<html>", 1);
        let t = task(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report (earlier draft): y.html"},
                {"entry":"20261007T010000Z","description":"Report: ~/.local/share/niri-tasks/reviews/x.html"}
            ]}"#,
        );
        assert_eq!(
            reports(&t, &dirs),
            vec![
                Doc { label: "Report · 2026-10-07".into(), path: reviews.join("x.md") },
                Doc {
                    label: "Report, earlier draft · 2026-10-06 · opens in the browser".into(),
                    path: reviews.join("y.html"),
                },
            ]
        );
        std::fs::remove_dir_all(&home).ok();
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib docs::tests::reports`
then `cargo test --lib docs::tests::a_report_note_may_be`
Expected: a compile error, `this function takes 1 argument but 2 arguments were supplied`.

- [ ] **Step 3: Resolve each note in `reports`, and pass `dirs` from `list`**

In `src/docs.rs`, change the import line

```rust
use crate::{dirs::Dirs, names, task, task::Task, text};
```

to

```rust
use crate::{dirs::Dirs, names, refine, task, task::Task, text};
```

Replace `reports` with:

```rust
/// The task's reports, newest first: each one's Markdown twin, or its HTML
/// when it has none. A note's path is found as [`refine::report_file`] finds
/// it, so a `~` or relative one counts. A report whose files are both gone is
/// left out.
pub fn reports(t: &Task, dirs: &Dirs) -> Vec<Doc> {
    t.report_notes()
        .into_iter()
        .filter_map(|n| {
            let kind = if n.draft { "Report, earlier draft" } else { "Report" };
            let html = refine::report_file(&n.path, dirs);
            let md = twin(&html);
            if md.is_file() {
                Some(Doc { label: format!("{kind} · {}", n.date), path: md })
            } else if html.is_file() {
                Some(Doc { label: format!("{kind} · {} · opens in the browser", n.date), path: html })
            } else {
                None
            }
        })
        .collect()
}
```

In `list`, change

```rust
    let mut docs = reports(t);
```

to

```rust
    let mut docs = reports(t, dirs);
```

If `PathBuf` is now unused in `docs.rs`, the build warns; it is still used by `Doc`, `twin` and `linked`, so it should not be.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --lib docs::tests`
Expected: PASS, the new test and `reports_prefer_the_twin_and_skip_what_is_gone` included.

- [ ] **Step 5: Document it**

In `llms.txt` line 46 (the `niritasks task report <uuid>` row), change

```
On a task whose notes link a report as `Report: <path>`, open that file,
```

to

```
On a task whose notes link a report as `Report: <path>` (absolute, `~/...`, or relative to `$XDG_DATA_HOME/niri-tasks/reviews`), open that file,
```

In `llms.txt` line 48 (the `niritasks task docs` row), change

```
the reports its notes link, newest first (each one's Markdown twin, or its HTML when made before twins);
```

to

```
the reports its notes link, newest first, a `~/...` or relative path found as `task report` finds it (each one's Markdown twin, or its HTML when made before twins);
```

In `README.md` line 96 (the **Open report** row), change

```
| **Open report** | its notes link a report (`Report: <path>`): an HTML one
```

to

```
| **Open report** | its notes link a report (`Report: <path>`, the path absolute, `~/...`, or a name in `~/.local/share/niri-tasks/reviews`): an HTML one
```

Each is a single line in its file; edit within the line, keep the rest of it unchanged.

- [ ] **Step 6: Run every test**

Run: `cargo test`
Expected: all pass, including `llms_txt_runs_every_subcommand_and_flag` and `llms_txt_has_the_llmstxt_shape` in `src/main.rs`.

- [ ] **Step 7: Commit**

```bash
git add src/docs.rs llms.txt README.md
git commit -m "fix(report): list a ~ or relative report note in task docs

task docs checked a Report: note's text as written, so a hand-typed
~/... or bare name was left out. Find it as task report now does, so
the docs list and the card agree on where a report is, and say in
llms.txt and the README that such a path counts.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Checking it by hand (after both tasks)

With `cargo install --path .` done, on a scratch task with a report `x.html` (and `x.md`) in `~/.local/share/niri-tasks/reviews`:

- `niritasks task note <uuid> "Report: x.html"` → `niritasks task report <uuid>` opens x.html; `niritasks task docs <uuid>` lists `Report · <date>\t…/reviews/x.md`.
- `niritasks task note <uuid> "Report: ~/.local/share/niri-tasks/reviews/x.html"` → the same.
- Rename x.html away → `task report` notifies `Report file gone, building a fresh one`.
