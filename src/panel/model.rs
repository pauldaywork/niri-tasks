//! What the task panel shows, as plain data.
//!
//! Split from the drawing for the same reason `rows.rs` is split from the
//! picker: the ordering, icons and cap are the parts worth testing, and none of
//! them needs a compositor.

use crate::task::Task;

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
            // The count is the text, so the peek reads "+3".
            Status::More => "",
        }
    }
}

/// A filter tab on the keyboard's panel: which of the cards it shows.
///
/// A filter, not a status: Planned and To refine go by the `+planned` tag,
/// which a card's icon can hide behind ▶ or the lock. A started task is
/// under Active and not Planned, which is for picking what to start next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    /// Started tasks.
    Active,
    /// Tasks carrying `+planned`, from Refine or Grill me, and not started.
    Planned,
    /// Tasks without `+planned`: the ones still worth refining.
    ToRefine,
}

impl Filter {
    /// The tabs left to right, which is also the order 1 to 4 pick them in.
    pub const TABS: [Filter; 4] = [Filter::All, Filter::Active, Filter::Planned, Filter::ToRefine];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Planned => "Planned",
            Filter::ToRefine => "To refine",
        }
    }

    /// The tab's label with how many tasks are under it, so a tab worth
    /// opening shows before it is opened.
    pub fn tab_label(self, count: usize) -> String {
        format!("{} {count}", self.label())
    }

    /// The one line a tab with nothing under it shows in place of cards.
    pub fn empty_text(self) -> &'static str {
        match self {
            Filter::All => "No tasks",
            Filter::Active => "No active tasks",
            Filter::Planned => "No planned tasks",
            Filter::ToRefine => "No tasks to refine",
        }
    }

    /// Whether this tab shows the card. Meant for the uncapped cards: the
    /// "+N more" card stands for no one task, so it is made after filtering.
    pub fn matches(self, card: &Card) -> bool {
        match self {
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

    /// How many of these cards each tab shows, in `TABS` order.
    pub fn counts(cards: &[Card]) -> [usize; 4] {
        Filter::TABS.map(|f| cards.iter().filter(|c| f.matches(c)).count())
    }
}

/// The task cards for one workspace tag, every one of them: the active task
/// first, then the rest most urgent first.
///
/// Active first even when something else is more urgent: it is the work in
/// progress, and the one card worth reading without hovering. Being started
/// outranks being blocked, since you started it anyway. Blocked outranks planned:
/// a plan does not make a task you cannot start yet startable.
pub fn cards(tasks: &[Task], blocked: &[String]) -> Vec<Card> {
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| {
        b.is_active().cmp(&a.is_active()).then(
            b.urgency
                .partial_cmp(&a.urgency)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    sorted
        .iter()
        .map(|t| Card {
            status: if t.is_active() {
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
        })
        .collect()
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
        });
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(uuid: &str, urgency: f64, active: bool) -> Task {
        Task {
            uuid: uuid.into(),
            description: uuid.into(),
            urgency,
            start: active.then(|| "20260927T080000Z".to_string()),
            annotations: Vec::new(),
            tags: Vec::new(),
            status: "pending".into(),
        }
    }

    fn texts(cards: &[Card]) -> Vec<&str> {
        cards.iter().map(|c| c.text.as_str()).collect()
    }

    fn planned(uuid: &str, active: bool) -> Task {
        let mut t = task(uuid, 1.0, active);
        t.tags = vec![crate::task::PLANNED_TAG.into()];
        t
    }

    #[test]
    fn no_tasks_no_cards() {
        assert!(cards(&[], &[]).is_empty());
    }

    #[test]
    fn active_first_then_most_urgent() {
        let got = cards(
            &[task("low", 1.0, false), task("started", 2.0, true), task("high", 9.0, false)],
            &[],
        );
        assert_eq!(texts(&got), vec!["started", "high", "low"]);
        assert_eq!(got[0].uuid.as_deref(), Some("started"), "a card knows its task");
        assert_eq!(got[0].status, Status::Active);
        assert_eq!(got[1].status, Status::Pending);
    }

    #[test]
    fn blocked_tasks_are_marked() {
        let got = cards(&[task("waits", 1.0, false)], &["waits".into()]);
        assert_eq!(got[0].status, Status::Blocked);
        assert_eq!(got[0].icon(), "\u{f023}");
    }

    #[test]
    fn started_outranks_blocked() {
        let got = cards(&[task("both", 1.0, true)], &["both".into()]);
        assert_eq!(got[0].status, Status::Active);
    }

    #[test]
    fn past_the_cap_the_rest_fold_into_one_card() {
        let many: Vec<Task> = (0..11).map(|i| task(&format!("t{i}"), i as f64, false)).collect();
        let got = cap(&cards(&many, &[]), CAP);
        assert_eq!(got.len(), CAP + 1);
        assert_eq!(got[0].text, "t10", "the cap keeps the most urgent");
        let last = got.last().unwrap();
        assert_eq!(last.status, Status::More);
        assert_eq!(last.text, "+3 more");
        assert_eq!(last.icon(), "");
        assert_eq!(last.uuid, None);
    }

    #[test]
    fn exactly_the_cap_has_no_more_card() {
        let many: Vec<Task> = (0..CAP).map(|i| task(&format!("t{i}"), 1.0, false)).collect();
        let got = cap(&cards(&many, &[]), CAP);
        assert_eq!(got.len(), CAP);
        assert!(got.iter().all(|c| c.status != Status::More));
    }

    #[test]
    fn descriptions_are_one_line() {
        let mut t = task("x", 1.0, false);
        t.description = "two\nlines".into();
        assert_eq!(cards(&[t], &[])[0].text, "two lines");
    }

    #[test]
    fn planned_tasks_get_the_filled_dot() {
        let got = cards(&[planned("p", false)], &[]);
        assert_eq!(got[0].status, Status::Planned);
        assert_eq!(got[0].icon(), "●");
    }

    #[test]
    fn started_outranks_planned() {
        let got = cards(&[planned("p", true)], &[]);
        assert_eq!(got[0].status, Status::Active);
    }

    /// A planned task that is waiting on another still cannot be started, and
    /// the lock is what says so.
    #[test]
    fn blocked_outranks_planned() {
        let got = cards(&[planned("p", false)], &["p".into()]);
        assert_eq!(got[0].status, Status::Blocked);
    }

    #[test]
    fn planned_does_not_change_the_order() {
        let mut low = planned("low", false);
        low.urgency = 1.0;
        let got = cards(&[low, task("high", 9.0, false)], &[]);
        assert_eq!(texts(&got), vec!["high", "low"]);
    }

    #[test]
    fn a_card_knows_its_task_is_planned_whatever_its_status() {
        let got = cards(&[planned("started", true), planned("waits", false), task("plain", 1.0, false)], &["waits".into()]);
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
        assert!(!cap(&cards(&many, &[]), CAP).last().unwrap().planned);
    }

    #[test]
    fn the_tabs_run_all_active_planned_to_refine() {
        let labels: Vec<&str> = Filter::TABS.iter().map(|f| f.label()).collect();
        assert_eq!(labels, vec!["All", "Active", "Planned", "To refine"]);
    }

    #[test]
    fn a_tab_label_carries_its_count() {
        assert_eq!(Filter::Planned.tab_label(3), "Planned 3");
        assert_eq!(Filter::ToRefine.tab_label(0), "To refine 0");
    }

    #[test]
    fn an_empty_tab_says_what_it_has_none_of() {
        assert_eq!(Filter::All.empty_text(), "No tasks");
        assert_eq!(Filter::Active.empty_text(), "No active tasks");
        assert_eq!(Filter::Planned.empty_text(), "No planned tasks");
        assert_eq!(Filter::ToRefine.empty_text(), "No tasks to refine");
    }

    /// A filter, not a status. A started planned task is under Active alone:
    /// Planned is for picking what to start, and it is started. Not under To
    /// refine either, which it is past.
    #[test]
    fn each_tab_picks_its_tasks_in_order() {
        let all = cards(
            &[
                task("plain", 9.0, false),
                planned("started-planned", true),
                task("started", 5.0, true),
                planned("planned", false),
            ],
            &[],
        );
        assert_eq!(texts(&Filter::All.pick(&all)), texts(&all));
        assert_eq!(texts(&Filter::Active.pick(&all)), vec!["started", "started-planned"]);
        assert_eq!(texts(&Filter::Planned.pick(&all)), vec!["planned"]);
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["started", "plain"]);
    }

    #[test]
    fn counts_are_per_tab_in_tab_order() {
        let all = cards(
            &[task("plain", 9.0, false), planned("started-planned", true), planned("planned", false)],
            &[],
        );
        assert_eq!(Filter::counts(&all), [3, 1, 1, 1]);
        assert_eq!(Filter::counts(&[]), [0, 0, 0, 0]);
    }
}
