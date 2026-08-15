//! Building the rows the task picker displays.
//!
//! Split out of the command handler so it can be tested against fixtures. The
//! live task database is too thin to exercise this — it has no annotated tasks
//! at all, so the `¶` marker would never be checked by a real-data diff.
//!
//! The row format is `<uuid>\t<display>`, shown with `--with-nth=2` and
//! `--accept-nth=1`: the description is what you read and filter, the uuid is
//! what comes back. Ids would be shorter but taskwarrior renumbers them as
//! tasks complete, so a stale id can point at the wrong task.

use crate::task::Task;

/// The row that opens the add box. A real row rather than relying on fuzzel
/// echoing unmatched text — that echo is what lets the project picker create
/// folders, but it is not documented to survive `--accept-nth`.
pub const ADD_SENTINEL: &str = "__add__";
pub const ADD_LABEL: &str = "＋ Add a task…";

/// Build the picker rows for a set of tasks, most urgent first.
///
/// The leading marker flags the active task; the trailing `¶` flags a task
/// carrying notes, which are otherwise invisible here — the row shows a
/// description, and an annotation is not one.
pub fn build(tasks: &[Task]) -> Vec<(String, String)> {
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| {
        b.urgency
            .partial_cmp(&a.urgency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut rows: Vec<(String, String)> = sorted
        .into_iter()
        .map(|t| {
            let mark = if t.is_active() { "▶ " } else { "  " };
            let notes = if t.has_notes() { " ¶" } else { "" };
            (t.uuid.clone(), format!("{mark}{}{notes}", t.description))
        })
        .collect();

    // Always present, so a workspace with no tasks still shows a picker that
    // visibly does something rather than looking like a dead shortcut.
    rows.push((ADD_SENTINEL.to_string(), ADD_LABEL.to_string()));
    rows
}

/// Widest display string, in characters — the input to the width clamp.
pub fn longest(rows: &[(String, String)]) -> usize {
    rows.iter().map(|(_, d)| d.chars().count()).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Annotation, Task};

    fn task(uuid: &str, desc: &str, urgency: f64, active: bool, notes: usize) -> Task {
        Task {
            uuid: uuid.into(),
            description: desc.into(),
            urgency,
            start: active.then(|| "20260815T080000Z".to_string()),
            annotations: (0..notes)
                .map(|i| Annotation {
                    entry: "20260815T080000Z".into(),
                    description: format!("note {i}"),
                })
                .collect(),
        }
    }

    #[test]
    fn empty_list_still_offers_the_add_row() {
        let rows = build(&[]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, ADD_SENTINEL);
    }

    #[test]
    fn sorts_most_urgent_first() {
        let rows = build(&[
            task("low", "low", 1.0, false, 0),
            task("high", "high", 9.0, false, 0),
            task("mid", "mid", 5.0, false, 0),
        ]);
        let uuids: Vec<&str> = rows.iter().map(|(u, _)| u.as_str()).collect();
        assert_eq!(uuids, vec!["high", "mid", "low", ADD_SENTINEL]);
    }

    #[test]
    fn active_task_gets_the_marker_and_others_get_padding() {
        let rows = build(&[
            task("a", "started", 5.0, true, 0),
            task("b", "idle", 1.0, false, 0),
        ]);
        assert_eq!(rows[0].1, "▶ started");
        assert_eq!(rows[1].1, "  idle", "alignment padding must match the marker width");
    }

    #[test]
    fn annotated_task_gets_the_pilcrow() {
        let rows = build(&[task("a", "has notes", 1.0, false, 2)]);
        assert_eq!(rows[0].1, "  has notes ¶");
    }

    #[test]
    fn active_and_annotated_together() {
        let rows = build(&[task("a", "both", 1.0, true, 1)]);
        assert_eq!(rows[0].1, "▶ both ¶");
    }

    /// The add row must be last regardless of urgency, so its position is
    /// predictable rather than depending on the task set.
    #[test]
    fn add_row_is_always_last() {
        let rows = build(&[task("a", "very urgent", 999.0, false, 0)]);
        assert_eq!(rows.last().unwrap().0, ADD_SENTINEL);
    }

    #[test]
    fn longest_counts_characters_not_bytes() {
        // The marker and pilcrow are multi-byte; a byte count would over-size
        // the picker and push it into the screen edge.
        let rows = build(&[task("a", "abc", 1.0, true, 1)]);
        assert_eq!(rows[0].1, "▶ abc ¶");
        assert_eq!(rows[0].1.chars().count(), 7);
        assert!(rows[0].1.len() > 7, "fixture should be multi-byte");
        assert_eq!(longest(&rows), ADD_LABEL.chars().count().max(7));
    }
}
