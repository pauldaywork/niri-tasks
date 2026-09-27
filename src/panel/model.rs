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
    /// The "+N more" card standing in for everything past the cap.
    More,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub status: Status,
    pub text: String,
}

impl Card {
    /// The status icon, which is what the peek shows first.
    pub fn icon(&self) -> &'static str {
        match self.status {
            Status::Active => "▶",
            Status::Pending => "○",
            Status::Blocked => "🔒",
            // The count is the text, so the peek reads "+3".
            Status::More => "",
        }
    }
}

/// The task cards for one workspace tag: the active task first, then the rest
/// most urgent first, capped at `cap` plus a "+N more" card.
///
/// Active first even when something else is more urgent: it is the work in
/// progress, and the one card worth reading without hovering. Being started
/// outranks being blocked, since you started it anyway.
pub fn cards(tasks: &[Task], blocked: &[String], cap: usize) -> Vec<Card> {
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| {
        b.is_active().cmp(&a.is_active()).then(
            b.urgency
                .partial_cmp(&a.urgency)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    let mut cards: Vec<Card> = sorted
        .iter()
        .take(cap)
        .map(|t| Card {
            status: if t.is_active() {
                Status::Active
            } else if blocked.contains(&t.uuid) {
                Status::Blocked
            } else {
                Status::Pending
            },
            text: crate::text::collapse_whitespace(&t.description),
        })
        .collect();

    if sorted.len() > cap {
        cards.push(Card {
            status: Status::More,
            text: format!("+{} more", sorted.len() - cap),
        });
    }
    cards
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
        }
    }

    fn texts(cards: &[Card]) -> Vec<&str> {
        cards.iter().map(|c| c.text.as_str()).collect()
    }

    #[test]
    fn no_tasks_no_cards() {
        assert!(cards(&[], &[], CAP).is_empty());
    }

    #[test]
    fn active_first_then_most_urgent() {
        let got = cards(
            &[task("low", 1.0, false), task("started", 2.0, true), task("high", 9.0, false)],
            &[],
            CAP,
        );
        assert_eq!(texts(&got), vec!["started", "high", "low"]);
        assert_eq!(got[0].status, Status::Active);
        assert_eq!(got[1].status, Status::Pending);
    }

    #[test]
    fn blocked_tasks_are_marked() {
        let got = cards(&[task("waits", 1.0, false)], &["waits".into()], CAP);
        assert_eq!(got[0].status, Status::Blocked);
        assert_eq!(got[0].icon(), "🔒");
    }

    #[test]
    fn started_outranks_blocked() {
        let got = cards(&[task("both", 1.0, true)], &["both".into()], CAP);
        assert_eq!(got[0].status, Status::Active);
    }

    #[test]
    fn past_the_cap_the_rest_fold_into_one_card() {
        let many: Vec<Task> = (0..11).map(|i| task(&format!("t{i}"), i as f64, false)).collect();
        let got = cards(&many, &[], CAP);
        assert_eq!(got.len(), CAP + 1);
        assert_eq!(got[0].text, "t10", "the cap keeps the most urgent");
        let last = got.last().unwrap();
        assert_eq!(last.status, Status::More);
        assert_eq!(last.text, "+3 more");
        assert_eq!(last.icon(), "");
    }

    #[test]
    fn exactly_the_cap_has_no_more_card() {
        let many: Vec<Task> = (0..CAP).map(|i| task(&format!("t{i}"), 1.0, false)).collect();
        let got = cards(&many, &[], CAP);
        assert_eq!(got.len(), CAP);
        assert!(got.iter().all(|c| c.status != Status::More));
    }

    #[test]
    fn descriptions_are_one_line() {
        let mut t = task("x", 1.0, false);
        t.description = "two\nlines".into();
        assert_eq!(cards(&[t], &[], CAP)[0].text, "two lines");
    }
}
