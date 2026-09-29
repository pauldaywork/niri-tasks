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

use niri_tasks::{task, task::NoteEdit, text};
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
    let sandbox = Sandbox::new();
    const TAG: &str = "sandbox";

    // ---- data location --------------------------------------------------
    // The refine sandbox's one writable path: it must be where taskwarrior
    // really writes, which here is the sandbox, not ~/.task.
    assert_eq!(task::data_location().expect("data location"), sandbox.dir.join("data"));

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
    // The add box's note rows: one annotation each, attached to the
    // task that was just created. The uuid comes back from `task add` itself,
    // so the notes cannot land on somebody else's task.
    let notes = vec!["first note".to_string(), "second note with due:2026-09-01".to_string()];
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
        "each note is its own annotation"
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

    // ---- replace description and notes in one import ---------------------
    // The task box's save for an existing task. Taskwarrior cannot edit or
    // delete one annotation in place, so the whole list goes back through
    // `task import` — which makes it the write most able to lose something:
    // an unchanged note's date, a tag, or the literal-description rule.
    task::add(TAG, &text::add_args("to be rewritten")).expect("add");
    let target = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "to be rewritten")
        .expect("find the task to rewrite")
        .uuid;
    for note in ["one", "two", "three"] {
        task::annotate(&target, note).expect("annotate");
    }
    let stamps: Vec<String> = task::get(&target)
        .expect("get")
        .expect("exists")
        .annotations
        .iter()
        .map(|a| a.entry.clone())
        .collect();
    assert_eq!(stamps.len(), 3, "three notes, each with its own stamp");

    let wrote = task::replace_text(
        &target,
        "rewritten due:2026-09-01",
        &[
            NoteEdit { entry: Some(stamps[0].clone()), text: "one, edited".into() },
            NoteEdit { entry: Some(stamps[2].clone()), text: "three".into() },
            NoteEdit { entry: None, text: "four".into() },
            NoteEdit { entry: None, text: "five".into() },
        ],
    )
    .expect("replace");
    assert!(wrote, "a changed task is written");

    let rewritten = task::get(&target).expect("get").expect("exists");
    assert_eq!(rewritten.description, "rewritten due:2026-09-01");
    assert!(
        raw(&target, "due").is_empty(),
        "the description goes through import literally, like an edit"
    );
    assert!(rewritten.tags.iter().any(|t| t == TAG), "import must not drop the tag");
    assert_eq!(rewritten.status, "pending");

    let notes: Vec<(String, String)> = rewritten
        .annotations
        .iter()
        .map(|a| (a.entry.clone(), a.description.clone()))
        .collect();
    assert_eq!(notes.len(), 4, "note two is gone, four and five are new");

    // Helper to find an annotation by its description text and return its entry timestamp.
    let stamp_of = |text: &str| -> String {
        notes
            .iter()
            .find(|(_, desc)| desc == text)
            .map(|(entry, _)| entry.clone())
            .unwrap_or_else(|| panic!("note with text '{}' not found", text))
    };

    // Check the set of note texts: "two" is deleted, "one" is edited, "three" is kept, "four" and "five" are new.
    let texts: std::collections::HashSet<&str> = notes.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(texts, std::collections::HashSet::from_iter(vec!["one, edited", "three", "four", "five"]));

    // Kept notes preserve their original entry timestamps.
    assert_eq!(stamp_of("one, edited"), stamps[0], "an edited note keeps its date");
    assert_eq!(stamp_of("three"), stamps[2], "an untouched note keeps its date");

    // New notes are dated now, in the order typed. Only the kept stamps are
    // compared above and below: a new note dated within a second of a kept one
    // can be bumped past it, so its exact stamp is not worth pinning down.
    let stamp_four = stamp_of("four");
    let stamp_five = stamp_of("five");
    assert_ne!(stamp_four, stamps[0], "new note 'four' has a different date than old notes");
    assert_ne!(stamp_four, stamps[2], "new note 'four' has a different date than old notes");
    assert_ne!(stamp_five, stamps[0], "new note 'five' has a different date than old notes");
    assert_ne!(stamp_five, stamps[2], "new note 'five' has a different date than old notes");
    assert!(stamp_four < stamp_five, "new notes are dated in the order they were typed");

    // Saving what is already there writes nothing — not even an undo entry.
    let undo = sandbox.dir.join("data").join("undo.data");
    let undo_lines = || std::fs::read_to_string(&undo).unwrap_or_default().lines().count();
    let before_noop = undo_lines();
    let unchanged: Vec<NoteEdit> = rewritten
        .annotations
        .iter()
        .map(|a| NoteEdit { entry: Some(a.entry.clone()), text: a.description.clone() })
        .collect();
    assert!(
        !task::replace_text(&target, &rewritten.description, &unchanged).expect("no-op"),
        "an unchanged save reports that it wrote nothing"
    );
    assert_eq!(undo_lines(), before_noop, "and leaves nothing in the undo log");

    // An empty description is not a task: nothing is written.
    assert!(!task::replace_text(&target, "", &[]).expect("empty description"));
    assert_eq!(task::get(&target).expect("get").expect("exists").annotations.len(), 4);

    // Deleting every note leaves a task with none.
    assert!(task::replace_text(&target, "rewritten due:2026-09-01", &[]).expect("clear notes"));
    assert!(!task::get(&target).expect("get").expect("exists").has_notes());

    // A new note placed above kept ones must not take their dates. The notes
    // here are written just now, so their stamps are at or after "now" — the
    // case where taskwarrior, resolving a stamp collision in array order,
    // would hand the new note the kept note's stamp and bump the kept one.
    task::add(TAG, &text::add_args("fresh notes")).expect("add");
    let fresh = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "fresh notes")
        .expect("find the fresh task")
        .uuid;
    for note in ["kept a", "kept b"] {
        task::annotate(&fresh, note).expect("annotate");
    }
    let fresh_stamps: Vec<String> = task::get(&fresh)
        .expect("get")
        .expect("exists")
        .annotations
        .iter()
        .map(|a| a.entry.clone())
        .collect();
    assert!(task::replace_text(
        &fresh,
        "fresh notes",
        &[
            NoteEdit { entry: None, text: "brand new".into() },
            NoteEdit { entry: Some(fresh_stamps[0].clone()), text: "kept a".into() },
            NoteEdit { entry: Some(fresh_stamps[1].clone()), text: "kept b".into() },
        ],
    )
    .expect("replace with a new note first"));
    let after: Vec<(String, String)> = task::get(&fresh)
        .expect("get")
        .expect("exists")
        .annotations
        .iter()
        .map(|a| (a.description.clone(), a.entry.clone()))
        .collect();
    for (text, stamp) in [("kept a", &fresh_stamps[0]), ("kept b", &fresh_stamps[1])] {
        let now = after.iter().find(|(d, _)| d == text).map(|(_, e)| e);
        assert_eq!(now, Some(stamp), "'{text}' keeps its date under a new note placed above it");
    }

    // A uuid that only starts like the task's is not the task: a short prefix
    // on the command line could match a different one, and import would then
    // overwrite it.
    let err = task::replace_text(&fresh[..8], "prefix write", &[]).expect_err("prefix must not match");
    assert!(err.to_string().contains("uuid"), "unexpected error: {err}");
    assert_eq!(
        task::get(&fresh).expect("get").expect("exists").description,
        "fresh notes",
        "a refused write changes nothing"
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

    // ---- stop leaves the tag with no active task --------------------------
    task::stop(&second).expect("stop");
    assert!(
        task::active_for_tag(TAG).expect("active").is_none(),
        "stopping the active task must leave the tag with none"
    );
    task::stop(&second).expect("stopping an already-stopped task is a no-op");

    // ---- wait parks the task, and stops it on the way ---------------------
    task::add(TAG, &text::add_args("park me")).expect("add");
    let parked = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "park me")
        .expect("find the task to park")
        .uuid;
    task::set_active(TAG, &parked).expect("start it first");

    // Taskwarrior 2.6 dropped the stored `waiting` status: the task keeps
    // `status:pending` plus a `wait` date, and `status:pending` filters
    // exclude it. So the wait date and the pending list are what to assert.
    task::wait(&parked).expect("wait");
    assert!(!raw(&parked, "wait").is_empty(), "wait date should be set");
    assert!(
        raw(&parked, "start").is_empty(),
        "a task must not come back from waiting still claiming to be in progress"
    );
    assert!(
        !task::pending_for_tag(TAG)
            .expect("list")
            .iter()
            .any(|t| t.uuid == parked),
        "a waiting task must drop out of the pending list"
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
