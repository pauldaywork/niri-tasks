//! End-to-end tests for the taskwarrior write path, against a sandboxed
//! database.
//!
//! `TASKRC` and `TASKDATA` point at a scratch directory, so none of this
//! touches the real `~/.task`. That matters: these tests add, modify, annotate,
//! complete and delete tasks.
//!
//! The two assertions that earn this file are `add_parses_taskwarrior_attributes`
//! and `edit_keeps_attribute_syntax_as_literal_text` — the deliberately
//! opposite quoting rules that the shell version encoded with `set -f` and an
//! unquoted expansion on one side, and `--` on the other. Getting them backwards
//! is silent: you would only notice when a task called "due:friday" appeared
//! with no due date, or a due date appeared from text you meant literally.
//!
//! Everything runs in one test function, sequentially, because the sandbox is
//! process-global state.

use niri_tasks::{task, text};
use std::path::PathBuf;

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-write-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).expect("create sandbox");
        std::fs::write(
            dir.join("taskrc"),
            format!("data.location={}/data\n", dir.display()),
        )
        .expect("write taskrc");

        std::env::set_var("TASKRC", dir.join("taskrc"));
        std::env::set_var("TASKDATA", dir.join("data"));

        Self { dir }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Read a raw field off a task, for the attribute assertions.
fn raw(uuid: &str, field: &str) -> String {
    let out = std::process::Command::new("task")
        .args(["rc.verbose=nothing", "rc.json.array=on", uuid, "export"])
        .output()
        .expect("task export");
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    json.get(0)
        .and_then(|t| t.get(field))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[test]
fn write_path_lifecycle() {
    let _sandbox = Sandbox::new();
    const TAG: &str = "sandbox";

    // ---- add ------------------------------------------------------------
    task::add(TAG, &text::add_args("write the thing")).expect("add");
    let tasks = task::pending_for_tag(TAG).expect("list");
    assert_eq!(tasks.len(), 1, "task should be tagged and pending");
    assert_eq!(tasks[0].description, "write the thing");
    assert!(!tasks[0].is_active());
    assert!(!tasks[0].has_notes());

    // ---- add parses taskwarrior attributes -------------------------------
    // The whole reason the description is word-split.
    task::add(TAG, &text::add_args("ship the release due:2026-09-01 priority:H"))
        .expect("add with attributes");
    let with_attrs = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description.starts_with("ship the release"))
        .expect("find the attributed task");

    assert_eq!(
        with_attrs.description, "ship the release",
        "due:/priority: must be consumed as attributes, not left in the description"
    );
    assert!(
        !raw(&with_attrs.uuid, "due").is_empty(),
        "due date should have been set"
    );
    assert_eq!(raw(&with_attrs.uuid, "priority"), "H");

    // ---- edit does NOT split --------------------------------------------
    // The opposite rule: text typed into the edit box stays literal.
    let uuid = tasks[0].uuid.clone();
    task::modify_description(&uuid, "remember due:2026-09-01 is just text here").expect("modify");
    let edited = task::get(&uuid).expect("get").expect("task exists");
    assert_eq!(
        edited.description, "remember due:2026-09-01 is just text here",
        "edit must pass text through as one argument, leaving attributes literal"
    );
    assert!(
        raw(&uuid, "due").is_empty(),
        "editing must not have set a due date from the literal text"
    );

    // ---- annotate --------------------------------------------------------
    task::annotate(&uuid, "a note with due:2026-09-01 in it").expect("annotate");
    let annotated = task::get(&uuid).expect("get").expect("exists");
    assert!(annotated.has_notes());
    assert_eq!(annotated.annotations.len(), 1);
    assert_eq!(
        annotated.annotations[0].description,
        "a note with due:2026-09-01 in it",
        "annotations keep attribute syntax literal too"
    );

    // ---- add with notes --------------------------------------------------
    // The add box's second text area: one annotation per line, attached to the
    // task that was just created. The uuid comes back from `task add` itself,
    // so the notes cannot land on somebody else's task.
    let notes = text::note_lines("first note\n\nsecond note with due:2026-09-01\n");
    task::add_with_notes(TAG, &text::add_args("raised with notes"), &notes)
        .expect("add with notes");

    let with_notes = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "raised with notes")
        .expect("find the task added with notes");

    assert_eq!(
        with_notes.annotations.len(),
        2,
        "each line is its own annotation, and the blank line is not one"
    );
    assert_eq!(with_notes.annotations[0].description, "first note");
    assert_eq!(
        with_notes.annotations[1].description, "second note with due:2026-09-01",
        "notes go through as one argument, so attribute syntax stays literal"
    );
    assert!(
        raw(&with_notes.uuid, "due").is_empty(),
        "a due: inside a note must not become the task's due date"
    );

    // A task added with no notes is just a task — no empty annotation.
    task::add_with_notes(TAG, &text::add_args("raised with no notes"), &[])
        .expect("add without notes");
    assert!(
        !task::pending_for_tag(TAG)
            .expect("list")
            .into_iter()
            .find(|t| t.description == "raised with no notes")
            .expect("find it")
            .has_notes()
    );

    // ---- empty input is a no-op, not an error ---------------------------
    task::add(TAG, &text::add_args("   ")).expect("empty add is a no-op");
    task::modify_description(&uuid, "").expect("empty edit is a no-op");
    task::annotate(&uuid, "").expect("empty note is a no-op");
    let unchanged = task::get(&uuid).expect("get").expect("exists");
    assert_eq!(unchanged.description, edited.description);
    assert_eq!(unchanged.annotations.len(), 1);

    // ---- set_active keeps exactly one active per tag ---------------------
    let all = task::pending_for_tag(TAG).expect("list");
    assert!(all.len() >= 2, "need two tasks to test exclusivity");
    let (first, second) = (all[0].uuid.clone(), all[1].uuid.clone());

    task::set_active(TAG, &first).expect("set active");
    assert_eq!(
        task::active_for_tag(TAG).expect("active").map(|t| t.uuid),
        Some(first.clone())
    );

    task::set_active(TAG, &second).expect("switch active");
    let active_now: Vec<String> = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .filter(|t| t.is_active())
        .map(|t| t.uuid)
        .collect();
    assert_eq!(
        active_now,
        vec![second.clone()],
        "setting a second task active must stop the first — this is what makes \
         `niritasks task active` unambiguous"
    );

    // ---- complete and delete --------------------------------------------
    let before = task::pending_for_tag(TAG).expect("list").len();
    task::complete(&first).expect("complete");
    assert_eq!(task::pending_for_tag(TAG).expect("list").len(), before - 1);

    task::delete(&second).expect("delete");
    assert_eq!(task::pending_for_tag(TAG).expect("list").len(), before - 2);

    // ---- tags scope the list --------------------------------------------
    task::add("othertag", &text::add_args("not mine")).expect("add to other tag");
    assert!(
        !task::pending_for_tag(TAG)
            .expect("list")
            .iter()
            .any(|t| t.description == "not mine"),
        "a task on another tag must not appear in this tag's list"
    );
}
