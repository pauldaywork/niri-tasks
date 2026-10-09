//! The project list: the `~/Projects` folders, and for Mod+Alt+W the GitHub
//! repos not cloned yet, that the task panel shows in place of the task
//! cards, narrowed by what is typed, and what picking one does. Plain data,
//! like the rest of the panel state: the ranking is handed in, so the daemon
//! ranks with fzf and a test with a substring match, and `surface.rs` only
//! draws what this says.

use crate::actions::Action;
use crate::project::{self, Choice, Projects, Row};

/// What the project list is for, and so what picking a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// Move to workspace: the task being moved, by uuid, and its
    /// description, for the line over the folders.
    Move { uuid: String, text: String },
    /// Mod+Alt+W: open the project picked on its own workspace, cloning a
    /// repo or making a folder of a new name, as `project open` does. It has
    /// no task, so it shows with no cards and on an unnamed workspace too:
    /// opening a project is how a workspace gets a name.
    Open,
}

/// Ranks `names` for `query`, best first. The daemon hands in fzf with its
/// substring fallback; tests hand in
/// [`substring_matches`](super::matching::substring_matches).
pub type Matcher<'a> = &'a dyn Fn(&[String], &str) -> Vec<String>;

/// What Enter or a click on the list does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    /// Run `niritasks` with these arguments: `project open -- <name>`, or
    /// `task move <uuid> <name>`.
    Spawn(Vec<String>),
    /// No row is shown and nothing typed could be one.
    Nothing,
}

/// What Escape did to the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escaped {
    /// Text was typed: it is cleared and every row shows again.
    Cleared,
    /// Nothing was typed: the list is done with.
    Closed,
}

/// What the panel says when Move to workspace finds no folder to move to.
pub const NO_DESTINATIONS: &str = "No other project to move this to.";

/// The footer while Move's list shows: typing narrows it, and these keys act.
pub const MOVE_KEYS: &str = "Type to filter · Up, Down: pick · Enter: move the task there · Esc: clear, then back";

/// The line over the list Mod+Alt+W opens.
pub const OPEN_TITLE: &str = "Open a project";

/// The footer while that list shows: Enter opens, and Escape with nothing
/// typed closes the panel, there being no cards it was opened from.
pub const OPEN_KEYS: &str = "Type to filter · Up, Down: pick · Enter: open it · Esc: clear, then close";

/// What the open list says with no folder to show and nothing typed: an
/// empty `~/Projects`, where the first project has to be made.
pub const TYPE_A_NAME: &str = "Type a name to make a new project";

/// A folder row's icon: Font Awesome's folder, Move to workspace's open one
/// shut.
const FOLDER_ICON: &str = "\u{f07b}";

/// A GitHub repo's icon: Font Awesome's GitHub mark, so a repo not cloned
/// yet reads apart from a folder.
const GITHUB_ICON: &str = "\u{f09b}";

/// The project list: every row, what is typed, the rows that match it and
/// the one the highlight is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectList {
    purpose: Purpose,
    /// Every row, in list order: folders, then repos.
    rows: Vec<Row>,
    /// What is typed in the field over the list.
    query: String,
    /// The rows matching `query`, best first, or every row with nothing
    /// typed.
    shown: Vec<Row>,
    /// The row in `shown` the highlight is on: Up, Down and Enter's.
    at: usize,
}

impl ProjectList {
    /// A list of `rows` for `purpose`, every row shown and the first
    /// highlighted.
    pub fn new(purpose: Purpose, rows: Vec<Row>) -> ProjectList {
        ProjectList { purpose, shown: rows.clone(), rows, query: String::new(), at: 0 }
    }

    /// What the list is for: moving a task, or opening a project.
    pub fn purpose(&self) -> &Purpose {
        &self.purpose
    }

    /// The task being moved, on Move to workspace's list.
    pub fn moving(&self) -> Option<&str> {
        match &self.purpose {
            Purpose::Move { uuid, .. } => Some(uuid),
            Purpose::Open => None,
        }
    }

    /// The rows matching what is typed, best first, or every row with
    /// nothing typed.
    pub fn shown(&self) -> &[Row] {
        &self.shown
    }

    /// The index in `shown` the highlight is on: Up, Down and Enter's.
    pub fn at(&self) -> usize {
        self.at
    }

    /// What is typed in the field over the list.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Narrow the list to `query`, ranked by `matcher`, the highlight back
    /// on the top match. False when the text is what it was, as when Escape
    /// has just emptied the field.
    pub fn narrow(&mut self, query: &str, matcher: Matcher) -> bool {
        if self.query == query {
            return false;
        }
        self.query = query.to_string();
        self.shown = if query.is_empty() {
            self.rows.clone()
        } else {
            let names: Vec<String> = self.rows.iter().map(|r| r.name().to_string()).collect();
            matcher(&names, query)
                .iter()
                .filter_map(|name| self.rows.iter().find(|r| r.name() == name).cloned())
                .collect()
        };
        self.at = 0;
        true
    }

    /// Up or Down, stopping at the ends. False with nothing shown.
    pub fn step(&mut self, forward: bool) -> bool {
        if self.shown.is_empty() {
            return false;
        }
        self.at = super::keys::step(self.at, self.shown.len(), forward);
        true
    }

    /// Escape: with text typed, clear it and show every row again; with
    /// none, the list is done with.
    pub fn escape(&mut self) -> Escaped {
        if self.query.is_empty() {
            return Escaped::Closed;
        }
        self.query.clear();
        self.shown = self.rows.clone();
        self.at = 0;
        Escaped::Cleared
    }

    /// What the typed text means when the open list shows nothing: a new
    /// name, a folder the plain-text ranking missed ("my project" for
    /// "my-project"), or a repo. None on Move, and while anything shows.
    fn typed(&self, projects: &Projects) -> Option<Choice> {
        if self.purpose != Purpose::Open || !self.shown.is_empty() {
            return None;
        }
        Some(projects.choice(&self.query))
    }

    /// The name a pick at `at` means: the shown row there, or, on the open
    /// list with nothing shown, what the typed text names.
    fn picked_name(&self, at: usize, projects: &Projects) -> Option<String> {
        if let Some(row) = self.shown.get(at) {
            return Some(row.name().to_string());
        }
        match self.typed(projects)? {
            Choice::Open(name) | Choice::Create(name) | Choice::Clone(name) => Some(name),
            Choice::Rejected(_) | Choice::Nothing => None,
        }
    }

    /// Enter or a click at `at`: the `niritasks` words that open the project
    /// or move the task, by purpose. `projects` is what the rows came from,
    /// so a typed name means here what `project open` will take it to mean.
    pub fn pick(&self, at: usize, projects: &Projects) -> Picked {
        let Some(name) = self.picked_name(at, projects) else {
            return Picked::Nothing;
        };
        Picked::Spawn(match &self.purpose {
            Purpose::Open => project::open_args(&name),
            Purpose::Move { uuid, .. } => {
                let mut args = Action::Move.args(uuid);
                args.push(name);
                args
            }
        })
    }

    /// The line under the title when the open list shows nothing: what
    /// Enter will do with the text, decided by the same [`Projects::choice`]
    /// that `project open` will run, or, with nothing typed and no row at
    /// all, to type a name. None on Move, and while anything shows.
    pub fn hint(&self, projects: &Projects) -> Option<String> {
        Some(match self.typed(projects)? {
            Choice::Clone(name) => format!("Enter clones {name} from GitHub"),
            Choice::Create(name) => format!("Enter makes ~/Projects/{name}"),
            Choice::Open(name) => format!("Enter opens ~/Projects/{name}"),
            Choice::Rejected(why) => why,
            Choice::Nothing => TYPE_A_NAME.to_string(),
        })
    }

    /// The line over the rows: which task is moving, or that a project is
    /// being opened.
    pub fn title(&self) -> String {
        match &self.purpose {
            Purpose::Move { text, .. } => format!("{}: {text}", Action::Move.label(false)),
            Purpose::Open => OPEN_TITLE.to_string(),
        }
    }

    /// The footer under the list: the keys that act on it.
    pub fn keys(&self) -> &'static str {
        match self.purpose {
            Purpose::Move { .. } => MOVE_KEYS,
            Purpose::Open => OPEN_KEYS,
        }
    }

    /// A row as its card reads: the icon, then the name. The two spaces are
    /// the gap a task card leaves after its icon.
    pub fn label(row: &Row) -> String {
        match row {
            Row::Repo(name) => format!("{GITHUB_ICON}  {name}"),
            Row::Folder(name) => format!("{FOLDER_ICON}  {name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projects(local: &[&str], remote: &[&str]) -> Projects {
        Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn substring(names: &[String], query: &str) -> Vec<String> {
        crate::panel::matching::substring_matches(names, query)
    }

    fn folders(names: &[&str]) -> Vec<Row> {
        names.iter().map(|n| Row::Folder(n.to_string())).collect()
    }

    fn moving(rows: &[&str]) -> ProjectList {
        ProjectList::new(Purpose::Move { uuid: "a".into(), text: "task a".into() }, folders(rows))
    }

    fn opening(p: &Projects) -> ProjectList {
        ProjectList::new(Purpose::Open, p.rows())
    }

    #[test]
    fn a_new_list_shows_every_row_with_the_first_highlighted() {
        let list = moving(&["x", "y"]);
        assert_eq!(list.shown(), folders(&["x", "y"]));
        assert_eq!(list.at(), 0);
        assert_eq!(list.query(), "");
    }

    #[test]
    fn narrowing_ranks_the_matches_and_goes_back_to_the_top() {
        let mut list = moving(&["alpha", "beta", "gamma"]);
        assert!(list.step(true));
        assert!(list.narrow("a", &substring));
        assert_eq!(list.shown(), folders(&["alpha", "beta", "gamma"]));
        assert!(list.narrow("et", &substring));
        assert_eq!(list.shown(), folders(&["beta"]));
        assert_eq!(list.at(), 0);
    }

    #[test]
    fn the_same_text_again_is_no_change() {
        let mut list = moving(&["x", "y"]);
        assert!(!list.narrow("", &substring));
        assert!(list.narrow("y", &substring));
        assert!(!list.narrow("y", &substring));
    }

    #[test]
    fn an_empty_query_shows_every_row_without_asking_the_matcher() {
        let mut list = moving(&["x", "y"]);
        list.narrow("x", &substring);
        let never = |_: &[String], _: &str| -> Vec<String> { panic!("the matcher is not asked for an empty query") };
        assert!(list.narrow("", &never));
        assert_eq!(list.shown(), folders(&["x", "y"]));
    }

    #[test]
    fn stepping_stops_at_the_ends_and_does_nothing_on_nothing() {
        let mut list = moving(&["x", "y"]);
        // Up at the top still counts as a step: the highlight stays put, but
        // the list is redrawn as it would be anywhere else.
        assert!(list.step(false));
        assert_eq!(list.at(), 0);
        assert!(list.step(true));
        assert!(list.step(true));
        assert_eq!(list.at(), 1);
        list.narrow("zz", &substring);
        assert!(!list.step(true));
    }

    #[test]
    fn escape_clears_typed_text_first_and_closes_second() {
        let mut list = moving(&["x", "y"]);
        list.narrow("y", &substring);
        assert_eq!(list.escape(), Escaped::Cleared);
        assert_eq!(list.shown(), folders(&["x", "y"]));
        assert_eq!(list.query(), "");
        assert_eq!(list.escape(), Escaped::Closed);
    }

    #[test]
    fn a_pick_on_move_moves_the_task_to_the_shown_row() {
        let p = projects(&["x", "y"], &[]);
        let mut list = moving(&["x", "y"]);
        list.step(true);
        assert_eq!(
            list.pick(list.at(), &p),
            Picked::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])
        );
    }

    #[test]
    fn typed_text_is_never_a_pick_on_move() {
        let p = projects(&["x"], &[]);
        let mut list = moving(&["x"]);
        list.narrow("brand new", &substring);
        assert_eq!(list.pick(0, &p), Picked::Nothing);
        assert_eq!(list.hint(&p), None);
    }

    #[test]
    fn a_pick_on_open_opens_the_shown_row_by_its_bare_name() {
        let p = projects(&["alpha"], &["convo"]);
        let list = opening(&p);
        assert_eq!(list.pick(1, &p), Picked::Spawn(project::open_args("convo")));
    }

    #[test]
    fn a_typed_new_name_opens_once_nothing_shows() {
        let p = projects(&["alpha"], &[]);
        let mut list = opening(&p);
        list.narrow("my thing", &substring);
        assert_eq!(list.hint(&p), Some("Enter makes ~/Projects/my-thing".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-thing")));
    }

    /// The hint and the pick agree with project open: a typed name that is
    /// one of the repos clones, and says so.
    #[test]
    fn a_typed_repo_name_says_it_clones() {
        let p = projects(&["alpha"], &["my-repo"]);
        let mut list = opening(&p);
        list.narrow("my repo", &substring);
        assert_eq!(list.hint(&p), Some("Enter clones my-repo from GitHub".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-repo")));
    }

    #[test]
    fn a_folder_the_plain_ranking_missed_still_opens() {
        let p = projects(&["my-project"], &[]);
        let mut list = opening(&p);
        list.narrow("my project", &substring);
        assert_eq!(list.hint(&p), Some("Enter opens ~/Projects/my-project".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-project")));
    }

    #[test]
    fn rejected_text_is_no_pick_and_says_why() {
        let p = projects(&["alpha"], &[]);
        let mut list = opening(&p);
        list.narrow("a/b", &substring);
        assert_eq!(list.pick(0, &p), Picked::Nothing);
        assert!(list.hint(&p).is_some_and(|h| h.contains('/')));
    }

    #[test]
    fn an_empty_projects_folder_asks_for_a_name() {
        let p = projects(&[], &[]);
        let list = opening(&p);
        assert_eq!(list.hint(&p), Some(TYPE_A_NAME.into()));
        assert_eq!(list.pick(0, &p), Picked::Nothing);
    }

    #[test]
    fn no_hint_while_anything_shows() {
        let p = projects(&["alpha"], &[]);
        let list = opening(&p);
        assert_eq!(list.hint(&p), None);
    }

    #[test]
    fn titles_keys_and_labels() {
        let p = projects(&["alpha"], &["convo"]);
        assert_eq!(moving(&["x"]).title(), "Move to workspace: task a");
        assert_eq!(moving(&["x"]).keys(), MOVE_KEYS);
        assert_eq!(opening(&p).title(), OPEN_TITLE);
        assert_eq!(opening(&p).keys(), OPEN_KEYS);
        assert_eq!(ProjectList::label(&Row::Repo("convo".into())), format!("{GITHUB_ICON}  convo"));
        assert_eq!(ProjectList::label(&Row::Folder("alpha".into())), format!("{FOLDER_ICON}  alpha"));
        assert_eq!(moving(&["x"]).moving(), Some("a"));
        assert_eq!(opening(&p).moving(), None);
    }
}
