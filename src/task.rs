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

/// The tag that marks a task as the one to do next. Taskwarrior's own: its
/// `urgency.next.coefficient` adds 15 urgency to a `+next` task, so
/// `task next`, which sorts by urgency, lifts it with no code here.
pub const UP_NEXT_TAG: &str = "next";

/// What a note linking the task's refine report starts with, as the refine
/// mod writes it: `Report: <path>`. `Report (earlier draft): <path>` marks a
/// report of a plan since changed, and is not the task's report.
pub const REPORT_NOTE: &str = "Report: ";

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    /// Taskwarrior's working-set number, what `task 48 …` takes: 0 once gc
    /// has run on a completed or deleted task; until then the export keeps
    /// its old id, and the panel's card zeroes it. Absent only on a
    /// hand-built task in a test, hence the default.
    #[serde(default)]
    pub id: u64,
    pub uuid: String,
    pub description: String,
    #[serde(default)]
    pub urgency: f64,
    /// Present only when the task has been started. Taskwarrior's `+ACTIVE`
    /// virtual tag is derived from this.
    #[serde(default)]
    pub start: Option<String>,
    /// When the task is parked until, as taskwarrior stamps it
    /// (`20261101T090000Z`, or `99991229T130000Z` for `wait:someday`).
    /// Taskwarrior's `+WAITING` is this date still being ahead of now; a date
    /// that has passed stays on the task and means nothing. `Option` already
    /// reads as `None` when the key is absent; the attribute keeps the
    /// struct's house style.
    #[serde(default)]
    pub wait: Option<String>,
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
    /// When the task was finished, as taskwarrior stamps it, on a completed
    /// or deleted task alone. The Finished tab lists the newest first and
    /// ages its cards from it. Empty on a task still to do.
    #[serde(default)]
    pub end: String,
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

    /// Marked as the one to do next, with Up next.
    pub fn is_up_next(&self) -> bool {
        self.tags.iter().any(|t| t == UP_NEXT_TAG)
    }

    /// The refine report linked from the task's notes: the path of the newest
    /// `Report: <path>` note, the last one, the mod appending them in the
    /// order the reports were made. None on a task with no such note; a
    /// `Report (earlier draft):` note is a report of a plan since changed,
    /// and never counts. A `Report:` note with nothing after it is skipped.
    pub fn report_path(&self) -> Option<&str> {
        self.annotations
            .iter()
            .rev()
            .find_map(|a| {
                a.description
                    .strip_prefix(REPORT_NOTE)
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
            })
    }

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
    // `rc.gc=off`, on reads alone. With gc on, an export moves the tasks
    // finished since the last gc out of pending.data, rewriting both
    // pending.data and completed.data. A `task done` or `task delete` leaves
    // its task in pending.data, so the daemon's next read wrote both files,
    // changed the mtimes it watches, and made it refresh a second time for
    // nothing (measured on 2.6.2). The same tasks and statuses come out
    // either way; only the `id`s differ. The panel shows the ids read with
    // gc off, which match what a non-gc command like `task 48 modify`
    // resolves; they renumber when a report's gc runs. Writes keep gc
    // through `base()`. No time is saved: an export takes about 6.5 ms with
    // or without it.
    let out = base()
        .arg("rc.gc=off")
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

/// Pending tasks carrying `tag`, most urgent first: taskwarrior's own
/// `status:pending` filter.
///
/// Nothing in `src/` calls it since [`listing`] splits one export in Rust; it
/// is kept so the write-path suite can check that split against taskwarrior's
/// own answer. The same order `task next` uses.
pub fn pending_for_tag(tag: &str) -> Result<Vec<Task>> {
    let mut tasks = export(&[&format!("+{tag}"), "status:pending"])?;
    tasks.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));
    Ok(tasks)
}

/// A workspace tag's tasks, split the way the task panel's tabs want them,
/// from one `task export`. Deleted tasks are not here: thrown away, not
/// finished.
#[derive(Debug, Default)]
pub struct Listing {
    /// Pending and not parked, most urgent first (the `task next` order).
    /// Each keeps whichever of `pending` or `waiting` it was exported with,
    /// so a stored `waiting` whose date has passed sits here with that status.
    pub pending: Vec<Task>,
    /// Parked: pending with a wait date still to come, taskwarrior's
    /// `+WAITING`. Each keeps the status it was exported with, `pending` from
    /// taskwarrior 2.6.
    pub waiting: Vec<Task>,
    /// The last [`FINISHED_CAP`] finished, the most recently finished first.
    pub completed: Vec<Task>,
}

impl Listing {
    /// Nothing on any tab: no pending, waiting or finished task.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.waiting.is_empty() && self.completed.is_empty()
    }
}

/// Every task on `tag`, split for the task panel's tabs. One export, whatever
/// the status, so the three parts are one snapshot: a write landing between
/// separate exports could show a task in two tabs or none.
pub fn listing(tag: &str) -> Result<Listing> {
    Ok(partition(export(&[&format!("+{tag}")])?, now_secs()))
}

/// The split behind [`listing`], apart from the export so it is tested
/// without a task database.
///
/// Taskwarrior 2.6's own rule: a pending task is waiting while its `wait` is
/// after `now`; the stored status stays `pending` either way. A `wait` that
/// does not parse counts as none, so a task is shown rather than hidden on a
/// stamp this tool cannot read.
fn partition(tasks: Vec<Task>, now: i64) -> Listing {
    let mut l = Listing::default();
    for t in tasks {
        match t.status.as_str() {
            // Taskwarrior before 2.6 stored `waiting` as a status of its own,
            // and old data can still carry it. Its gc rewrites that to
            // `pending` today, but this split must not depend on gc having
            // run, so both go through the same wait rule.
            "pending" | "waiting" => {
                // Strictly after: the second the wait date arrives, the task
                // is back on the list.
                let parked = t.wait.as_deref().and_then(stamp_secs).is_some_and(|w| w > now);
                if parked {
                    l.waiting.push(t);
                } else {
                    l.pending.push(t);
                }
            }
            "completed" => l.completed.push(t),
            // Deleted, a recurring parent, or anything a later taskwarrior adds:
            // on none of the panel's tabs.
            _ => {}
        }
    }
    l.pending.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));
    l.completed = latest_finished(l.completed);
    l
}

/// The tasks for `tag` parked as waiting, those with a wait date still to
/// come, which `pending_for_tag` leaves out: taskwarrior's own
/// `status:waiting` filter.
///
/// Nothing in `src/` calls it since [`listing`] splits one export in Rust; it
/// is kept so the write-path suite can check that split against taskwarrior's
/// own answer. Taskwarrior 2.6 exports these tasks as `pending`, and they are
/// returned as exported: the panel tells them apart by the listing's part
/// now, so no status needs rewriting.
pub fn waiting_for_tag(tag: &str) -> Result<Vec<Task>> {
    export(&[&format!("+{tag}"), "status:waiting"])
}

/// Whether the task is parked as waiting: taskwarrior's own `+WAITING`, a
/// wait date still to come. `get` cannot say, since 2.6 exports a waiting
/// task as `pending`, and a wait date that has passed stays on the task.
pub fn is_waiting(uuid: &str) -> Result<bool> {
    Ok(!export(&[uuid, "+WAITING"])?.is_empty())
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

/// How many finished tasks the task panel's Finished tab lists: enough to
/// find one closed too soon, and few enough to keep the list short. Past the
/// panel's own cap of 8 cards the rest fold into "+N more", as on any tab.
pub const FINISHED_CAP: usize = 12;

/// The last [`FINISHED_CAP`] tasks finished on `tag`, the most recently
/// finished first: taskwarrior's own `status:completed` filter, cut as the
/// Finished tab cuts it. Completed only: a deleted task was thrown away, not
/// finished, and its status says `deleted` even though it keeps its `end`.
///
/// Nothing in `src/` calls it since [`listing`] splits one export in Rust; it
/// is kept so the write-path suite can check that split against taskwarrior's
/// own answer.
pub fn completed_for_tag(tag: &str) -> Result<Vec<Task>> {
    Ok(latest_finished(export(&[&format!("+{tag}"), "status:completed"])?))
}

/// Newest `end` first, cut to [`FINISHED_CAP`]. Apart from
/// [`completed_for_tag`] so the order and the cut are tested without a
/// task database. The stamp sorts as text the way it does as a time.
fn latest_finished(mut tasks: Vec<Task>) -> Vec<Task> {
    tasks.sort_by(|a, b| b.end.cmp(&a.end));
    tasks.truncate(FINISHED_CAP);
    tasks
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

/// Add a task and attach one annotation per note, returning the new task's
/// uuid — `None` when the description was empty and nothing was added.
///
/// The notes are the add box's rows, in order, empty ones already dropped.
/// They are attached one at a time rather than joined, because separate
/// annotations are what the box's rows are counting.
///
/// A note that cannot be attached fails the whole call: the task is already
/// added by then, so the caller is told rather than left believing the notes
/// went with it.
///
/// The uuid is what Add & refine hands to `niritasks task refine`.
pub fn add_with_notes(tag: &str, description_args: &[&str], notes: &[String]) -> Result<Option<String>> {
    let Some(uuid) = add(tag, description_args)? else {
        return Ok(None);
    };
    for note in notes {
        annotate(&uuid, note)?;
    }
    Ok(Some(uuid))
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

/// `task stop`, with its output dropped. On a task that was never started it
/// exits non-zero and prints "not started", and every caller counts that as
/// already stopped, so neither the status nor the words mean anything to them.
fn stop_quietly(uuid: &str) {
    let _ = base()
        .arg(uuid)
        .arg("stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Put a task on the list, not being worked on: stop it, and clear any wait
/// date so a parked task comes back. Waiting hides a task from the panel, so
/// this is the one way back for it.
///
/// `stop` exits non-zero when the task was not started, and stopped is then
/// already true, so its status is deliberately ignored. Clearing a wait that
/// was never set succeeds.
pub fn stop(uuid: &str) -> Result<()> {
    stop_quietly(uuid);
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("wait:")
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify wait:` failed");
    Ok(())
}

/// Put a finished task back on the list, pending again: Back to list on
/// the Finished tab. Taskwarrior drops its `end` with the status. `stop`
/// cannot do it, since neither stopping nor clearing a wait touches a
/// completed task's status. The wait is cleared in the same command: a task
/// finished while parked keeps its wait date, and with it its `end`, and
/// would come back pending but still hidden from the list.
fn reopen(uuid: &str) -> Result<()> {
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("status:pending")
        .arg("wait:")
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify status:pending wait:` failed");
    Ok(())
}

/// Park a task as waiting.
///
/// Taskwarrior's waiting status is really a wait date; `someday` is far enough
/// out to mean "parked until picked up by hand". A started task is stopped on
/// the way, for the same reason `move_to_tag` stops one: a task coming back
/// from waiting should not quietly still claim to be the work in progress.
pub fn wait(uuid: &str) -> Result<()> {
    stop_quietly(uuid);

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
    stop_quietly(uuid);

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

/// Put the up next tag on a task, or take it off. Only the tag changes:
/// `+next` or `-next` goes as one argument of its own, so taskwarrior reads
/// it as a tag and not as words for the description, and the task's status,
/// start, wait, other tags and notes stay as they were. A started task stays
/// started.
pub fn set_up_next(uuid: &str, on: bool) -> Result<()> {
    let sign = if on { '+' } else { '-' };
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg(format!("{sign}{UP_NEXT_TAG}"))
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify {sign}{UP_NEXT_TAG}` failed");
    Ok(())
}

/// Where a task can be moved to: `niritasks task status`'s argument, which a
/// card's Back to list, Stop, Waiting, Complete and Remove run.
///
/// Active and Stopped drive taskwarrior's start/stop flag; Waiting, Completed
/// and Deleted are its real statuses. From a card they are all just "where is
/// this task now".
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Status {
    Active,
    Stopped,
    Waiting,
    Completed,
    Deleted,
}

impl Status {
    /// Every state, in the order `task status --help` lists them.
    pub const ALL: [Status; 5] = [
        Status::Active,
        Status::Stopped,
        Status::Waiting,
        Status::Completed,
        Status::Deleted,
    ];

    /// The word its notification starts with.
    pub fn label(self) -> &'static str {
        match self {
            Status::Active => "Active",
            Status::Stopped => "Stopped",
            Status::Waiting => "Waiting",
            Status::Completed => "Completed",
            Status::Deleted => "Deleted",
        }
    }
}

/// Move a task to `status`.
///
/// Stopped on a completed task reopens it, which is how Back to list on
/// the Finished tab puts a task back.
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
        // Back to list on a finished card runs Stopped, as it does on a
        // waiting one: one command for both ways back.
        Status::Stopped if current.status == "completed" => reopen(uuid),
        Status::Stopped => stop(uuid),
        Status::Waiting => wait(uuid),
        Status::Completed => complete(uuid),
        Status::Deleted => delete(uuid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `end` is when a task was finished: present on a completed task,
    /// absent on one still to do.
    #[test]
    fn parses_end() {
        let json = r#"[{"uuid":"a","description":"d","status":"completed","end":"20261007T040233Z"},
                       {"uuid":"b","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].end, "20261007T040233Z");
        assert_eq!(tasks[1].end, "");
    }

    /// `id` is read from the export, and absent means 0, as taskwarrior
    /// writes it on a completed or deleted task.
    #[test]
    fn parses_id() {
        let json = r#"[{"id":48,"uuid":"a","description":"d"},
                       {"id":0,"uuid":"b","description":"d","status":"completed"},
                       {"uuid":"c","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].id, 48);
        assert_eq!(tasks[1].id, 0);
        assert_eq!(tasks[2].id, 0);
    }

    fn finished(uuid: &str, day: u32) -> Task {
        serde_json::from_value(serde_json::json!({
            "uuid": uuid,
            "description": uuid,
            "status": "completed",
            "end": format!("202610{day:02}T120000Z"),
        }))
        .unwrap()
    }

    /// The Finished tab's list: the most recently finished first, and never
    /// more than twelve, however many there are.
    #[test]
    fn the_latest_finished_come_first_and_stop_at_twelve() {
        let got = latest_finished((1..=15).map(|d| finished(&format!("t{d}"), d)).collect());
        assert_eq!(got.len(), FINISHED_CAP);
        assert_eq!(FINISHED_CAP, 12);
        assert_eq!(got[0].uuid, "t15");
        assert_eq!(got[11].uuid, "t4");
    }

    #[test]
    fn fewer_than_twelve_finished_are_all_kept_newest_first() {
        let got = latest_finished(vec![finished("old", 1), finished("new", 9), finished("mid", 5)]);
        let uuids: Vec<&str> = got.iter().map(|t| t.uuid.as_str()).collect();
        assert_eq!(uuids, vec!["new", "mid", "old"]);
    }

    /// The CLI takes each state's label lower-cased, so `task status <uuid>
    /// completed` notifies "Completed".
    #[test]
    fn a_status_is_its_label_lower_cased_on_the_command_line() {
        use clap::ValueEnum;
        for s in Status::ALL {
            let cli = s.to_possible_value().unwrap();
            assert_eq!(cli.get_name(), s.label().to_lowercase());
        }
        assert_eq!(Status::value_variants().len(), Status::ALL.len());
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

    #[test]
    fn the_next_tag_marks_a_task_up_next() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["proj","next"]}"#).unwrap();
        assert!(t.is_up_next());
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(!bare.is_up_next());
    }

    /// Tags are case-sensitive, and `+NEXT` is not Taskwarrior's `next`.
    #[test]
    fn up_next_is_matched_exactly() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["NEXT","next_x"]}"#).unwrap();
        assert!(!t.is_up_next());
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

    /// The report the mod linked from the task: the last `Report: <path>`
    /// note, which is the newest, the mod appending them in the order the
    /// reports were made.
    #[test]
    fn the_newest_report_note_is_the_tasks_report() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Goal: x"},
                {"entry":"20261006T020000Z","description":"Report: /r/first.html"},
                {"entry":"20261006T030000Z","description":"Report: /r/second.html"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/second.html"));
    }

    /// A task with no report note, or no notes at all, has no report.
    #[test]
    fn a_task_without_a_report_note_has_no_report() {
        let noted: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[{"entry":"20261006T010000Z","description":"Goal: x"}]}"#,
        )
        .unwrap();
        assert_eq!(noted.report_path(), None);
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(bare.report_path(), None);
    }

    /// A report of an earlier draft of the plan is not the plan's report,
    /// and a note that merely mentions a report is not a link to one.
    #[test]
    fn an_earlier_drafts_report_and_a_mention_are_not_the_report() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report (earlier draft): /r/old.html"},
                {"entry":"20261006T020000Z","description":"See the Report: /r/not-this.html"},
                {"entry":"20261006T030000Z","description":"Report:/r/no-space.html"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), None);
    }

    /// The path is the rest of the line, trimmed, so a note written with a
    /// trailing space still opens.
    #[test]
    fn the_report_path_is_trimmed() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[{"entry":"20261006T010000Z","description":"Report: /r/a b.html "}]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/a b.html"));
    }

    /// A `Report:` note with nothing after it links nothing, and does not
    /// hide the report an older note links.
    #[test]
    fn a_blank_report_note_is_skipped() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report: /r/a.html"},
                {"entry":"20261006T020000Z","description":"Report:  "}
            ]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/a.html"));
        let only_blank: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[{"entry":"20261006T010000Z","description":"Report: "}]}"#,
        )
        .unwrap();
        assert_eq!(only_blank.report_path(), None);
    }

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
        for bad in ["", "2026-10-06", "20261006T120929", "20261306T120929Z", "202é006T120929Z", "20261006X120929Z"] {
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

    /// The fixed "now" the split is tested against: noon on 2026-10-09.
    fn noon() -> i64 {
        stamp_secs("20261009T120000Z").unwrap()
    }

    /// A hand-built exported task: what `partition` sees from `task export`.
    fn t(uuid: &str, status: &str, wait: Option<&str>, urgency: f64, end: &str) -> Task {
        serde_json::from_value(serde_json::json!({
            "uuid": uuid,
            "description": uuid,
            "status": status,
            "wait": wait,
            "urgency": urgency,
            "end": end,
        }))
        .unwrap()
    }

    fn ids(tasks: &[Task]) -> Vec<&str> {
        tasks.iter().map(|t| t.uuid.as_str()).collect()
    }

    /// `wait` is read from the export, and absent means none.
    #[test]
    fn parses_wait() {
        let json = r#"[{"uuid":"a","description":"d","wait":"99991229T130000Z"},
                       {"uuid":"b","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].wait.as_deref(), Some("99991229T130000Z"));
        assert_eq!(tasks[1].wait, None);
    }

    #[test]
    fn a_pending_task_without_a_wait_is_pending() {
        let l = partition(vec![t("a", "pending", None, 1.0, "")], noon());
        assert_eq!(ids(&l.pending), vec!["a"]);
        assert!(l.waiting.is_empty() && l.completed.is_empty());
    }

    /// `wait:someday` and any other date still to come park the task.
    #[test]
    fn a_wait_still_to_come_is_waiting() {
        let l = partition(
            vec![
                t("someday", "pending", Some("99991229T130000Z"), 1.0, ""),
                t("soon", "pending", Some("20261009T120001Z"), 1.0, ""),
            ],
            noon(),
        );
        assert_eq!(ids(&l.waiting), vec!["someday", "soon"]);
        assert!(l.pending.is_empty());
    }

    /// A passed wait date stays on the task and means nothing.
    #[test]
    fn a_passed_wait_is_pending() {
        let l = partition(vec![t("a", "pending", Some("20261009T110000Z"), 1.0, "")], noon());
        assert_eq!(ids(&l.pending), vec!["a"]);
        assert!(l.waiting.is_empty());
    }

    /// Strictly after now is waiting, so a wait of exactly now is back on the
    /// list, as it is in taskwarrior the moment that second arrives.
    #[test]
    fn a_wait_of_exactly_now_is_pending() {
        let l = partition(vec![t("a", "pending", Some("20261009T120000Z"), 1.0, "")], noon());
        assert_eq!(ids(&l.pending), vec!["a"]);
        assert!(l.waiting.is_empty());
    }

    /// A stamp this tool cannot read shows the task rather than hiding it.
    #[test]
    fn an_unreadable_wait_is_pending() {
        let l = partition(vec![t("a", "pending", Some("someday"), 1.0, "")], noon());
        assert_eq!(ids(&l.pending), vec!["a"]);
        assert!(l.waiting.is_empty());
    }

    #[test]
    fn completed_come_newest_first_and_stop_at_the_cap() {
        let tasks = (1..=15).map(|d| t(&format!("c{d}"), "completed", None, 0.0, &format!("202610{d:02}T120000Z")));
        let l = partition(tasks.collect(), noon());
        assert_eq!(l.completed.len(), FINISHED_CAP);
        assert_eq!(l.completed[0].uuid, "c15");
        assert_eq!(l.completed[FINISHED_CAP - 1].uuid, "c4");
    }

    /// Deleted was thrown away, not finished: no tab lists it.
    #[test]
    fn deleted_tasks_are_dropped() {
        let l = partition(vec![t("d", "deleted", None, 0.0, "20261009T100000Z")], noon());
        assert!(l.is_empty());
    }

    /// A recurring parent, or a status a later taskwarrior adds, is on no tab.
    #[test]
    fn an_unknown_status_is_dropped() {
        let l = partition(
            vec![t("r", "recurring", None, 0.0, ""), t("x", "", None, 0.0, "")],
            noon(),
        );
        assert!(l.is_empty());
    }

    #[test]
    fn no_tasks_is_an_empty_listing() {
        let l = partition(Vec::new(), noon());
        assert!(l.is_empty());
    }

    /// The split reports what taskwarrior exported: the part, not the
    /// status, says a task is waiting.
    #[test]
    fn waiting_tasks_keep_their_exported_status() {
        let l = partition(vec![t("w", "pending", Some("99991229T130000Z"), 1.0, "")], noon());
        assert_eq!(l.waiting[0].status, "pending");
    }

    /// The `task next` order, as `pending_for_tag` sorts it.
    #[test]
    fn pending_come_most_urgent_first() {
        let l = partition(
            vec![
                t("low", "pending", None, 1.0, ""),
                t("high", "pending", None, 9.5, ""),
                t("mid", "pending", Some("20261001T000000Z"), 4.0, ""),
            ],
            noon(),
        );
        assert_eq!(ids(&l.pending), vec!["high", "mid", "low"]);
    }

    /// A stored `waiting`, from data written before taskwarrior 2.6, goes
    /// through the wait rule like `pending`: parked while the date is ahead,
    /// back on the list once it has passed.
    #[test]
    fn a_stored_waiting_status_follows_the_wait_rule() {
        let l = partition(
            vec![
                t("ahead", "waiting", Some("99991229T130000Z"), 1.0, ""),
                t("passed", "waiting", Some("20261001T000000Z"), 1.0, ""),
            ],
            noon(),
        );
        assert_eq!(ids(&l.waiting), vec!["ahead"]);
        assert_eq!(ids(&l.pending), vec!["passed"]);
    }

    /// Finished is finished: a wait date left on a completed task parks
    /// nothing.
    #[test]
    fn a_completed_task_with_a_wait_ahead_stays_completed() {
        let l = partition(vec![t("c", "completed", Some("99991229T130000Z"), 0.0, "20261008T120000Z")], noon());
        assert_eq!(ids(&l.completed), vec!["c"]);
        assert!(l.pending.is_empty() && l.waiting.is_empty());
    }

    #[test]
    fn a_listing_with_any_part_is_not_empty() {
        assert!(Listing::default().is_empty());
        for task in [
            t("p", "pending", None, 1.0, ""),
            t("w", "pending", Some("99991229T130000Z"), 1.0, ""),
            t("c", "completed", None, 0.0, "20261008T120000Z"),
        ] {
            assert!(!partition(vec![task], noon()).is_empty());
        }
    }
}
