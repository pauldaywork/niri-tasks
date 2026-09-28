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
use std::process::Command;

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

    /// Every note, one per line — what the note box lists above its input and
    /// what `niritasks task get-notes` prints.
    ///
    /// One definition, because there are three callers and they must agree:
    /// notes are invisible from the picker, which shows a description, so this
    /// listing is the only place they are read.
    pub fn notes_list(&self) -> String {
        self.annotations
            .iter()
            .map(Annotation::line)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Annotation {
    /// One note as every surface shows it: the date it was added, then the text.
    pub fn line(&self) -> String {
        format!(
            "{}  {}",
            self.date(),
            crate::text::collapse_whitespace(&self.description)
        )
    }

    /// The `entry` stamp as a date a person reads.
    ///
    /// Taskwarrior stores it as `20260816T130710Z`; the first eight characters
    /// are the day, and hyphens are what make them read as one. Anything that
    /// is not that shape is passed through untouched rather than sliced into
    /// nonsense — the stamp is taskwarrior's to define, not ours.
    fn date(&self) -> String {
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

/// The active (started) pending task for `tag`, if any.
///
/// `set_active` keeps at most one per tag, but take the first regardless — a
/// task started by hand in a terminal should not produce two answers.
pub fn active_for_tag(tag: &str) -> Result<Option<Task>> {
    Ok(export(&[&format!("+{tag}"), "+ACTIVE", "status:pending"])?
        .into_iter()
        .next())
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

/// Add a task and attach one annotation per note line.
///
/// The notes come from the add box's second text area, already split one per
/// line by [`crate::text::note_lines`]. They are attached one at a time rather
/// than joined, because separate annotations are what the picker's `¶` marker
/// and the note box's list are counting — a single annotation holding three
/// lines would read as one note everywhere afterwards.
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

/// Stop a task without starting another. `stop` exits non-zero when the task
/// was not started, and stopped is then already true, so its status is
/// deliberately ignored.
pub fn stop(uuid: &str) -> Result<()> {
    let _ = base().arg(uuid).arg("stop").status();
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
/// A moved task is stopped on the way out. "One active task per tag" is what
/// lets the task panel and `niritasks task active` have a single answer, and a task that
/// carried its `start` across would either hand the destination a second active
/// task or quietly claim to be the work in progress on a workspace nobody is
/// looking at. `stop` exits non-zero when the task was not started, which is
/// the normal case, so only the retag's status is checked.
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

/// Make `uuid` the one active task for `tag`.
///
/// Clears the tag's current active task first, so a tag never has two — which
/// is what lets `niritasks task active` have a single unambiguous answer. `stop` exits
/// non-zero when nothing matches, which is the normal case, so its status is
/// deliberately ignored.
pub fn set_active(tag: &str, uuid: &str) -> Result<()> {
    let _ = base()
        .arg(format!("+{tag}"))
        .arg("+ACTIVE")
        .arg("stop")
        .status();

    let status = base().arg(uuid).arg("start").status()?;
    anyhow::ensure!(status.success(), "`task start` failed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_task_without_annotations() {
        // `.annotations` is absent rather than empty on a task with none.
        let json = r#"[{"uuid":"abc","description":"do a thing","urgency":3.5}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].uuid, "abc");
        assert!(!tasks[0].has_notes());
        assert!(!tasks[0].is_active());
    }

    /// One listing, used by the note box, the CLI and the daemon alike. It was
    /// three copies of this format string before, free to drift apart.
    #[test]
    fn notes_list_is_one_line_per_note() {
        let json = r#"[{
            "uuid":"abc","description":"x","urgency":1.0,
            "annotations":[
                {"entry":"20260815T080000Z","description":"first"},
                {"entry":"20260816T091500Z","description":"second  note   wrapped"}
            ]
        }]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        let listing = tasks[0].notes_list();
        let lines: Vec<&str> = listing.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("  first"));
        assert!(
            lines[1].ends_with("  second note wrapped"),
            "a note's own whitespace is collapsed for the listing"
        );
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

    #[test]
    fn a_task_with_no_notes_lists_nothing() {
        let json = r#"[{"uuid":"abc","description":"x","urgency":1.0}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].notes_list(), "");
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
}
