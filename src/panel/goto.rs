//! The Go to list: what can be done with a task and where it can be gone
//! to, which the task panel shows in place of the cards when the card's
//! Go to… is pressed — refining and starting it, its report, its plan and
//! spec, its two herdr sessions, its worktree, VS Code, and reading it
//! aloud — each row only when it applies, and what picking one runs. Plain
//! data, like the project list: the rows are read off the task's
//! `TaskState` and what the surface found on the machine ([`Found`]), and
//! `surface.rs` only draws what this says. A document's row shows how old it
//! is, from the note linking it, else its file ([`Since`]).

use super::style as c;
use crate::actions::{Action, TaskState};
use crate::task::{self, Link};

/// The button's words, and the start of the list's title.
pub const LABEL: &str = "Go to…";

/// The footer while the Go to list shows. No text field: there are fourteen
/// rows at most.
pub const GOTO_KEYS: &str = "Up, Down: pick · Enter: do it · Esc: back to the card";

/// What the rows need to know that the task's state does not say, found
/// by the surface as Go to… is pressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Found {
    /// The task has a worktree (`work::worktree_path`).
    pub worktree: bool,
    /// VS Code (`code`) is on `$PATH`.
    pub editor: bool,
    /// A speech is playing or being got ready, any task's: reading aloud
    /// again stops it.
    pub speaking: bool,
}

/// When each document the task's notes link was made, as a taskwarrior
/// stamp, for the age at its row's right end: the linking note's date,
/// else the file's last change. Worked out as Go to… is pressed and never
/// saved. None for a document not linked, or with neither date.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Since {
    pub report: Option<String>,
    pub branch_report: Option<String>,
    pub plan: Option<String>,
    pub spec: Option<String>,
}

impl Since {
    /// The stamps of the `report`, `branch_report`, `plan` and `spec` a card links, asking
    /// `modified` for the Unix seconds a linked path last changed whenever
    /// a note's date is not taskwarrior's shape.
    pub fn of(
        report: Option<&Link>,
        branch_report: Option<&Link>,
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
        Since { report: since(report), branch_report: since(branch_report), plan: since(plan), spec: since(spec) }
    }
}

/// One row on the Go to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Work it up into a plan with Claude; again, once it has one.
    Refine,
    /// Refine, interviewing first.
    Grill,
    /// Its worktree and a Claude to plan it; back to both once started.
    Start,
    /// Claude builds the report of its plan, a fresh one if it has one.
    Report,
    /// Claude explains what its worktree branch has changed, a fresh report
    /// each time.
    Explain,
    /// The report its notes link, `Report: <path>`, in the browser.
    OpenReport,
    /// The branch report its notes link, `Branch report: <path>`.
    BranchReport,
    /// The plan its notes link, `Plan: <path>`.
    Plan,
    /// The design spec its notes link, `Spec: <path>`.
    Spec,
    /// Its refine Claude, `task-<uuid8>`, in the workspace's herdr session.
    RefineSession,
    /// Its working Claude, `work-<uuid8>`, there.
    WorkSession,
    /// A terminal in its worktree.
    Worktree,
    /// VS Code on its worktree, else the project folder.
    Editor,
    /// Read it aloud; or stop the speech playing.
    ReadAloud,
}

use Item::*;

/// The kinds of step the list's rows are, each drawn in its own colour, so
/// similar rows look alike. Where a row was a button before, its group
/// keeps that button's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// A Claude works on the plan: Refine, Grill me, Generate report. The
    /// Refine, Grill me and Report buttons' mauve.
    Plan,
    /// Get started or Back to worktree: Start's green.
    Work,
    /// Go to a Claude already running: the old Go to session's blue, Go
    /// to…'s own.
    Session,
    /// Read what the task links: its report, plan and spec. Yellow.
    Document,
    /// Its worktree in a terminal or VS Code. Sapphire.
    Folder,
    /// Read aloud: the old Speak button's pink.
    Aloud,
}

impl Group {
    pub const ALL: [Group; 6] = [Group::Plan, Group::Work, Group::Session, Group::Document, Group::Folder, Group::Aloud];

    /// Its CSS class, on each of its rows.
    pub fn class(self) -> &'static str {
        match self {
            Group::Plan => "goto-plan",
            Group::Work => "goto-work",
            Group::Session => "goto-session",
            Group::Document => "goto-document",
            Group::Folder => "goto-folder",
            Group::Aloud => "goto-aloud",
        }
    }

    /// Its rows' colour.
    pub fn colour(self) -> &'static str {
        match self {
            Group::Plan => c::REFINE,
            Group::Work => c::ACTIVE,
            Group::Session => c::GO_TO,
            Group::Document => c::EDIT,
            Group::Folder => c::MOVE,
            Group::Aloud => c::READ_ALOUD,
        }
    }
}


impl Item {
    /// Every row, in the order the list shows the ones that apply.
    pub const ALL: [Item; 14] = [
        Refine, Grill, Start, Report, Explain, OpenReport, BranchReport, Plan, Spec, RefineSession, WorkSession, Worktree, Editor, ReadAloud,
    ];

    /// Whether the row shows for a task in `state`, with what was `found`.
    /// Refining, starting and the herdr sessions are for a task on the
    /// list, as their buttons were, Start giving way to Go to work session
    /// while the work Claude is live, and Refine and Grill me to Go to
    /// refine session while the refine Claude is, since each would only
    /// focus it; generating
    /// a report for a planned one, explaining the branch whenever there is
    /// a worktree, on or off the list;
    /// the documents whenever the notes link them, the worktree whenever
    /// there is one, VS Code whenever it is installed, and reading aloud
    /// always. Nothing on a processing task: a Claude is writing it.
    pub fn applies(self, state: TaskState, found: Found) -> bool {
        if state.processing {
            return false;
        }
        let on_list = !state.off_list();
        match self {
            Refine | Grill => on_list && !state.has_refine,
            Start => on_list && !state.has_work,
            Report => on_list && state.planned,
            Explain => found.worktree,
            OpenReport => state.has_report,
            BranchReport => state.has_branch_report,
            Plan => state.has_plan,
            Spec => state.has_spec,
            RefineSession => on_list && state.has_refine,
            WorkSession => on_list && state.has_work,
            Worktree => found.worktree,
            Editor => found.editor,
            ReadAloud => true,
        }
    }

    /// The rows a task in `state` gets, in [`Item::ALL`]'s order.
    pub fn of(state: TaskState, found: Found) -> Vec<Item> {
        Self::ALL.into_iter().filter(|i| i.applies(state, found)).collect()
    }

    /// The row's words. Four read as the step they take: Redo refine on a
    /// planned task, Back to worktree on a started one, Regenerate report
    /// once one exists, and Stop reading aloud while a speech plays.
    pub fn label(self, state: TaskState, found: Found) -> &'static str {
        match self {
            Refine if state.planned => "Redo refine",
            Refine => "Refine",
            Grill => "Grill me",
            Start if state.active => "Back to worktree",
            Start => "Get started",
            Report if state.has_report => "Regenerate report",
            Report => "Generate report",
            Explain => "Explain branch",
            OpenReport => "Open report",
            BranchReport => "Branch report",
            Plan => "Planning doc",
            Spec => "Design spec",
            RefineSession => "Go to refine session",
            WorkSession => "Go to work session",
            Worktree => "Open worktree folder",
            Editor => "Open in VS Code",
            ReadAloud if found.speaking => "Stop reading aloud",
            ReadAloud => "Read aloud",
        }
    }

    /// The kind of step the row is, which gives it its colour.
    pub fn group(self) -> Group {
        match self {
            Refine | Grill | Report | Explain => Group::Plan,
            Start => Group::Work,
            RefineSession | WorkSession => Group::Session,
            OpenReport | BranchReport | Plan | Spec => Group::Document,
            Worktree | Editor => Group::Folder,
            ReadAloud => Group::Aloud,
        }
    }

    /// The row's glyph, from the Nerd Font the cards use; no two alike.
    /// Font Awesome's: magic wand (Refine's own), comments (Grill me's),
    /// play (Start's), file-text outline, file-text, numbered list,
    /// lightbulb, comments outline, terminal, open folder outline and code,
    /// plus code-fork (Explain branch) and file (Branch report);
    /// Read aloud's speaker is Material Design's, Speak's old one.
    pub fn icon(self) -> &'static str {
        match self {
            Refine => "\u{f0d0}",
            Grill => "\u{f086}",
            Start => "\u{f04b}",
            Report => "\u{f0f6}",
            Explain => "\u{f126}",
            OpenReport => "\u{f15c}",
            BranchReport => "\u{f15b}",
            Plan => "\u{f0cb}",
            Spec => "\u{f0eb}",
            RefineSession => "\u{f0e6}",
            WorkSession => "\u{f120}",
            Worktree => "\u{f115}",
            Editor => "\u{f121}",
            ReadAloud => "\u{f0580}",
        }
    }

    /// The `niritasks` arguments picking the row runs. Generate report
    /// always builds (`--fresh`); Open report opens the linked one, which
    /// `task report` rebuilds if its file has gone.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Refine => return Action::Refine.args(uuid),
            Grill => &["task", "refine", uuid, "--grill"],
            Start => return Action::Start.args(uuid),
            Report => &["task", "report", uuid, "--fresh"],
            Explain => &["task", "report", uuid, "--branch", "--fresh"],
            OpenReport => &["task", "report", uuid],
            BranchReport => &["task", "report", uuid, "--branch"],
            Plan => &["task", "open", uuid, "plan"],
            Spec => &["task", "open", uuid, "spec"],
            RefineSession => &["task", "session", uuid, "--refine"],
            WorkSession => &["task", "session", uuid, "--work"],
            Worktree => &["task", "open", uuid, "worktree"],
            Editor => &["task", "open", uuid, "editor"],
            ReadAloud => &["task", "speak", uuid],
        };
        words.iter().map(|w| w.to_string()).collect()
    }

    /// Whether picking the row keeps the panel's keyboard: only Read
    /// aloud, which plays in the background, as Speak's button did. Every
    /// other row opens a herdr tab, a browser, a terminal or an editor,
    /// which takes it.
    pub fn keeps_keyboard(self) -> bool {
        self == ReadAloud
    }
}

/// The Go to list: the task it is for, its rows, and the one the highlight
/// is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoToList {
    uuid: String,
    /// The task's description, for the line over the rows.
    text: String,
    /// What the rows were picked by, for their words.
    state: TaskState,
    found: Found,
    /// When its documents were made, for their rows' ages.
    since: Since,
    rows: Vec<Item>,
    /// The row the highlight is on: Up, Down and Enter's.
    at: usize,
}

impl GoToList {
    /// The list for the task `uuid`, described `text`, in `state`, with
    /// what was `found` and when its documents were made, the first row
    /// highlighted. None with no row: a processing task.
    pub fn new(uuid: &str, text: &str, state: TaskState, found: Found, since: Since) -> Option<GoToList> {
        let rows = Item::of(state, found);
        (!rows.is_empty()).then(|| GoToList { uuid: uuid.into(), text: text.into(), state, found, since, rows, at: 0 })
    }

    /// The task the list is for.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    pub fn rows(&self) -> &[Item] {
        &self.rows
    }

    /// The index in `rows` the highlight is on.
    pub fn at(&self) -> usize {
        self.at
    }

    /// Up or Down, stopping at the ends.
    pub fn step(&mut self, forward: bool) {
        self.at = super::keys::step(self.at, self.rows.len(), forward);
    }

    /// The row at `at`, for Enter or a click. None past the last.
    pub fn item(&self, at: usize) -> Option<Item> {
        self.rows.get(at).copied()
    }

    /// The line over the rows: the button's words and the task's.
    pub fn title(&self) -> String {
        format!("{LABEL}: {}", self.text)
    }

    /// A row as its card reads: the icon, then the words. The two spaces
    /// are the gap a task card leaves after its icon.
    pub fn row_label(&self, item: Item) -> String {
        format!("{}  {}", item.icon(), item.label(self.state, self.found))
    }

    /// How long ago a document row's document was made, as a task card
    /// shows its age, `now` in Unix seconds: Open report, Branch report,
    /// Planning doc and Design spec alone. None on every other row, and on a document with
    /// no date.
    pub fn row_age(&self, item: Item, now: i64) -> Option<String> {
        let since = match item {
            OpenReport => &self.since.report,
            BranchReport => &self.since.branch_report,
            Plan => &self.since.plan,
            Spec => &self.since.spec,
            _ => return None,
        };
        task::age(since.as_deref()?, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on_list() -> TaskState {
        TaskState::default()
    }

    fn planned() -> TaskState {
        TaskState { planned: true, ..TaskState::default() }
    }

    /// Everything a task can have, on the list.
    fn everything() -> TaskState {
        TaskState {
            planned: true,
            has_work: true,
            has_refine: true,
            has_report: true,
            has_branch_report: true,
            has_spec: true,
            has_plan: true,
            ..TaskState::default()
        }
    }

    fn all_found() -> Found {
        Found { worktree: true, editor: true, speaking: false }
    }

    #[test]
    fn a_plain_task_on_the_list_can_be_refined_started_or_read() {
        assert_eq!(Item::of(on_list(), Found::default()), vec![Refine, Grill, Start, ReadAloud]);
    }

    /// A planned task can have its report generated; once one is linked,
    /// regenerated and opened, as two rows.
    #[test]
    fn reports_generate_on_a_planned_task_and_open_once_linked() {
        assert_eq!(Item::of(planned(), Found::default()), vec![Refine, Grill, Start, Report, ReadAloud]);
        let reported = TaskState { has_report: true, ..planned() };
        assert_eq!(Item::of(reported, Found::default()), vec![Refine, Grill, Start, Report, OpenReport, ReadAloud]);
        assert_eq!(Report.label(planned(), Found::default()), "Generate report");
        assert_eq!(Report.label(reported, Found::default()), "Regenerate report");
        // A report linked on an unplanned task still opens; there is no plan to build one from.
        let unplanned = TaskState { has_report: true, ..on_list() };
        assert_eq!(Item::of(unplanned, Found::default()), vec![Refine, Grill, Start, OpenReport, ReadAloud]);
    }

    #[test]
    fn everything_on_the_list_shows_every_row_in_order() {
        let every_but_what_a_live_claude_takes: Vec<Item> = Item::ALL.into_iter().filter(|i| ![Refine, Grill, Start].contains(i)).collect();
        assert_eq!(Item::of(everything(), all_found()), every_but_what_a_live_claude_takes);
    }

    /// Back to worktree and Go to work session would both focus a live work
    /// Claude, so only the session shows then; Back to worktree comes back
    /// once that Claude has gone, to start a fresh one.
    #[test]
    fn a_live_work_claude_shows_its_session_in_place_of_start() {
        let started = TaskState { active: true, ..on_list() };
        let live = TaskState { has_work: true, ..started };
        assert!(Item::of(started, Found::default()).contains(&Start));
        assert!(!Item::of(started, Found::default()).contains(&WorkSession));
        assert!(Item::of(live, Found::default()).contains(&WorkSession));
        assert!(!Item::of(live, Found::default()).contains(&Start));
    }

    /// Redo refine and Grill me would only focus a live refine Claude, as
    /// Go to refine session does, so only the session shows then; both come
    /// back once that Claude has gone. Generate report has its own tab, so
    /// it stays.
    #[test]
    fn a_live_refine_claude_shows_its_session_in_place_of_refine() {
        let live = TaskState { has_refine: true, ..planned() };
        assert_eq!(Item::of(planned(), Found::default()), vec![Refine, Grill, Start, Report, ReadAloud]);
        assert_eq!(Item::of(live, Found::default()), vec![Start, Report, RefineSession, ReadAloud]);
    }

    /// The herdr sessions each need their own Claude live.
    #[test]
    fn each_session_needs_its_own_claude() {
        let refine = TaskState { has_refine: true, ..on_list() };
        let work = TaskState { has_work: true, ..on_list() };
        assert!(Item::of(refine, Found::default()).contains(&RefineSession));
        assert!(!Item::of(refine, Found::default()).contains(&WorkSession));
        assert!(Item::of(work, Found::default()).contains(&WorkSession));
        assert!(!Item::of(work, Found::default()).contains(&RefineSession));
    }

    /// Off the list, only what works on any task: explaining the branch,
    /// the documents, the worktree, VS Code and reading aloud. Never refining, starting, a
    /// report built or a session.
    #[test]
    fn off_the_list_only_what_works_on_any_task() {
        for off in [TaskState { waiting: true, ..everything() }, TaskState { finished: true, ..everything() }] {
            assert_eq!(Item::of(off, all_found()), vec![Explain, OpenReport, BranchReport, Plan, Spec, Worktree, Editor, ReadAloud], "{off:?}");
        }
    }

    #[test]
    fn a_processing_task_gets_no_list() {
        let processing = TaskState { processing: true, ..everything() };
        assert_eq!(Item::of(processing, all_found()), Vec::new());
        assert_eq!(GoToList::new("a", "task a", processing, all_found(), Since::default()), None);
    }

    /// Explain branch shows on any task with a worktree, planned or not,
    /// finished or not; Branch report once its notes link one.
    #[test]
    fn a_worktree_can_have_its_branch_explained_and_opened() {
        let worktree = Found { worktree: true, ..Found::default() };
        assert_eq!(Item::of(on_list(), worktree), vec![Refine, Grill, Start, Explain, Worktree, ReadAloud]);
        assert!(!Item::of(on_list(), Found::default()).contains(&Explain), "no worktree, no branch");
        let finished = TaskState { finished: true, ..TaskState::default() };
        assert!(Item::of(finished, worktree).contains(&Explain));
        let explained = TaskState { has_branch_report: true, ..on_list() };
        assert!(Item::of(explained, Found::default()).contains(&BranchReport));
        assert_eq!(Explain.label(on_list(), worktree), "Explain branch");
        assert_eq!(BranchReport.label(explained, worktree), "Branch report");
        assert_eq!(Explain.args("u"), ["task", "report", "u", "--branch", "--fresh"]);
        assert_eq!(BranchReport.args("u"), ["task", "report", "u", "--branch"]);
        assert_eq!(Explain.group(), Group::Plan);
        assert_eq!(BranchReport.group(), Group::Document);
        let processing = TaskState { processing: true, ..on_list() };
        assert!(Item::of(processing, worktree).is_empty());
    }

    /// The branch report's row shows its age, from its note.
    #[test]
    fn the_branch_report_row_shows_its_age() {
        let l = link("/r/b.html", "20261006T010000Z");
        let since = Since::of(None, Some(&l), None, None, &|_| None);
        assert_eq!(since.branch_report.as_deref(), Some("20261006T010000Z"));
        let state = TaskState { has_branch_report: true, ..on_list() };
        let list = GoToList::new("u", "t", state, Found::default(), since).unwrap();
        assert!(list.row_age(BranchReport, crate::task::stamp_secs("20261008T010000Z").unwrap()).is_some());
    }

    /// The worktree and VS Code rows follow what was found.
    #[test]
    fn the_worktree_and_editor_rows_follow_what_was_found() {
        assert!(!Item::of(on_list(), Found::default()).contains(&Worktree));
        assert!(Item::of(on_list(), Found { worktree: true, ..Found::default() }).contains(&Worktree));
        assert!(!Item::of(on_list(), Found::default()).contains(&Editor));
        assert!(Item::of(on_list(), Found { editor: true, ..Found::default() }).contains(&Editor));
    }

    #[test]
    fn rows_read_as_the_step_they_take() {
        assert_eq!(Refine.label(on_list(), Found::default()), "Refine");
        assert_eq!(Refine.label(planned(), Found::default()), "Redo refine");
        assert_eq!(Start.label(on_list(), Found::default()), "Get started");
        assert_eq!(Start.label(TaskState { active: true, ..on_list() }, Found::default()), "Back to worktree");
        assert_eq!(ReadAloud.label(on_list(), Found::default()), "Read aloud");
        assert_eq!(ReadAloud.label(on_list(), Found { speaking: true, ..Found::default() }), "Stop reading aloud");
    }

    #[test]
    fn each_row_runs_its_command() {
        let u = "c53b6e3d";
        let run = |item: Item| item.args(u);
        assert_eq!(run(Refine), vec!["task", "refine", u]);
        assert_eq!(run(Grill), vec!["task", "refine", u, "--grill"]);
        assert_eq!(run(Start), vec!["task", "start", u]);
        assert_eq!(run(Report), vec!["task", "report", u, "--fresh"]);
        assert_eq!(run(Explain), vec!["task", "report", u, "--branch", "--fresh"]);
        assert_eq!(run(OpenReport), vec!["task", "report", u]);
        assert_eq!(run(BranchReport), vec!["task", "report", u, "--branch"]);
        assert_eq!(run(Plan), vec!["task", "open", u, "plan"]);
        assert_eq!(run(Spec), vec!["task", "open", u, "spec"]);
        assert_eq!(run(RefineSession), vec!["task", "session", u, "--refine"]);
        assert_eq!(run(WorkSession), vec!["task", "session", u, "--work"]);
        assert_eq!(run(Worktree), vec!["task", "open", u, "worktree"]);
        assert_eq!(run(Editor), vec!["task", "open", u, "editor"]);
        assert_eq!(run(ReadAloud), vec!["task", "speak", u]);
    }

    /// Icons and words alone tell the rows apart, whichever way the four
    /// that change read.
    #[test]
    fn every_row_has_its_own_icon_and_words() {
        let mut icons: Vec<&str> = Item::ALL.iter().map(|i| i.icon()).collect();
        icons.sort();
        icons.dedup();
        assert_eq!(icons.len(), Item::ALL.len());
        for (state, found) in [(on_list(), Found::default()), (everything(), Found { speaking: true, ..all_found() })] {
            let mut words: Vec<&str> = Item::ALL.iter().map(|i| i.label(state, found)).collect();
            words.sort();
            words.dedup();
            assert_eq!(words.len(), Item::ALL.len(), "{state:?}");
        }
    }

    #[test]
    fn only_read_aloud_keeps_the_keyboard() {
        let keeps: Vec<Item> = Item::ALL.into_iter().filter(|i| i.keeps_keyboard()).collect();
        assert_eq!(keeps, vec![ReadAloud]);
    }

    #[test]
    fn a_new_list_highlights_the_first_row_and_steps_to_the_ends() {
        let mut list = GoToList::new("a", "task a", everything(), all_found(), Since::default()).unwrap();
        assert_eq!(list.uuid(), "a");
        assert_eq!(list.at(), 0);
        list.step(false);
        assert_eq!(list.at(), 0, "stops at the first");
        for _ in 0..20 {
            list.step(true);
        }
        assert_eq!(list.at(), list.rows().len() - 1, "stops at the last");
        assert_eq!(list.item(list.at()), Some(ReadAloud));
        assert_eq!(list.item(99), None);
    }

    #[test]
    fn the_title_names_the_button_and_the_task_and_rows_wear_their_icon() {
        let list = GoToList::new("a", "Fix the peek", planned(), Found::default(), Since::default()).unwrap();
        assert_eq!(list.title(), "Go to…: Fix the peek");
        assert_eq!(list.row_label(Refine), "\u{f0d0}  Redo refine");
        assert_eq!(list.rows(), [Refine, Grill, Start, Report, ReadAloud]);
    }

    /// Similar rows share a colour, and no two groups have the same one.
    #[test]
    fn rows_are_coloured_by_their_group() {
        assert_eq!(Refine.group(), Grill.group());
        assert_eq!(Grill.group(), Report.group());
        assert_eq!(RefineSession.group(), WorkSession.group());
        assert_eq!(OpenReport.group(), Plan.group());
        assert_eq!(Plan.group(), Spec.group());
        assert_eq!(Worktree.group(), Editor.group());
        assert_ne!(Start.group(), WorkSession.group());
        for (i, a) in Group::ALL.iter().enumerate() {
            for b in &Group::ALL[i + 1..] {
                assert_ne!(a.colour(), b.colour(), "{a:?} and {b:?}");
                assert_ne!(a.class(), b.class());
            }
        }
    }

    fn link(path: &str, entry: &str) -> crate::task::Link {
        crate::task::Link { path: path.into(), entry: entry.into() }
    }

    /// 2026-10-08T00:00:00Z, four days after the links below were made.
    const NOW: i64 = 1_791_417_600;

    /// Each document row shows how long ago its note linked it; no other
    /// row has an age, and a document not linked has none.
    #[test]
    fn document_rows_show_their_age_and_others_none() {
        assert_eq!(crate::task::stamp_secs("20261008T000000Z"), Some(NOW));
        let since = Since::of(
            Some(&link("/r/a.html", "20261004T000000Z")),
            None,
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
        let since = Since::of(t.report_link().as_ref(), None, None, None, &|_| None);
        let list = GoToList::new("a", "d", everything(), all_found(), since).unwrap();
        assert_eq!(list.row_age(OpenReport, NOW).as_deref(), Some("12h"));
    }

    /// A note's date that cannot be read gives way to the file's last
    /// change, found by the note's path; with neither, no age.
    #[test]
    fn an_unreadable_note_date_falls_back_to_the_files() {
        let modified = |path: &str| (path == "docs/p.md").then_some(NOW - 3 * 3600);
        let since = Since::of(Some(&link("/r/gone.html", "someday")), None, Some(&link("docs/p.md", "")), None, &modified);
        assert_eq!(since, Since { report: None, branch_report: None, plan: Some("20261007T210000Z".into()), spec: None });
        let list = GoToList::new("a", "d", everything(), all_found(), since).unwrap();
        assert_eq!(list.row_age(Plan, NOW).as_deref(), Some("3h"));
        assert_eq!(list.row_age(OpenReport, NOW), None);
    }
}
