//! What the task panel shows, as plain data.
//!
//! Split from the drawing: the ordering, icons and cap are the parts worth
//! testing, and none of them needs a compositor.

use crate::actions::TaskState;
use crate::task::{Listing, Task};

/// The most task cards a panel shows. Past this, the rest fold into one
/// "+N more" card rather than running off the bottom of the screen.
pub const CAP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Pending,
    Blocked,
    /// Worked up into a plan by the `refine-task` skill.
    Planned,
    /// Parked with the Waiting button. Off the hover panel and the peek, and
    /// on the keyboard's Waiting tab only.
    Waiting,
    /// Completed, on the keyboard's Finished tab only, as a waiting task is
    /// on the Waiting tab only.
    Finished,
    /// The "+N more" card standing in for everything past the cap.
    More,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub status: Status,
    pub text: String,
    /// The task a click on the card acts on. `None` on the "+N more" card,
    /// which stands for no one task.
    pub uuid: Option<String>,
    /// The task carries `+planned`. Apart from `status`, which shows a
    /// started or blocked planned task as Active or Blocked: the Planned and
    /// To refine tabs go by the tag, whatever the card's icon says.
    pub planned: bool,
    /// The task carries `+next`, whatever its status: an active task up next
    /// is still up next, and its Up next button offers to clear it. Whether
    /// the card is drawn yellow is [`Card::shows_up_next`].
    pub up_next: bool,
    /// The stamp the age at the card's right end counts from: when the
    /// task was added, or, on a finished card, when it was finished. The
    /// stamp, not the age: the age changes every minute, and a changed card
    /// re-renders the panel, disarming a half-pressed Remove. Empty on
    /// "+N more".
    pub since: String,
    /// The task's notes, oldest first, text only: what the keyboard's panel
    /// shows under the description once the card's body is pressed. Empty on
    /// "+N more" and on a task with none.
    pub notes: Vec<String>,
}

impl Card {
    /// The status icon, which is what the peek shows first.
    pub fn icon(&self) -> &'static str {
        match self.status {
            Status::Active => "▶",
            Status::Pending => "○",
            // Font Awesome's lock, from the Nerd Font waybar already uses: flat
            // and one colour, where the emoji lock is a picture.
            Status::Blocked => "\u{f023}",
            // The pending circle filled in: still waiting, but worked up.
            Status::Planned => "●",
            // Font Awesome's pause, the Waiting button's own icon.
            Status::Waiting => "\u{f04c}",
            // Font Awesome's check: done.
            Status::Finished => "\u{f00c}",
            // The count is the text, so the peek reads "+3".
            Status::More => "",
        }
    }

    /// Whether the card is drawn yellow, as up next. Not an active card,
    /// which stays green, nor a waiting or finished one, which keep their
    /// dimmed look: starting or parking a task keeps its `+next`, and the
    /// card shows the stronger fact. Every other card is yellow, the lock
    /// and the dot too.
    pub fn shows_up_next(&self) -> bool {
        self.up_next && !matches!(self.status, Status::Active | Status::Waiting | Status::Finished)
    }

    /// How long ago the task was added, or finished on a finished card,
    /// `now` in Unix seconds. None on "+N more", and on a stamp taskwarrior
    /// wrote in some other shape.
    pub fn age(&self, now: i64) -> Option<String> {
        crate::task::age(&self.since, now)
    }

    /// The task's state, as the task actions read it, with whether a Claude
    /// is on it. None on "+N more", which stands for no one task.
    pub fn state(&self, has_session: bool) -> Option<TaskState> {
        (self.status != Status::More).then(|| TaskState {
            active: self.status == Status::Active,
            waiting: self.status == Status::Waiting,
            finished: self.status == Status::Finished,
            planned: self.planned,
            up_next: self.up_next,
            has_session,
        })
    }
}

/// A filter tab on the keyboard's panel: which of the cards it shows.
///
/// A filter, not a status: Planned and To refine go by the `+planned` tag,
/// which a card's icon can hide behind ▶ or the lock. A started task is
/// under Active and not Planned, which is for picking what to start next. A
/// waiting task is under Waiting and no other tab, and a finished one under
/// Finished: it is parked or done, and All is what the hover shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    /// Started tasks.
    Active,
    /// Tasks carrying `+planned`, from Refine or Grill me, and not started.
    Planned,
    /// Tasks without `+planned`: the ones still worth refining.
    ToRefine,
    /// Tasks parked as waiting.
    Waiting,
    /// Tasks completed lately, the last [`crate::task::FINISHED_CAP`].
    Finished,
}

impl Filter {
    /// The tabs left to right, which is also the order 1 to 6 pick them in.
    pub const TABS: [Filter; 6] = [
        Filter::All,
        Filter::Active,
        Filter::Planned,
        Filter::ToRefine,
        Filter::Waiting,
        Filter::Finished,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Planned => "Planned",
            Filter::ToRefine => "To refine",
            Filter::Waiting => "Waiting",
            Filter::Finished => "Finished",
        }
    }

    /// The one line a tab with nothing under it shows in place of cards. Only
    /// All is ever shown empty, on a workspace whose tasks are all waiting or finished;
    /// every other tab is hidden while it has nothing.
    pub fn empty_text(self) -> &'static str {
        match self {
            Filter::All => "No tasks",
            Filter::Active => "No active tasks",
            Filter::Planned => "No planned tasks",
            Filter::ToRefine => "No tasks to refine",
            Filter::Waiting => "No waiting tasks",
            Filter::Finished => "No finished tasks",
        }
    }

    /// Whether this tab shows the card. Meant for the uncapped cards: the
    /// "+N more" card stands for no one task, so it is made after filtering.
    pub fn matches(self, card: &Card) -> bool {
        match self {
            Filter::Waiting => card.status == Status::Waiting,
            Filter::Finished => card.status == Status::Finished,
            _ if matches!(card.status, Status::Waiting | Status::Finished) => false,
            Filter::All => true,
            Filter::Active => card.status == Status::Active,
            Filter::Planned => card.planned && card.status != Status::Active,
            Filter::ToRefine => !card.planned,
        }
    }

    /// The cards this tab shows, in the order `cards` put them.
    pub fn pick(self, cards: &[Card]) -> Vec<Card> {
        cards.iter().filter(|c| self.matches(c)).cloned().collect()
    }

    /// The tabs worth showing over these cards, in `TABS` order: All, where
    /// the panel opens, and every other tab with a task under it.
    pub fn shown(cards: &[Card]) -> Vec<Filter> {
        Filter::TABS
            .into_iter()
            .filter(|f| *f == Filter::All || cards.iter().any(|c| f.matches(c)))
            .collect()
    }

    /// The tasks under this tab, by uuid and uncapped: what Clear all on the
    /// Waiting tab deletes, which is every task the tab lists, those folded
    /// into "+N more" too.
    pub fn uuids(self, cards: &[Card]) -> Vec<String> {
        self.pick(cards).into_iter().filter_map(|c| c.uuid).collect()
    }
}

/// A tab on the keyboard's panel: a filter tab over the cards, or Ideas, the
/// workspace's notepad, which is not a filter and shows no cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Filter(Filter),
    Ideas,
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Filter(filter) => filter.label(),
            Tab::Ideas => "Ideas",
        }
    }

    /// The tabs on show, left to right: the filter tabs [`Filter::shown`]
    /// keeps, then Ideas, always and last.
    pub fn shown(cards: &[Card]) -> Vec<Tab> {
        Filter::shown(cards).into_iter().map(Tab::Filter).chain([Tab::Ideas]).collect()
    }
}

/// The task cards for one workspace tag, every one of them: the active tasks
/// first, then the up next ones, then the rest by priority, newest first,
/// with the blocked ones under the rest. Finished tasks, which only the
/// Finished tab shows, go last, the one finished most recently first.
///
/// Active first even when something else is higher priority: it is the work
/// in progress, and the one card worth reading without hovering. Up next
/// comes right under it, being the one to do next. Being started or up next
/// outranks being blocked, since you picked it anyway. Below them, a task
/// that cannot be started yet goes under every one that can. Newest first,
/// not urgency: taskwarrior's age coefficient lifts old tasks, and a task
/// just added is the one most likely to matter. Due dates count for nothing
/// here.
///
/// Whether a task is waiting or finished is which part of the [`Listing`] it
/// is in, not its status: taskwarrior 2.6 exports a waiting task as
/// `pending`.
pub fn cards(listing: &Listing, blocked: &[String]) -> Vec<Card> {
    // Only below up next: an active or up next task stays on top blocked.
    let sinks = |t: &Task| !t.is_active() && !t.is_up_next() && blocked.contains(&t.uuid);
    // Each task with the part it came from, which is what makes it waiting or
    // finished: no status string is read. Merged pending, waiting, completed,
    // the order the parts used to arrive in as one list, so the stable sort
    // keeps full ties where they were.
    let parts = [
        (&listing.pending, Part::Pending),
        (&listing.waiting, Part::Waiting),
        (&listing.completed, Part::Finished),
    ];
    let mut sorted: Vec<(&Task, Part)> =
        parts.iter().flat_map(|&(tasks, part)| tasks.iter().map(move |t| (t, part))).collect();
    let finished = |part: Part| part == Part::Finished;
    sorted.sort_by(|&(a, pa), &(b, pb)| {
        finished(pa).cmp(&finished(pb)).then_with(|| {
            if finished(pa) {
                // Both finished: the one finished last on top.
                return b.end.cmp(&a.end);
            }
            b.is_active()
                .cmp(&a.is_active())
                .then(b.is_up_next().cmp(&a.is_up_next()))
                .then(sinks(a).cmp(&sinks(b)))
                .then(b.priority_rank().cmp(&a.priority_rank()))
                // The stamp sorts as text the way it does as a time.
                .then(b.entry.cmp(&a.entry))
        })
    });

    sorted
        .iter()
        .map(|&(t, part)| Card {
            // Finished first, then waiting, both by the part the task came
            // from: parking a task stops it, so a waiting task is not the
            // work in progress whatever else it carries.
            status: if finished(part) {
                Status::Finished
            } else if part == Part::Waiting {
                Status::Waiting
            } else if t.is_active() {
                Status::Active
            } else if blocked.contains(&t.uuid) {
                Status::Blocked
            } else if t.is_planned() {
                Status::Planned
            } else {
                Status::Pending
            },
            text: crate::text::collapse_whitespace(&t.description),
            uuid: Some(t.uuid.clone()),
            planned: t.is_planned(),
            up_next: t.is_up_next(),
            since: if finished(part) { t.end.clone() } else { t.entry.clone() },
            notes: t.annotations.iter().map(|a| a.description.clone()).collect(),
        })
        .collect()
}

/// Which part of a [`Listing`] a task came from, for [`cards`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Pending,
    Waiting,
    Finished,
}

/// The first `n` cards, and past that one "+N more" card for the rest.
///
/// Kept apart from `cards` because the panel holds every card and caps them
/// itself: the "+N more" card shows the rest in place.
pub fn cap(cards: &[Card], n: usize) -> Vec<Card> {
    let mut shown = cards[..n.min(cards.len())].to_vec();
    if cards.len() > n {
        shown.push(Card {
            status: Status::More,
            text: format!("+{} more", cards.len() - n),
            uuid: None,
            planned: false,
            up_next: false,
            since: String::new(),
            notes: Vec::new(),
        });
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A task added on this day of October 2026: a higher day is newer, so
    /// it sorts higher, as a higher urgency used to.
    fn task(uuid: &str, day: u32, active: bool) -> Task {
        Task {
            uuid: uuid.into(),
            description: uuid.into(),
            urgency: 0.0,
            start: active.then(|| "20260927T080000Z".to_string()),
            wait: None,
            annotations: Vec::new(),
            tags: Vec::new(),
            status: "pending".into(),
            entry: format!("202610{day:02}T120000Z"),
            priority: None,
            end: String::new(),
        }
    }

    /// A listing of tasks still to do and nothing parked or finished.
    fn todo(pending: Vec<Task>) -> Listing {
        Listing { pending, ..Listing::default() }
    }

    /// A listing of finished tasks alone.
    fn done(completed: Vec<Task>) -> Listing {
        Listing { completed, ..Listing::default() }
    }

    /// A listing of parked tasks alone.
    fn parked(waiting: Vec<Task>) -> Listing {
        Listing { waiting, ..Listing::default() }
    }

    fn texts(cards: &[Card]) -> Vec<&str> {
        cards.iter().map(|c| c.text.as_str()).collect()
    }

    fn planned(uuid: &str, active: bool) -> Task {
        let mut t = task(uuid, 1, active);
        t.tags = vec![crate::task::PLANNED_TAG.into()];
        t
    }

    fn up_next(uuid: &str, day: u32, active: bool) -> Task {
        let mut t = task(uuid, day, active);
        t.tags = vec![crate::task::UP_NEXT_TAG.into()];
        t
    }

    fn with_priority(mut t: Task, p: &str) -> Task {
        t.priority = Some(p.into());
        t
    }

    #[test]
    fn no_tasks_no_cards() {
        assert!(cards(&Listing::default(), &[]).is_empty());
    }

    #[test]
    fn active_first_then_newest() {
        let got = cards(
            &todo(vec![task("low", 1, false), task("started", 2, true), task("high", 9, false)]),
            &[],
        );
        assert_eq!(texts(&got), vec!["started", "high", "low"]);
        assert_eq!(got[0].uuid.as_deref(), Some("started"), "a card knows its task");
        assert_eq!(got[0].status, Status::Active);
        assert_eq!(got[1].status, Status::Pending);
    }

    #[test]
    fn blocked_tasks_are_marked() {
        let got = cards(&todo(vec![task("waits", 1, false)]), &["waits".into()]);
        assert_eq!(got[0].status, Status::Blocked);
        assert_eq!(got[0].icon(), "\u{f023}");
    }

    #[test]
    fn started_outranks_blocked() {
        let got = cards(&todo(vec![task("both", 1, true)]), &["both".into()]);
        assert_eq!(got[0].status, Status::Active);
    }

    #[test]
    fn past_the_cap_the_rest_fold_into_one_card() {
        let many: Vec<Task> = (0..11).map(|i| task(&format!("t{i}"), i as u32 + 1, false)).collect();
        let got = cap(&cards(&todo(many), &[]), CAP);
        assert_eq!(got.len(), CAP + 1);
        assert_eq!(got[0].text, "t10", "the cap keeps the newest");
        let last = got.last().unwrap();
        assert_eq!(last.status, Status::More);
        assert_eq!(last.text, "+3 more");
        assert_eq!(last.icon(), "");
        assert_eq!(last.uuid, None);
    }

    #[test]
    fn exactly_the_cap_has_no_more_card() {
        let many: Vec<Task> = (0..CAP).map(|i| task(&format!("t{i}"), 1, false)).collect();
        let got = cap(&cards(&todo(many), &[]), CAP);
        assert_eq!(got.len(), CAP);
        assert!(got.iter().all(|c| c.status != Status::More));
    }

    #[test]
    fn descriptions_are_one_line() {
        let mut t = task("x", 1, false);
        t.description = "two\nlines".into();
        assert_eq!(cards(&todo(vec![t]), &[])[0].text, "two lines");
    }

    /// A card carries its task's notes, in order and text only, for the
    /// keyboard's panel to show under the description; "+N more" has none.
    #[test]
    fn a_card_carries_its_tasks_notes() {
        let mut noted = task("n", 2, false);
        noted.annotations = vec![
            crate::task::Annotation { entry: "20261001T120000Z".into(), description: "first note".into() },
            crate::task::Annotation { entry: "20261002T120000Z".into(), description: "second note".into() },
        ];
        let cards = cards(&todo(vec![noted, task("plain", 1, false)]), &[]);
        assert_eq!(cards[0].notes, vec!["first note".to_string(), "second note".to_string()]);
        assert!(cards[1].notes.is_empty());
        let capped = cap(&cards, 1);
        assert!(capped[1].notes.is_empty(), "+N more stands for no one task");
    }

    #[test]
    fn planned_tasks_get_the_filled_dot() {
        let got = cards(&todo(vec![planned("p", false)]), &[]);
        assert_eq!(got[0].status, Status::Planned);
        assert_eq!(got[0].icon(), "●");
    }

    #[test]
    fn started_outranks_planned() {
        let got = cards(&todo(vec![planned("p", true)]), &[]);
        assert_eq!(got[0].status, Status::Active);
    }

    /// A planned task that is waiting on another still cannot be started, and
    /// the lock is what says so.
    #[test]
    fn blocked_outranks_planned() {
        let got = cards(&todo(vec![planned("p", false)]), &["p".into()]);
        assert_eq!(got[0].status, Status::Blocked);
    }

    #[test]
    fn planned_does_not_change_the_order() {
        let low = planned("low", false);
        let got = cards(&todo(vec![low, task("high", 9, false)]), &[]);
        assert_eq!(texts(&got), vec!["high", "low"]);
    }

    #[test]
    fn a_card_knows_its_task_is_planned_whatever_its_status() {
        let got = cards(&todo(vec![planned("started", true), planned("waits", false), task("plain", 1, false)]), &["waits".into()]);
        assert_eq!(got[0].status, Status::Active);
        assert!(got[0].planned, "a started planned task is still planned");
        let waits = got.iter().find(|c| c.text == "waits").unwrap();
        assert_eq!(waits.status, Status::Blocked);
        assert!(waits.planned, "a blocked planned task is still planned");
        assert!(!got.iter().find(|c| c.text == "plain").unwrap().planned);
    }

    #[test]
    fn the_more_card_is_not_planned() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| planned(&format!("t{i}"), false)).collect();
        assert!(!cap(&cards(&todo(many), &[]), CAP).last().unwrap().planned);
    }

    /// A task parked with `wait:someday`. Taskwarrior still exports it as
    /// `pending`; it is waiting because it sits in the listing's waiting part.
    fn waiting(uuid: &str) -> Task {
        let mut t = task(uuid, 1, false);
        t.wait = Some("99991229T130000Z".into());
        t
    }

    #[test]
    fn the_tabs_run_all_active_planned_to_refine_waiting_finished() {
        let labels: Vec<&str> = Filter::TABS.iter().map(|f| f.label()).collect();
        assert_eq!(labels, vec!["All", "Active", "Planned", "To refine", "Waiting", "Finished"]);
    }

    /// A task added on 1 October and finished on this day.
    fn finished(uuid: &str, day: u32) -> Task {
        let mut t = task(uuid, 1, false);
        t.status = "completed".into();
        t.end = format!("202610{day:02}T180000Z");
        t
    }

    #[test]
    fn a_finished_task_gets_the_check() {
        let got = cards(&done(vec![finished("done", 2)]), &[]);
        assert_eq!(got[0].status, Status::Finished);
        assert_eq!(got[0].icon(), "\u{f00c}");
    }

    /// Finished tasks go under every task still to do, the one finished
    /// last on top, whatever was added when.
    #[test]
    fn finished_tasks_go_last_most_recently_finished_first() {
        let listing = Listing {
            pending: vec![task("todo", 1, false), task("started", 1, true)],
            waiting: Vec::new(),
            completed: vec![finished("old", 2), finished("new", 5)],
        };
        let got = cards(&listing, &[]);
        assert_eq!(texts(&got), vec!["started", "todo", "new", "old"]);
    }

    /// Finished is finished: under the Finished tab and no other, so All
    /// stays what the hover shows.
    #[test]
    fn a_finished_task_is_only_under_finished() {
        let mut planned_done = finished("planned-done", 3);
        planned_done.tags = vec![crate::task::PLANNED_TAG.into()];
        let listing = Listing {
            pending: vec![task("plain", 9, false)],
            waiting: vec![waiting("parked")],
            completed: vec![finished("done", 4), planned_done],
        };
        let all = cards(&listing, &[]);
        assert_eq!(texts(&Filter::All.pick(&all)), vec!["plain"]);
        assert!(Filter::Planned.pick(&all).is_empty());
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["plain"]);
        assert_eq!(texts(&Filter::Waiting.pick(&all)), vec!["parked"]);
        assert_eq!(texts(&Filter::Finished.pick(&all)), vec!["done", "planned-done"]);
    }

    /// The age on a finished card is how long ago it was finished.
    #[test]
    fn a_finished_card_ages_from_when_it_was_finished() {
        let got = cards(&done(vec![finished("done", 6)]), &[]);
        assert_eq!(got[0].since, "20261006T180000Z");
        // 2026-10-06 12:00:00Z is 1_791_288_000, so 18:00 is 6h on.
        let later = 1_791_288_000 + 6 * 3_600 + 2 * 3_600;
        assert_eq!(got[0].age(later).as_deref(), Some("2h"));
    }

    /// A finished card's state is finished, and it is not drawn yellow even
    /// when the task still carries `+next`.
    #[test]
    fn a_finished_card_is_finished_and_never_yellow() {
        let mut next_done = finished("done", 2);
        next_done.tags = vec![crate::task::UP_NEXT_TAG.into()];
        let got = cards(&done(vec![next_done]), &[]);
        assert_eq!(got[0].state(false), Some(TaskState { finished: true, up_next: true, ..TaskState::default() }));
        assert!(!got[0].shows_up_next());
    }

    #[test]
    fn the_finished_tab_comes_after_waiting_and_before_ideas() {
        let listing = Listing {
            pending: vec![task("plain", 9, false)],
            waiting: vec![waiting("parked")],
            completed: vec![finished("done", 2)],
        };
        let all = cards(&listing, &[]);
        assert_eq!(
            Tab::shown(&all),
            vec![
                Tab::Filter(Filter::All),
                Tab::Filter(Filter::ToRefine),
                Tab::Filter(Filter::Waiting),
                Tab::Filter(Filter::Finished),
                Tab::Ideas,
            ]
        );
        assert_eq!(Filter::shown(&cards(&done(vec![finished("done", 2)]), &[])), vec![Filter::All, Filter::Finished]);
    }

    /// The part decides, never the status: a stored `waiting` whose date has
    /// passed is in the pending part, and its card is pending.
    #[test]
    fn a_waiting_status_in_the_pending_part_is_pending() {
        let mut stored = task("stored", 1, false);
        stored.status = "waiting".into();
        let got = cards(&todo(vec![stored]), &[]);
        assert_eq!(got[0].status, Status::Pending);
    }

    #[test]
    fn a_waiting_task_gets_the_pause_icon() {
        let got = cards(&parked(vec![waiting("parked")]), &[]);
        assert_eq!(got[0].status, Status::Waiting);
        assert_eq!(got[0].icon(), "\u{f04c}");
    }

    /// Parked is parked: a waiting task is under Waiting and nowhere else,
    /// planned or not, so All stays what the hover shows.
    #[test]
    fn a_waiting_task_is_only_under_waiting() {
        let mut planned_parked = waiting("planned-parked");
        planned_parked.tags = vec![crate::task::PLANNED_TAG.into()];
        let listing = Listing {
            pending: vec![task("plain", 9, false)],
            waiting: vec![waiting("parked"), planned_parked],
            completed: Vec::new(),
        };
        let all = cards(&listing, &[]);
        assert_eq!(texts(&Filter::All.pick(&all)), vec!["plain"]);
        assert!(Filter::Planned.pick(&all).is_empty());
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["plain"]);
        assert_eq!(texts(&Filter::Waiting.pick(&all)), vec!["parked", "planned-parked"]);
    }

    /// All always, since it is where the panel opens; the rest only when they
    /// have a task under them.
    #[test]
    fn only_tabs_with_tasks_are_shown() {
        let all = cards(&Listing { pending: vec![task("plain", 9, false)], waiting: vec![waiting("parked")], completed: Vec::new() }, &[]);
        assert_eq!(Filter::shown(&all), vec![Filter::All, Filter::ToRefine, Filter::Waiting]);
        assert_eq!(Filter::shown(&[]), vec![Filter::All]);
    }

    #[test]
    fn an_empty_tab_says_what_it_has_none_of() {
        assert_eq!(Filter::All.empty_text(), "No tasks");
        assert_eq!(Filter::Active.empty_text(), "No active tasks");
        assert_eq!(Filter::Planned.empty_text(), "No planned tasks");
        assert_eq!(Filter::ToRefine.empty_text(), "No tasks to refine");
        assert_eq!(Filter::Waiting.empty_text(), "No waiting tasks");
        assert_eq!(Filter::Finished.empty_text(), "No finished tasks");
    }

    /// A filter, not a status. A started planned task is under Active alone:
    /// Planned is for picking what to start, and it is started. Not under To
    /// refine either, which it is past.
    #[test]
    fn each_tab_picks_its_tasks_in_order() {
        let all = cards(
            &todo(vec![
                task("plain", 9, false),
                planned("started-planned", true),
                task("started", 5, true),
                planned("planned", false),
            ]),
            &[],
        );
        assert_eq!(texts(&Filter::All.pick(&all)), texts(&all));
        assert_eq!(texts(&Filter::Active.pick(&all)), vec!["started", "started-planned"]);
        assert_eq!(texts(&Filter::Planned.pick(&all)), vec!["planned"]);
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["started", "plain"]);
    }

    /// Clear all deletes what the Waiting tab lists, and nothing it does not.
    #[test]
    fn a_tabs_uuids_are_its_tasks_and_no_others() {
        let listing = Listing {
            pending: vec![task("plain", 9, false), task("started", 5, true)],
            waiting: vec![waiting("parked"), waiting("also-parked")],
            completed: Vec::new(),
        };
        let all = cards(&listing, &[]);
        assert_eq!(Filter::Waiting.uuids(&all), vec!["parked", "also-parked"]);
        assert!(Filter::Waiting.uuids(&cards(&todo(vec![task("plain", 9, false)]), &[])).is_empty());
    }

    /// Every one the tab lists, not just those on screen before "+N more".
    #[test]
    fn a_tabs_uuids_are_uncapped() {
        let many: Vec<Task> = (0..CAP + 2).map(|i| waiting(&format!("w{i}"))).collect();
        assert_eq!(Filter::Waiting.uuids(&cards(&parked(many), &[])).len(), CAP + 2);
    }

    /// Up next is the one to do next, so it sits right under the work in
    /// progress, even when something else is higher priority or newer.
    #[test]
    fn up_next_sorts_right_under_the_active_tasks() {
        let got = cards(
            &todo(vec![task("low", 1, false), task("started", 2, true), task("high", 9, false), up_next("next", 1, false)]),
            &[],
        );
        assert_eq!(texts(&got), vec!["started", "next", "high", "low"]);
    }

    /// Started outranks up next, as it outranks blocked: an active task
    /// stays first, and green.
    #[test]
    fn started_outranks_up_next() {
        let got = cards(&todo(vec![up_next("next", 9, false), task("started", 1, true)]), &[]);
        assert_eq!(texts(&got), vec!["started", "next"]);
    }

    /// The card knows its task is up next whatever its status, so the button
    /// can offer to clear it on an active card too.
    #[test]
    fn a_card_knows_its_task_is_up_next_whatever_its_status() {
        let got = cards(&todo(vec![up_next("started", 1, true), up_next("plain", 1, false), task("other", 1, false)]), &[]);
        assert!(got[0].up_next, "a started up next task is still up next");
        assert!(got[1].up_next);
        assert!(!got[2].up_next);
    }

    /// Yellow on every card but an active one, which stays green, and a
    /// waiting one, which keeps its Waiting look. The lock and the dot are
    /// yellow too.
    #[test]
    fn an_up_next_card_shows_yellow_unless_active_or_waiting() {
        let mut planned_next = up_next("planned", 1, false);
        planned_next.tags.push(crate::task::PLANNED_TAG.into());
        let mut waiting_next = up_next("parked", 1, false);
        waiting_next.wait = Some("99991229T130000Z".into());
        let listing = Listing {
            pending: vec![up_next("plain", 1, false), up_next("blocked", 1, false), planned_next, up_next("started", 1, true)],
            waiting: vec![waiting_next],
            completed: Vec::new(),
        };
        let got = cards(&listing, &["blocked".into()]);
        let shows = |text: &str| got.iter().find(|c| c.text == text).unwrap().shows_up_next();
        assert!(shows("plain"));
        assert!(shows("blocked"), "the lock is yellow too");
        assert!(shows("planned"), "the dot is yellow too");
        assert!(!shows("started"), "an active card stays green");
        assert!(!shows("parked"), "a waiting card keeps its look");
        assert!(!cards(&todo(vec![task("other", 1, false)]), &[])[0].shows_up_next());
    }

    /// The "+N more" card stands for no task, so it is never up next.
    #[test]
    fn the_more_card_is_not_up_next() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| up_next(&format!("t{i}"), 1, false)).collect();
        let more = cap(&cards(&todo(many), &[]), CAP).pop().unwrap();
        assert!(!more.up_next);
        assert!(!more.shows_up_next());
    }

    /// A card's status and tags are its task's state; "+N more" has none.
    #[test]
    fn a_cards_state_is_its_tasks() {
        let card = |status, planned, up_next| Card { status, text: "t".into(), uuid: Some("u".into()), planned, up_next, since: String::new(), notes: Vec::new() };
        assert_eq!(
            card(Status::Active, true, true).state(true),
            Some(TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true })
        );
        assert_eq!(
            card(Status::Waiting, false, false).state(false),
            Some(TaskState { waiting: true, ..TaskState::default() })
        );
        assert_eq!(card(Status::Blocked, false, false).state(false), Some(TaskState::default()));
        let more = cap(&[card(Status::Pending, false, false), card(Status::Pending, false, false)], 1).pop().unwrap();
        assert_eq!(more.status, Status::More);
        assert_eq!(more.state(true), None);
    }

    /// Ideas is a tab, not a filter: always on show, after the filter tabs.
    #[test]
    fn ideas_is_always_shown_last() {
        let all = cards(&Listing { pending: vec![task("plain", 9, false)], waiting: vec![waiting("parked")], completed: Vec::new() }, &[]);
        assert_eq!(
            Tab::shown(&all),
            vec![Tab::Filter(Filter::All), Tab::Filter(Filter::ToRefine), Tab::Filter(Filter::Waiting), Tab::Ideas]
        );
        assert_eq!(Tab::shown(&[]), vec![Tab::Filter(Filter::All), Tab::Ideas]);
    }

    #[test]
    fn a_tab_reads_as_its_filter_or_ideas() {
        assert_eq!(Tab::Filter(Filter::ToRefine).label(), "To refine");
        assert_eq!(Tab::Ideas.label(), "Ideas");
    }

    /// Newest first below up next, whatever the urgency: taskwarrior's age
    /// coefficient lifts old tasks, which is what this replaces.
    #[test]
    fn newest_first_whatever_the_urgency() {
        let mut old = task("old", 1, false);
        old.urgency = 20.0;
        let got = cards(&todo(vec![old, task("new", 9, false)]), &[]);
        assert_eq!(texts(&got), vec!["new", "old"]);
    }

    /// A task with no entry stamp counts as the oldest.
    #[test]
    fn a_task_without_an_entry_sorts_after_one_with_it() {
        let mut bare = task("bare", 9, false);
        bare.entry = String::new();
        let got = cards(&todo(vec![bare, task("old", 1, false)]), &[]);
        assert_eq!(texts(&got), vec!["old", "bare"]);
    }

    #[test]
    fn priority_outranks_age_h_over_m_over_l_over_none() {
        let got = cards(
            &todo(vec![
                task("none-new", 9, false),
                with_priority(task("l", 4, false), "L"),
                with_priority(task("h-old", 1, false), "H"),
                with_priority(task("m", 2, false), "M"),
                with_priority(task("h-new", 3, false), "H"),
            ]),
            &[],
        );
        assert_eq!(texts(&got), vec!["h-new", "h-old", "m", "l", "none-new"]);
    }

    /// Up next outranks priority, as active does.
    #[test]
    fn active_then_up_next_outrank_priority() {
        let got = cards(
            &todo(vec![with_priority(task("high", 9, false), "H"), up_next("next", 1, false), task("started", 1, true)]),
            &[],
        );
        assert_eq!(texts(&got), vec!["started", "next", "high"]);
    }

    /// Below up next, a task that cannot be started yet waits under every one
    /// that can, high priority or new as it is; priority then age order each
    /// half.
    #[test]
    fn blocked_sinks_below_the_unblocked_rest() {
        let got = cards(
            &todo(vec![
                with_priority(task("blocked-h", 9, false), "H"),
                task("blocked-new", 8, false),
                task("blocked-old", 2, false),
                task("free-old", 1, false),
                with_priority(task("free-l", 1, false), "L"),
            ]),
            &["blocked-h".into(), "blocked-new".into(), "blocked-old".into()],
        );
        assert_eq!(texts(&got), vec!["free-l", "free-old", "blocked-h", "blocked-new", "blocked-old"]);
    }

    /// Being blocked does not move an active or up next task off the top.
    #[test]
    fn active_and_up_next_stay_on_top_when_blocked() {
        let got = cards(
            &todo(vec![task("free", 9, false), up_next("next", 1, false), task("started", 1, true)]),
            &["next".into(), "started".into()],
        );
        assert_eq!(texts(&got), vec!["started", "next", "free"]);
    }

    /// The card keeps the stamp, not the age, so it reads the same minute to
    /// minute and the panel need not re-render to age it.
    #[test]
    fn a_card_ages_from_its_tasks_stamp() {
        let got = cards(&todo(vec![task("t", 6, false)]), &[]);
        assert_eq!(got[0].since, "20261006T120000Z");
        // 2026-10-06 12:00:00Z is 1_791_288_000 (Task 1's 12:09:29 less 569s).
        let later = 1_791_288_000 + 3 * 3_600;
        assert_eq!(got[0].age(later).as_deref(), Some("3h"));
    }

    /// "+N more" stands for no one task, so it has no age.
    #[test]
    fn the_more_card_has_no_age() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| task(&format!("t{i}"), i as u32 + 1, false)).collect();
        let more = cap(&cards(&todo(many), &[]), CAP).pop().unwrap();
        assert_eq!(more.since, "");
        assert_eq!(more.age(i64::MAX / 2), None);
    }
}
