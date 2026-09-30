//! Taskwarrior, via the `task` CLI.
//!
//! This is the only module that assumes anything about taskwarrior's interface.
//! The machine here runs 2.6.2, where `task export` emits JSON and there is no
//! library to link against. Keeping it behind one module means a later move to
//! 3.x (taskchampion) touches one file.
//!
//! Every invocation carries `rc.confirmation=no rc.verbose=nothing` so nothing
//! blocks on a prompt or prints chatter into output the caller is parsing.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

/// The tag the `refine-task` skill puts on a task once it has been worked up
/// into a plan. Not `ready`: Taskwarrior already has a virtual `+READY`
/// meaning something else, one Shift key away.
pub const PLANNED_TAG: &str = "planned";

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub uuid: String,
    pub description: String,
    #[serde(default)]
    pub urgency: f64,
    /// Present only when the task has been started. Taskwarrior's `+ACTIVE`
    /// virtual tag is derived from this.
    #[serde(default)]
    pub start: Option<String>,
    /// Absent rather than empty on a task with no notes, hence the default.
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Absent rather than empty on a task with no tags, hence the default.
    #[serde(default)]
    pub tags: Vec<String>,
    /// `task export`'s status string ("pending", "completed", "deleted", …).
    /// `task::get` is unfiltered by status, so a caller that only makes sense
    /// on a pending task — `refine`, say — has to check this itself.
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Annotation {
    pub entry: String,
    pub description: String,
}

impl Task {
    pub fn is_active(&self) -> bool {
        self.start.is_some()
    }

    pub fn has_notes(&self) -> bool {
        !self.annotations.is_empty()
    }

    /// Worked up into a plan by the `refine-task` skill.
    pub fn is_planned(&self) -> bool {
        self.tags.iter().any(|t| t == PLANNED_TAG)
    }
}

impl Annotation {
    /// One note as `niritasks task get-notes` prints it: the date it was added, then the text.
    pub fn line(&self) -> String {
        format!(
            "{}  {}",
            self.date(),
            crate::text::collapse_whitespace(&self.description)
        )
    }

    /// The `entry` stamp as a date a person reads — a task box row's date, and
    /// the front of [`Annotation::line`].
    ///
    /// Taskwarrior stores it as `20260816T130710Z`; the first eight characters
    /// are the day, and hyphens are what make them read as one. Anything that
    /// is not that shape is passed through untouched rather than sliced into
    /// nonsense — the stamp is taskwarrior's to define, not ours.
    pub fn date(&self) -> String {
        // Characters, not bytes: `&self.entry[..8]` panics when the eighth byte
        // lands inside a multi-byte character, and this runs inside a
        // long-lived daemon.
        let day: String = self.entry.chars().take(8).collect();
        if day.len() != 8 || !day.chars().all(|c| c.is_ascii_digit()) {
            return self.entry.clone();
        }
        format!("{}-{}-{}", &day[..4], &day[4..6], &day[6..8])
    }
}

fn base() -> Command {
    let mut cmd = Command::new("task");
    cmd.arg("rc.confirmation=no")
        .arg("rc.verbose=nothing")
        .arg("rc.json.array=on");
    cmd
}

/// Run a `task export` filter and parse the result.
///
/// `export` exits non-zero when the filter matches nothing, which is a normal
/// case rather than an error — an empty list is returned instead.
fn export(filter: &[&str]) -> Result<Vec<Task>> {
    export_values(filter)?
        .into_iter()
        .map(|v| serde_json::from_value(v).context("could not parse a task from `task export`"))
        .collect()
}

/// [`export`] without the parse: each task exactly as taskwarrior wrote it.
/// [`replace_text`] needs this form, because `task import` drops any field it
/// is not handed back, and [`Task`] only models the fields this tool reads.
fn export_values(filter: &[&str]) -> Result<Vec<Value>> {
    let out = base()
        .args(filter)
        .arg("export")
        .output()
        .context("could not run `task` — is taskwarrior installed?")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(trimmed).context("could not parse `task export` output as JSON")
}

/// Pending tasks carrying `tag`, most urgent first.
///
/// The same order `task next` uses, so the picker agrees with the terminal.
pub fn pending_for_tag(tag: &str) -> Result<Vec<Task>> {
    let mut tasks = export(&[&format!("+{tag}"), "status:pending"])?;
    tasks.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));
    Ok(tasks)
}

/// The active (started) pending tasks for `tag`. Several at once is normal:
/// each agent working a task in its own worktree has one.
pub fn active_for_tag(tag: &str) -> Result<Vec<Task>> {
    export(&[&format!("+{tag}"), "+ACTIVE", "status:pending"])
}

/// The uuids of `tag`'s pending tasks that are waiting on another pending task.
///
/// Asked of taskwarrior rather than worked out from each task's `depends`:
/// whether a dependency still blocks depends on *its* status, which may sit on
/// another tag entirely, and `+BLOCKED` already knows.
pub fn blocked_uuids_for_tag(tag: &str) -> Result<Vec<String>> {
    Ok(export(&[&format!("+{tag}"), "+BLOCKED", "status:pending"])?
        .into_iter()
        .map(|t| t.uuid)
        .collect())
}

pub fn get(uuid: &str) -> Result<Option<Task>> {
    Ok(export(&[uuid])?.into_iter().next())
}

/// Where taskwarrior keeps its database, as taskwarrior itself resolves it —
/// `TASKDATA`, `.taskrc`'s `data.location` and its `$HOME` expansion included.
/// The refine sandbox lets writes through to here and nowhere else.
pub fn data_location() -> Result<std::path::PathBuf> {
    let out = base()
        .args(["_get", "rc.data.location"])
        .output()
        .context("could not run `task` — is taskwarrior installed?")?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    anyhow::ensure!(!path.is_empty(), "taskwarrior did not say where its data is");
    Ok(std::path::PathBuf::from(path))
}

/// Add a task, returning the uuid of the task created.
///
/// `description_args` is deliberately pre-split by [`crate::text::add_args`] so
/// taskwarrior parses its own attribute syntax — "due:friday" sets a due date
/// rather than becoming part of the description.
///
/// `rc.verbose=new-uuid` is what makes the uuid available: the default report
/// says "Created task 12.", and an id is exactly the handle that renumbers out
/// from under you as other tasks complete. It overrides the `rc.verbose=nothing`
/// in [`base`] because taskwarrior takes the last setting of a name.
///
/// `None` means nothing was added — an empty description — rather than a
/// failure to find the uuid, which is an error.
pub fn add(tag: &str, description_args: &[&str]) -> Result<Option<String>> {
    if description_args.is_empty() {
        return Ok(None);
    }
    let out = base()
        .arg("rc.verbose=new-uuid")
        .arg("add")
        .args(description_args)
        .arg(format!("+{tag}"))
        .output()
        .context("could not run `task add`")?;
    anyhow::ensure!(out.status.success(), "`task add` failed");

    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(Some(parse_new_uuid(&stdout).context(
        "`task add` did not report the new task's uuid",
    )?))
}

/// Pull the uuid out of taskwarrior's "Created task <uuid>." line.
///
/// Split out from [`add`] so the parsing can be tested without writing to a
/// task database — it is the one part of the exchange that would fail silently
/// if a future taskwarrior worded the line differently.
fn parse_new_uuid(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Created task "))
        .map(|rest| rest.trim_end_matches('.').trim().to_string())
        .find(|uuid| {
            uuid.len() == 36 && uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        })
}

/// Add a task and attach one annotation per note.
///
/// The notes are the add box's rows, in order, empty ones already dropped.
/// They are attached one at a time rather than joined, because separate
/// annotations are what the picker's `¶` marker and the box's rows are
/// counting.
///
/// A note that cannot be attached fails the whole call: the task is already
/// added by then, so the caller is told rather than left believing the notes
/// went with it.
pub fn add_with_notes(tag: &str, description_args: &[&str], notes: &[String]) -> Result<()> {
    let Some(uuid) = add(tag, description_args)? else {
        return Ok(());
    };
    for note in notes {
        annotate(&uuid, note)?;
    }
    Ok(())
}

/// Replace a task's description.
///
/// The text is passed as a single argument after `--`, so a typed "due:" stays
/// literal text — the opposite of [`add`], deliberately.
pub fn modify_description(uuid: &str, description: &str) -> Result<()> {
    if description.is_empty() {
        return Ok(());
    }
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("--")
        .arg(description)
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify` failed");
    Ok(())
}

/// Attach a note. Same quoting rule as [`modify_description`].
pub fn annotate(uuid: &str, note: &str) -> Result<()> {
    if note.is_empty() {
        return Ok(());
    }
    let status = base()
        .arg(uuid)
        .arg("annotate")
        .arg("--")
        .arg(note)
        .status()
        .context("could not run `task annotate`")?;
    anyhow::ensure!(status.success(), "`task annotate` failed");
    Ok(())
}

/// One note as the task box hands it back.
///
/// `entry` is the stamp of the note it was loaded from, which is what lets an
/// edited note keep the date it was first written. A note typed in the box has
/// none, and taskwarrior dates it when it is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEdit {
    pub entry: Option<String>,
    pub text: String,
}

/// Replace a task's description and its whole list of notes, in one write.
///
/// Taskwarrior has no command to edit or delete one annotation by position,
/// but `task import` of a uuid that already exists replaces that task. So this
/// exports the task as it stands *now* — not as it was when the box opened,
/// because import drops any field it is not given, and a stale copy would undo
/// whatever changed in between — swaps in the two fields the box edits, and
/// imports it back.
///
/// The description goes through as JSON, so it stays literal the way
/// [`modify_description`] keeps it: a typed `due:` is text, not a due date.
///
/// Returns whether anything was written. Nothing is when the description is
/// empty — a task has to be called something — or when nothing changed, so
/// opening the box and saving leaves no trace in the undo log.
pub fn replace_text(uuid: &str, description: &str, notes: &[NoteEdit]) -> Result<bool> {
    if description.is_empty() {
        return Ok(false);
    }
    let current = export_values(&[uuid])?
        .into_iter()
        .next()
        .context("task not found")?;
    // A short uuid prefix can match a different task than the one meant, and
    // import would overwrite that one, so only an exact match is written.
    anyhow::ensure!(
        current.get("uuid").and_then(Value::as_str) == Some(uuid),
        "task uuid does not match `{uuid}`; refusing to overwrite another task"
    );
    let Some(updated) = with_text(current, description, notes) else {
        return Ok(false);
    };

    let mut child = base()
        .arg("import")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("could not run `task import`")?;
    // Taken, so it is dropped — and the pipe closed — before the wait: import
    // reads to end of input. taskwarrior's import expects an array of tasks.
    let json_array = serde_json::json!([updated]).to_string();
    child
        .stdin
        .take()
        .context("`task import` has no stdin")?
        .write_all(json_array.as_bytes())
        .context("could not write to `task import`")?;
    let out = child.wait_with_output().context("`task import` did not finish")?;
    anyhow::ensure!(
        out.status.success(),
        "`task import` failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(true)
}

/// `task` with its description and notes swapped for these, or `None` when
/// that would change nothing.
///
/// Split out from [`replace_text`] so the one part that decides what reaches
/// the database can be tested without one. A task with no notes exports no
/// `annotations` key rather than an empty list, so the key is removed, not
/// emptied, when every note is deleted, and a missing key compares as none.
fn with_text(mut task: Value, description: &str, notes: &[NoteEdit]) -> Option<Value> {
    // Kept notes go first, new ones after, each group in the order given.
    // Taskwarrior settles two notes with the same stamp in array order, so a
    // new note ahead of a kept one whose stamp is at or after now would be
    // dated with that stamp and push the kept note a second on — changing a
    // date that must not change. It sorts by stamp anyway, so what the box
    // shows is unaffected. An unchanged save has no new notes and its kept
    // ones arrive in stored order, so the comparison below still holds.
    let (kept, new): (Vec<&NoteEdit>, Vec<&NoteEdit>) = notes.iter().partition(|n| n.entry.is_some());
    let annotations: Vec<Value> = kept
        .into_iter()
        .chain(new)
        .map(|n| match &n.entry {
            Some(entry) => serde_json::json!({ "entry": entry, "description": n.text }),
            None => serde_json::json!({ "description": n.text }),
        })
        .collect();

    let same_description = task.get("description").and_then(Value::as_str) == Some(description);
    let current = task
        .get("annotations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if same_description && current == annotations {
        return None;
    }

    // `task export` always emits objects; anything else is not a task this
    // could write back, and "nothing to do" is the safe reading of it.
    let fields = task.as_object_mut()?;
    fields.insert("description".into(), description.into());
    if annotations.is_empty() {
        fields.remove("annotations");
    } else {
        fields.insert("annotations".into(), Value::Array(annotations));
    }
    Some(task)
}

pub fn delete(uuid: &str) -> Result<()> {
    let status = base().arg(uuid).arg("delete").status()?;
    anyhow::ensure!(status.success(), "`task delete` failed");
    Ok(())
}

pub fn complete(uuid: &str) -> Result<()> {
    let status = base().arg(uuid).arg("done").status()?;
    anyhow::ensure!(status.success(), "`task done` failed");
    Ok(())
}

/// Put a task on the list, not being worked on: stop it, and clear any wait
/// date so a parked task comes back. Waiting hides a task from the panel, so
/// this is the one way back for it.
///
/// `stop` exits non-zero when the task was not started, and stopped is then
/// already true, so its status is deliberately ignored. Clearing a wait that
/// was never set succeeds.
pub fn stop(uuid: &str) -> Result<()> {
    let _ = base().arg(uuid).arg("stop").status();
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("wait:")
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify wait:` failed");
    Ok(())
}

/// Park a task as waiting.
///
/// Taskwarrior's waiting status is really a wait date; `someday` is far enough
/// out to mean "parked until picked up by hand". A started task is stopped on
/// the way, for the same reason `move_to_tag` stops one: a task coming back
/// from waiting should not quietly still claim to be the work in progress.
pub fn wait(uuid: &str) -> Result<()> {
    let _ = base().arg(uuid).arg("stop").status();

    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("wait:someday")
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify wait:someday` failed");
    Ok(())
}

/// Move a task from one workspace's tag to another.
///
/// The two tags go as separate arguments, each one entirely `-tag` or `+tag`,
/// which is what makes taskwarrior read them as metadata rather than as words
/// to append to the description.
///
/// A moved task is stopped on the way out: one that carried its `start` across
/// would claim to be in progress on a workspace nobody is working in. `stop`
/// exits non-zero when the task was not started, which is the normal case, so
/// only the retag's status is checked.
pub fn move_to_tag(uuid: &str, from: &str, to: &str) -> Result<()> {
    let _ = base().arg(uuid).arg("stop").status();

    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg(format!("-{from}"))
        .arg(format!("+{to}"))
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify` failed");
    Ok(())
}

/// Mark a task as being worked on. Other active tasks stay active: several
/// tasks run at once, one per worktree, and starting one must not pull another
/// agent's claim out from under it.
///
/// Taskwarrior refuses to start a task twice, so an already active one is left
/// as it is.
pub fn set_active(uuid: &str) -> Result<()> {
    let current = get(uuid)?.with_context(|| format!("no task {uuid}"))?;
    if current.is_active() {
        return Ok(());
    }
    let status = base().arg(uuid).arg("start").status()?;
    anyhow::ensure!(status.success(), "`task start` failed");
    Ok(())
}

/// Where a task can be moved to: the menu's "Update status" list, and
/// `niritasks task status`'s argument. One list for both, so a script and the
/// menu can never offer different states.
///
/// Active and Stopped drive taskwarrior's start/stop flag; Waiting, Completed
/// and Deleted are its real statuses. From the menu they are all just "where
/// is this task now".
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Status {
    Active,
    Stopped,
    Waiting,
    Completed,
    Deleted,
}

impl Status {
    /// In the order the menu lists them.
    pub const ALL: [Status; 5] = [
        Status::Active,
        Status::Stopped,
        Status::Waiting,
        Status::Completed,
        Status::Deleted,
    ];

    /// The word the menu shows, and the one its notification starts with.
    pub fn label(self) -> &'static str {
        match self {
            Status::Active => "Active",
            Status::Stopped => "Stopped",
            Status::Waiting => "Waiting",
            Status::Completed => "Completed",
            Status::Deleted => "Deleted",
        }
    }

    pub fn from_label(label: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|s| s.label() == label)
    }
}

/// Move a task to `status`.
///
/// A task already completed or deleted is left as it is and this returns Ok:
/// taskwarrior refuses to complete a task twice, and a script retrying after a
/// half-finished run should not fail on the half that worked.
pub fn set_status(uuid: &str, status: Status) -> Result<()> {
    let current = get(uuid)?.with_context(|| format!("no task {uuid}"))?;
    match (status, current.status.as_str()) {
        (Status::Completed, "completed") | (Status::Deleted, "deleted") => return Ok(()),
        _ => {}
    }
    match status {
        Status::Active => set_active(uuid),
        Status::Stopped => stop(uuid),
        Status::Waiting => wait(uuid),
        Status::Completed => complete(uuid),
        Status::Deleted => delete(uuid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CLI takes the menu's words, lower-cased, and the menu's rows read
    /// back to the same state — so `task status <uuid> completed` and picking
    /// "Completed" are the same thing.
    #[test]
    fn a_status_is_the_same_word_in_the_menu_and_on_the_command_line() {
        use clap::ValueEnum;
        for s in Status::ALL {
            let cli = s.to_possible_value().unwrap();
            assert_eq!(cli.get_name(), s.label().to_lowercase());
            assert_eq!(Status::from_label(s.label()), Some(s));
        }
        assert_eq!(Status::value_variants().len(), Status::ALL.len());
        assert_eq!(Status::from_label("Done"), None);
    }

    #[test]
    fn parses_a_task_without_annotations() {
        // `.annotations` is absent rather than empty on a task with none.
        let json = r#"[{"uuid":"abc","description":"do a thing","urgency":3.5}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].uuid, "abc");
        assert!(!tasks[0].has_notes());
        assert!(!tasks[0].is_active());
    }

    /// The stamp taskwarrior stores is not a date anyone reads.
    #[test]
    fn note_dates_read_as_dates() {
        let a = Annotation {
            entry: "20260816T130710Z".into(),
            description: "a note".into(),
        };
        assert_eq!(a.line(), "2026-08-16  a note");
    }

    /// A stamp of some other shape is shown as it is. Slicing blindly would
    /// turn an unexpected format into invented punctuation, or panic on a
    /// multi-byte boundary.
    #[test]
    fn an_unexpected_stamp_is_passed_through() {
        for stamp in ["", "2026-08-16", "whenever", "日付です"] {
            let a = Annotation {
                entry: stamp.into(),
                description: "x".into(),
            };
            assert_eq!(a.line(), format!("{stamp}  x"));
        }
    }

    /// The uuid comes off `rc.verbose=new-uuid`'s line, not off the id in the
    /// default one — the whole reason for asking taskwarrior differently.
    #[test]
    fn reads_the_uuid_out_of_the_created_line() {
        assert_eq!(
            parse_new_uuid("Created task 58ebacaa-3e39-4098-8229-1691927fc8ba.\n"),
            Some("58ebacaa-3e39-4098-8229-1691927fc8ba".to_string())
        );
    }

    /// "Created task 12." is the *id* form, which is what this parse exists to
    /// avoid mistaking for a handle: better no uuid, and an error, than a
    /// number that points at a different task next week.
    #[test]
    fn refuses_the_id_form_and_junk() {
        assert_eq!(parse_new_uuid("Created task 12.\n"), None);
        assert_eq!(parse_new_uuid(""), None);
        assert_eq!(parse_new_uuid("something else entirely\n"), None);
    }

    #[test]
    fn detects_active_and_annotated_tasks() {
        let json = r#"[{
            "uuid":"abc","description":"x","urgency":1.0,
            "start":"20260815T080000Z",
            "annotations":[{"entry":"20260815T080000Z","description":"a note"}]
        }]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert!(tasks[0].is_active());
        assert!(tasks[0].has_notes());
        assert_eq!(tasks[0].annotations[0].description, "a note");
    }

    /// `task export` leaves `tags` out entirely on an untagged task, so the
    /// field must default rather than fail to parse.
    #[test]
    fn tags_absent_from_export_are_empty() {
        let t: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(t.tags.is_empty());
        assert!(!t.is_planned());
    }

    /// `refine` refuses a task that is not pending; this is the field it
    /// reads to know.
    #[test]
    fn status_is_read_from_export() {
        let json = r#"[{"uuid":"abc","description":"x","urgency":1.0,"status":"pending"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].status, "pending");
    }

    #[test]
    fn the_planned_tag_marks_a_task_planned() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["proj","planned"]}"#).unwrap();
        assert!(t.is_planned());
    }

    /// Tags are case-sensitive, and `+PLANNED` is not ours.
    #[test]
    fn planned_is_matched_exactly() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["PLANNED","planned_x"]}"#).unwrap();
        assert!(!t.is_planned());
    }

    fn exported() -> serde_json::Value {
        serde_json::json!({
            "uuid": "u", "description": "old", "status": "pending", "tags": ["x"],
            "annotations": [
                {"entry": "20260801T000000Z", "description": "one"},
                {"entry": "20260802T000000Z", "description": "two"}
            ]
        })
    }

    fn kept(entry: &str, text: &str) -> NoteEdit {
        NoteEdit { entry: Some(entry.into()), text: text.into() }
    }

    /// `task import` replaces the whole task, so everything but the two
    /// fields the box edits has to go back exactly as it came out.
    #[test]
    fn with_text_changes_only_the_description_and_notes() {
        let out = with_text(
            exported(),
            "new",
            &[kept("20260802T000000Z", "two, edited"), NoteEdit { entry: None, text: "three".into() }],
        )
        .expect("changed");
        assert_eq!(out["description"], "new");
        assert_eq!(out["tags"], serde_json::json!(["x"]));
        assert_eq!(out["status"], "pending");
        assert_eq!(
            out["annotations"],
            serde_json::json!([
                {"entry": "20260802T000000Z", "description": "two, edited"},
                {"description": "three"}
            ]),
            "a kept note keeps its stamp; a new one has none, so taskwarrior dates it now"
        );
    }

    /// Taskwarrior settles a stamp collision in array order, so a new note
    /// ahead of a kept one could be dated with the kept note's stamp.
    #[test]
    fn with_text_puts_kept_notes_before_new_ones() {
        let out = with_text(
            exported(),
            "old",
            &[
                NoteEdit { entry: None, text: "new".into() },
                kept("20260801T000000Z", "one"),
            ],
        )
        .expect("changed");
        assert_eq!(
            out["annotations"],
            serde_json::json!([
                {"entry": "20260801T000000Z", "description": "one"},
                {"description": "new"}
            ])
        );
    }

    #[test]
    fn with_text_is_none_when_nothing_changed() {
        let same = [kept("20260801T000000Z", "one"), kept("20260802T000000Z", "two")];
        assert!(with_text(exported(), "old", &same).is_none());
    }

    /// A task with no notes exports no `annotations` key at all, so "no notes
    /// before, none now" must compare equal rather than `[]` vs missing.
    #[test]
    fn with_text_treats_a_missing_notes_key_as_no_notes() {
        let bare = serde_json::json!({"uuid": "u", "description": "old"});
        assert!(with_text(bare, "old", &[]).is_none());
    }

    /// Deleting every note has to remove the key: import keeps whatever
    /// list it is given, and an empty one is not what taskwarrior exports.
    #[test]
    fn with_text_drops_the_notes_key_when_every_note_is_deleted() {
        let out = with_text(exported(), "old", &[]).expect("changed");
        assert!(out.get("annotations").is_none());
    }
}
