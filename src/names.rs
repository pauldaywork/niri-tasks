//! The names a task carries into herdr: its agents' names, the label its tabs
//! show, and what its Claude's pane is called.
//! Refine names its agent `task-<uuid8>`, Generate report its `report-<uuid8>`
//! and Start working its `work-<uuid8>`, so nothing is stored on the task — the
//! panel finds a task's agent by asking herdr for those names. They are spelled
//! here once, so the pane link, Refine, Report, Start working and the panel's
//! lookup cannot drift apart.

use crate::text;

/// The first eight characters of a uuid, lowercased — enough to tell one
/// task's branch and agent from another's.
pub fn uuid8(uuid: &str) -> String {
    uuid.chars().take(8).collect::<String>().to_ascii_lowercase()
}

/// The herdr agent name for a task's refine session, Refine's and Grill me's.
/// One name per task is what lets a second Refine find the first instead of
/// opening another.
pub fn refine_agent(uuid: &str) -> String {
    format!("task-{}", uuid8(uuid))
}

/// The herdr agent name for a task's working Claude. `work-`, not Refine's
/// `task-`, so a refine still open on the task is never mistaken for it.
pub fn work_agent(uuid: &str) -> String {
    format!("work-{}", uuid8(uuid))
}

/// The herdr agent name for a task's report session. Its own `report-`, not
/// Refine's `task-`, so Generate report never lands in a refine tab still
/// open on the task — that tab was started without the report option, and
/// may be mid-interview — while a second press finds the first report.
pub fn report_agent(uuid: &str) -> String {
    format!("report-{}", uuid8(uuid))
}

/// The herdr agent name for a task's Explain branch session. Its own
/// `branch-`, so Explain branch never lands in a plan report's tab, and a
/// second press finds the first.
pub fn branch_agent(uuid: &str) -> String {
    format!("branch-{}", uuid8(uuid))
}

/// Both of `uuid`'s agent names, its working Claude (`work-`) first and its
/// refine (`task-`) second: the working Claude is the one you most likely
/// want back, and a refine still open beside it is second.
pub fn both_agents(uuid: &str) -> [String; 2] {
    [work_agent(uuid), refine_agent(uuid)]
}

/// Whether `name` is a `task-`, `work-`, `report-` or `branch-` agent: one
/// named by Refine, Start working, Generate report or Explain branch, or
/// linked already, and so named for a task.
pub fn is_task_agent(name: &str) -> bool {
    ["task-", "work-", "report-", "branch-"].iter().any(|p| name.starts_with(p))
}

/// Whether `s` is a uuid8 as [`uuid8`] spells one: eight lowercase hex
/// digits.
fn is_uuid8(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The uuid8 of the task a `task-`, `work-` or `report-<uuid8>` agent is
/// named for, or None for any other name: how the focused pane's task is
/// found from its Claude.
pub fn uuid8_of_agent(name: &str) -> Option<String> {
    let rest = ["task-", "work-", "report-"].iter().find_map(|p| name.strip_prefix(p))?;
    is_uuid8(rest).then(|| rest.to_string())
}

/// The uuid8 of the task a `task/<slug>-<uuid8>` branch is for, as
/// `work::branch_name` spells it, or None for any other branch: how a
/// shell's pane is found to be on a task.
pub fn uuid8_of_branch(branch: &str) -> Option<String> {
    let (slug, uuid8) = branch.strip_prefix("task/")?.rsplit_once('-')?;
    (!slug.is_empty() && is_uuid8(uuid8)).then(|| uuid8.to_string())
}

/// Longest description, in characters, a tab label carries before eliding.
const LABEL_MAX: usize = 30;

/// A description cut to fit a tab label, by characters, with an ellipsis. Its
/// whitespace is collapsed first, so a two-line description reads as one.
pub fn elide(description: &str) -> String {
    elide_to(description, LABEL_MAX)
}

/// The tab Explain branch runs a branch's tests in, before its report:
/// `Tests: <description>`, cut short as a Claude tab's label is.
pub fn tests_tab_label(description: &str) -> String {
    format!("Tests: {}", elide(description))
}

/// `text` with its whitespace collapsed, cut to `max` characters with an
/// ellipsis when longer.
pub fn elide_to(text: &str, max: usize) -> String {
    let d = text::collapse_whitespace(text);
    if d.chars().count() > max {
        let cut: String = d.chars().take(max - 1).collect();
        format!("{cut}…")
    } else {
        d
    }
}

/// Longest pane label, and token value, herdr keeps (herdr 0.9.3 caps both
/// at 80 characters).
pub const PANE_LABEL_MAX: usize = 80;

/// Which of a task's Claudes a pane holds, as its border label says. Every
/// refine mode is `Refine`: they share one agent, `task-<uuid8>`. A report
/// runs its own, `report-<uuid8>`, and Explain branch its own,
/// `branch-<uuid8>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Refine,
    Report,
    Branch,
    Work,
}

/// What a task's Claude pane is called in herdr: the label drawn in its
/// border (with `ui.pane_borders = "always"`, even on a lone pane), and the
/// display-only tokens a sidebar row can show as `$task_id`, `$task` and
/// `$task_uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneName {
    pub label: String,
    pub tokens: Vec<(String, String)>,
}

impl PaneName {
    /// `Refine #61 · <description>`, `Report #61 · …`, `Branch #61 · …` or `Work #61 · …`, cut to what herdr
    /// keeps. A task with no number, as a finished one has after gc, shows
    /// none rather than `#0`.
    pub fn for_task(role: Role, id: u64, uuid: &str, description: &str) -> PaneName {
        let verb = match role {
            Role::Refine => "Refine",
            Role::Report => "Report",
            Role::Branch => "Branch",
            Role::Work => "Work",
        };
        let head = if id == 0 { verb.to_string() } else { format!("{verb} #{id}") };
        let description = text::collapse_whitespace(description);
        let mut tokens = Vec::new();
        if id != 0 {
            tokens.push(("task_id".to_string(), id.to_string()));
        }
        tokens.push(("task".to_string(), elide_to(&description, PANE_LABEL_MAX)));
        tokens.push(("task_uuid".to_string(), uuid.to_string()));
        PaneName { label: elide_to(&format!("{head} · {description}"), PANE_LABEL_MAX), tokens }
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
        assert_eq!(report_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "report-7cd9fd3a");
    }

    #[test]
    fn both_agents_prefer_the_working_claude() {
        assert_eq!(both_agents("7CD9FD3A-d27b"), ["work-7cd9fd3a", "task-7cd9fd3a"]);
    }

    #[test]
    fn is_task_agent_knows_every_prefix() {
        assert!(is_task_agent("task-x"));
        assert!(is_task_agent("work-x"));
        assert!(is_task_agent("report-x"));
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

    #[test]
    fn a_pane_label_names_the_role_the_task_number_and_the_description() {
        let n = PaneName::for_task(Role::Refine, 61, "7b4f4c5c-185a", "feat: Name the task  on its\nClaude's pane");
        assert_eq!(n.label, "Refine #61 · feat: Name the task on its Claude's pane");
        let n = PaneName::for_task(Role::Work, 61, "7b4f4c5c-185a", "fix the peek");
        assert_eq!(n.label, "Work #61 · fix the peek");
        let n = PaneName::for_task(Role::Report, 61, "7b4f4c5c-185a", "fix the peek");
        assert_eq!(n.label, "Report #61 · fix the peek");
    }

    /// herdr caps a label at 80 characters; cut here, with an ellipsis, it
    /// at least says it was cut.
    #[test]
    fn a_long_pane_label_is_elided_to_what_herdr_keeps() {
        let n = PaneName::for_task(Role::Work, 61, "u", &"word ".repeat(40));
        assert_eq!(n.label.chars().count(), PANE_LABEL_MAX);
        assert!(n.label.ends_with('…'), "{}", n.label);
    }

    /// A finished task loses its number to gc: no `#0`, no task_id.
    #[test]
    fn a_task_with_no_number_has_none_on_its_pane() {
        let n = PaneName::for_task(Role::Work, 0, "u-1", "fix the peek");
        assert_eq!(n.label, "Work · fix the peek");
        assert!(!n.tokens.iter().any(|(k, _)| k == "task_id"), "{:?}", n.tokens);
    }

    #[test]
    fn the_pane_tokens_carry_the_number_the_description_and_the_uuid() {
        let n = PaneName::for_task(Role::Refine, 61, "7b4f4c5c-185a", "fix  the peek");
        let tokens: Vec<(&str, &str)> = n.tokens.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(tokens, [("task_id", "61"), ("task", "fix the peek"), ("task_uuid", "7b4f4c5c-185a")]);
    }

    #[test]
    fn elide_to_cuts_by_characters() {
        assert_eq!(elide_to("abcdef", 4), "abc…");
        assert_eq!(elide_to("abcd", 4), "abcd");
        assert_eq!(elide("x".repeat(31).as_str()).chars().count(), 30, "elide keeps the tab label's 30");
    }

    /// The three agent names a task's Claude can carry give its uuid8;
    /// anything else, or a prefix with no uuid8 after it, gives none.
    #[test]
    fn an_agent_named_for_a_task_gives_its_uuid8() {
        assert_eq!(uuid8_of_agent("work-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent("task-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent("report-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent(&work_agent("1234ABCD-5678")).as_deref(), Some("1234abcd"), "the builder's own");
        for name in ["reviewer", "work-1234abc", "work-1234abcde", "work-1234ABCD", "work-", "xwork-1234abcd"] {
            assert_eq!(uuid8_of_agent(name), None, "{name}");
        }
    }

    /// A `task/<slug>-<uuid8>` branch gives its uuid8; the slug may hold
    /// dashes of its own.
    #[test]
    fn a_task_branch_gives_its_uuid8() {
        assert_eq!(uuid8_of_branch("task/fix-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_branch("task/fix-the-peek-on-1234abcd").as_deref(), Some("1234abcd"));
        for branch in ["main", "task/1234abcd", "task/-1234abcd", "feature/fix-1234abcd", "task/fix-1234abc", "task/fix-1234abcz"] {
            assert_eq!(uuid8_of_branch(branch), None, "{branch}");
        }
    }

    #[test]
    fn a_branch_report_has_its_own_agent_and_label() {
        assert_eq!(branch_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "branch-7cd9fd3a");
        assert!(is_task_agent("branch-7cd9fd3a"));
        let n = PaneName::for_task(Role::Branch, 61, "7b4f4c5c-185a", "fix the peek");
        assert!(n.label.starts_with("Branch #61 · fix the peek"), "{}", n.label);
    }
}
