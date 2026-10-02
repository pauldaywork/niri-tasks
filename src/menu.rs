//! The action menu: the fuzzel menu of what can be done to one task, as
//! plain data. A view of the task actions (`actions.rs`): which of them it
//! offers and in what order. Plus its two entries that are not one command,
//! Update status and Move to workspace, which ask a second question first.

use crate::actions::{Action, TaskState};

/// One row of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// A task action. Picking it runs the action's own `niritasks` command.
    Run(Action),
    /// Pick a state from `task::Status::ALL`, then move the task to it.
    Status,
    /// Pick a `~/Projects` folder, then retag the task to its workspace.
    Move,
}

/// The task actions the menu offers, in its order. Go to session leads,
/// while there is a Claude to go to. The row's Back to list, Stop, Waiting
/// and Remove are Update status's states here.
const ACTIONS: [Action; 8] = [
    Action::Session,
    Action::Edit,
    Action::Note,
    Action::Speak,
    Action::UpNext,
    Action::Refine,
    Action::Grill,
    Action::Start,
];

impl Entry {
    /// The row's words. `up_next` is whether the task is, for Up next's.
    pub fn label(self, up_next: bool) -> &'static str {
        match self {
            Entry::Run(action) => action.label(up_next),
            Entry::Status => "Update status",
            Entry::Move => "Move to workspace",
        }
    }
}

/// The menu for a task in `state`, top to bottom: the actions that apply,
/// then Update status and Move to workspace, which every task gets.
pub fn entries(state: TaskState) -> Vec<Entry> {
    ACTIONS
        .into_iter()
        .filter(|a| a.applies(state))
        .map(Entry::Run)
        .chain([Entry::Status, Entry::Move])
        .collect()
}

/// The entry fuzzel handed back, by its words. None for anything else:
/// fuzzel echoes typed text that matches no row.
pub fn picked(label: &str, state: TaskState) -> Option<Entry> {
    entries(state).into_iter().find(|e| e.label(state.up_next) == label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(state: TaskState) -> Vec<&'static str> {
        entries(state).into_iter().map(|e| e.label(state.up_next)).collect()
    }

    const ON_LIST: [&str; 9] = [
        "Edit", "Note", "Speak", "Up next", "Refine", "Grill me", "Start working", "Update status", "Move to workspace",
    ];

    #[test]
    fn a_task_on_the_list_gets_the_whole_menu() {
        assert_eq!(labels(TaskState::default()), ON_LIST);
    }

    /// Go to session leads the menu when there is a session to go to, and a
    /// task with none gets exactly the menu it always had.
    #[test]
    fn go_to_session_leads_the_menu_only_when_there_is_one() {
        let with = labels(TaskState { has_session: true, ..TaskState::default() });
        assert_eq!(with[0], "Go to session");
        assert_eq!(with[1..], ON_LIST);
    }

    /// The menu offers the step up next would take: Not up next, in the same
    /// place, on a task already up next.
    #[test]
    fn the_menu_offers_to_clear_up_next_on_a_task_up_next() {
        let marked = labels(TaskState { up_next: true, ..TaskState::default() });
        assert_eq!(marked[3], "Not up next");
        assert!(!marked.contains(&"Up next"));
    }

    /// Start working on an active task goes back to its worktree and its
    /// Claude, so the menu keeps it there; stopping is Update status's.
    #[test]
    fn an_active_task_keeps_start_working() {
        assert_eq!(labels(TaskState { active: true, ..TaskState::default() }), ON_LIST);
    }

    /// A waiting task is parked: no Start working, Refine or Grill me,
    /// which are for a task on the list, nor Up next or Go to session.
    /// Update status → Stopped brings it back.
    #[test]
    fn a_waiting_task_gets_no_start_working_or_refine() {
        for has_session in [false, true] {
            for up_next in [false, true] {
                let state = TaskState { waiting: true, has_session, up_next, ..TaskState::default() };
                assert_eq!(labels(state), ["Edit", "Note", "Speak", "Update status", "Move to workspace"], "{state:?}");
            }
        }
    }

    /// Every row the menu shows finds its own entry again, and no two rows
    /// read the same.
    #[test]
    fn every_row_picks_its_own_entry() {
        for waiting in [false, true] {
            for has_session in [false, true] {
                for up_next in [false, true] {
                    let state = TaskState { waiting, has_session, up_next, ..TaskState::default() };
                    for entry in entries(state) {
                        assert_eq!(picked(entry.label(up_next), state), Some(entry), "{state:?}");
                    }
                }
            }
        }
    }

    /// Typed text that matches no row, or a row this task's menu does not
    /// have, picks nothing.
    #[test]
    fn words_off_the_menu_pick_nothing() {
        assert_eq!(picked("Edti", TaskState::default()), None);
        assert_eq!(picked("Start working", TaskState { waiting: true, ..TaskState::default() }), None);
        assert_eq!(picked("Up next", TaskState { up_next: true, ..TaskState::default() }), None);
        assert_eq!(picked("Stop", TaskState { active: true, ..TaskState::default() }), None);
    }

    #[test]
    fn the_status_and_move_rows_run_their_own_pickers() {
        assert_eq!(picked("Update status", TaskState::default()), Some(Entry::Status));
        assert_eq!(picked("Move to workspace", TaskState::default()), Some(Entry::Move));
        assert_eq!(picked("Edit", TaskState::default()), Some(Entry::Run(Action::Edit)));
    }
}
