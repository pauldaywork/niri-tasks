//! The Go to list: what can be done with a task and where it can be gone
//! to, which the task panel shows in place of the cards when the card's
//! Go to… is pressed — refining and starting it, its report, its plan and
//! spec, its two herdr sessions, its worktree, VS Code, and reading it
//! aloud — each row only when it applies, and what picking one runs. Plain
//! data, like the project list: the rows are read off the task's
//! `TaskState` and what the surface found on the machine ([`Found`]), and
//! `surface.rs` only draws what this says.

use crate::actions::{Action, TaskState};

/// The button's words, and the start of the list's title.
pub const LABEL: &str = "Go to…";

/// The footer while the Go to list shows. No text field: there are twelve
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
    /// The report its notes link, `Report: <path>`, in the browser.
    OpenReport,
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

impl Item {
    /// Every row, in the order the list shows the ones that apply.
    pub const ALL: [Item; 12] =
        [Refine, Grill, Start, Report, OpenReport, Plan, Spec, RefineSession, WorkSession, Worktree, Editor, ReadAloud];

    /// Whether the row shows for a task in `state`, with what was `found`.
    /// Refining, starting and the herdr sessions are for a task on the
    /// list, as their buttons were; generating a report for a planned one;
    /// the documents whenever the notes link them, the worktree whenever
    /// there is one, VS Code whenever it is installed, and reading aloud
    /// always. Nothing on a processing task: a Claude is writing it.
    pub fn applies(self, state: TaskState, found: Found) -> bool {
        if state.processing {
            return false;
        }
        let on_list = !state.off_list();
        match self {
            Refine | Grill | Start => on_list,
            Report => on_list && state.planned,
            OpenReport => state.has_report,
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
            OpenReport => "Open report",
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

    /// The row's glyph, from the Nerd Font the cards use; no two alike.
    /// Font Awesome's: magic wand (Refine's own), comments (Grill me's),
    /// play (Start's), file-text outline, file-text, numbered list,
    /// lightbulb, comments outline, terminal, open folder outline and code;
    /// Read aloud's speaker is Material Design's, Speak's old one.
    pub fn icon(self) -> &'static str {
        match self {
            Refine => "\u{f0d0}",
            Grill => "\u{f086}",
            Start => "\u{f04b}",
            Report => "\u{f0f6}",
            OpenReport => "\u{f15c}",
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
            OpenReport => &["task", "report", uuid],
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
    rows: Vec<Item>,
    /// The row the highlight is on: Up, Down and Enter's.
    at: usize,
}

impl GoToList {
    /// The list for the task `uuid`, described `text`, in `state`, with
    /// what was `found`, the first row highlighted. None with no row: a
    /// processing task.
    pub fn new(uuid: &str, text: &str, state: TaskState, found: Found) -> Option<GoToList> {
        let rows = Item::of(state, found);
        (!rows.is_empty()).then(|| GoToList { uuid: uuid.into(), text: text.into(), state, found, rows, at: 0 })
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
        assert_eq!(Item::of(everything(), all_found()), Item::ALL.to_vec());
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

    /// Off the list, only what works on any task: the documents, the
    /// worktree, VS Code and reading aloud. Never refining, starting, a
    /// report built or a session.
    #[test]
    fn off_the_list_only_what_works_on_any_task() {
        for off in [TaskState { waiting: true, ..everything() }, TaskState { finished: true, ..everything() }] {
            assert_eq!(Item::of(off, all_found()), vec![OpenReport, Plan, Spec, Worktree, Editor, ReadAloud], "{off:?}");
        }
    }

    #[test]
    fn a_processing_task_gets_no_list() {
        let processing = TaskState { processing: true, ..everything() };
        assert_eq!(Item::of(processing, all_found()), Vec::new());
        assert_eq!(GoToList::new("a", "task a", processing, all_found()), None);
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
        assert_eq!(run(OpenReport), vec!["task", "report", u]);
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
        let mut list = GoToList::new("a", "task a", everything(), all_found()).unwrap();
        assert_eq!(list.uuid(), "a");
        assert_eq!(list.at(), 0);
        list.step(false);
        assert_eq!(list.at(), 0, "stops at the first");
        for _ in 0..20 {
            list.step(true);
        }
        assert_eq!(list.at(), Item::ALL.len() - 1, "stops at the last");
        assert_eq!(list.item(list.at()), Some(ReadAloud));
        assert_eq!(list.item(99), None);
    }

    #[test]
    fn the_title_names_the_button_and_the_task_and_rows_wear_their_icon() {
        let list = GoToList::new("a", "Fix the peek", planned(), Found::default()).unwrap();
        assert_eq!(list.title(), "Go to…: Fix the peek");
        assert_eq!(list.row_label(Refine), "\u{f0d0}  Redo refine");
        assert_eq!(list.rows(), [Refine, Grill, Start, Report, ReadAloud]);
    }
}
