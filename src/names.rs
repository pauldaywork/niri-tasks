//! The names a task carries into herdr: its agents' names, and the label its
//! tabs show. Refine names its agent `task-<uuid8>` and Start working names
//! its `work-<uuid8>`, so nothing is stored on the task — the panel finds a
//! task's agent by asking herdr for those names. They are spelled here once,
//! so the pane link, Refine, Start working and the panel's lookup cannot
//! drift apart.

use crate::text;

/// The first eight characters of a uuid, lowercased — enough to tell one
/// task's branch and agent from another's.
pub fn uuid8(uuid: &str) -> String {
    uuid.chars().take(8).collect::<String>().to_ascii_lowercase()
}

/// The herdr agent name for a task's refine session. One name per task is
/// what lets a second Refine find the first instead of opening another.
pub fn refine_agent(uuid: &str) -> String {
    format!("task-{}", uuid8(uuid))
}

/// The herdr agent name for a task's working Claude. `work-`, not Refine's
/// `task-`, so a refine still open on the task is never mistaken for it.
pub fn work_agent(uuid: &str) -> String {
    format!("work-{}", uuid8(uuid))
}

/// Both of `uuid`'s agent names, its working Claude (`work-`) first and its
/// refine (`task-`) second: the working Claude is the one you most likely
/// want back, and a refine still open beside it is second.
pub fn both_agents(uuid: &str) -> [String; 2] {
    [work_agent(uuid), refine_agent(uuid)]
}

/// Whether `name` is a `task-` or `work-` agent: one named by Refine or Start
/// working, or linked already, and so named for a task.
pub fn is_task_agent(name: &str) -> bool {
    name.starts_with("task-") || name.starts_with("work-")
}

/// Longest description, in characters, a tab label carries before eliding.
const LABEL_MAX: usize = 30;

/// A description cut to fit a tab label, by characters, with an ellipsis. Its
/// whitespace is collapsed first, so a two-line description reads as one.
pub fn elide(description: &str) -> String {
    let d = text::collapse_whitespace(description);
    if d.chars().count() > LABEL_MAX {
        let cut: String = d.chars().take(LABEL_MAX - 1).collect();
        format!("{cut}…")
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid8_is_the_first_eight_lowercased() {
        assert_eq!(uuid8("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "7cd9fd3a");
        assert_eq!(uuid8("abc"), "abc");
    }

    /// herdr names must match `[a-z][a-z0-9_-]{0,31}`; a uuid's first eight
    /// characters are hex, and enough to tell one task's session from another.
    #[test]
    fn the_agents_are_named_after_their_task() {
        assert_eq!(refine_agent("00DEEEE1-3cbd-465d-8c85-c4c4d643b1d0"), "task-00deeee1");
        assert_eq!(work_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "work-7cd9fd3a");
    }

    #[test]
    fn both_agents_prefer_the_working_claude() {
        assert_eq!(both_agents("7CD9FD3A-d27b"), ["work-7cd9fd3a", "task-7cd9fd3a"]);
    }

    #[test]
    fn is_task_agent_knows_both_prefixes() {
        assert!(is_task_agent("task-x"));
        assert!(is_task_agent("work-x"));
        assert!(!is_task_agent("reviewer"));
        assert!(!is_task_agent(""));
    }

    /// A tab label shares herdr's sidebar with every other tab; a long
    /// description is cut, by characters, with an ellipsis.
    #[test]
    fn long_descriptions_are_elided() {
        let long = elide(&"é".repeat(40));
        assert_eq!(long.chars().count(), LABEL_MAX);
        assert_eq!(long, format!("{}…", "é".repeat(29)));
        assert_eq!(elide("two\n lines"), "two lines");
        let exact = "a".repeat(LABEL_MAX);
        assert_eq!(elide(&exact), exact);
    }
}
