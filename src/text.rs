//! Text handling for task descriptions and notes.
//!
//! Two rules here look similar but are deliberately opposite, and getting them
//! backwards is the most likely way to regress this port:
//!
//! * **Adding** a task word-splits the description, so taskwarrior's own
//!   attribute syntax works — "ship the release due:friday priority:H" sets a
//!   due date and a priority rather than becoming part of the description.
//! * **Editing** and **annotating** do *not* split. The text is passed as one
//!   argument after `--`, so a typed `due:` stays literal text.
//!
//! The split is explicit rather than a shell side effect, so arguments never
//! hit a glob expander.

/// Collapse any run of whitespace to a single space, then trim.
///
/// Taskwarrior descriptions are single-line, and the box is multi-line, so a
/// pasted newline has to go somewhere.
pub fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Split a description into the arguments `task add` should receive.
///
/// This is what makes `due:friday` work. Empty input yields an empty vec, which
/// callers treat as "nothing to do".
pub fn add_args(description: &str) -> Vec<&str> {
    description.split_whitespace().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_newlines_for_single_line_descriptions() {
        assert_eq!(collapse_whitespace("a\nb"), "a b");
        assert_eq!(collapse_whitespace("a  \n\t  b"), "a b");
        assert_eq!(collapse_whitespace("  padded  "), "padded");
        assert_eq!(collapse_whitespace(""), "");
        assert_eq!(collapse_whitespace("   "), "");
    }

    /// Attributes reach `task` as their own arguments.
    #[test]
    fn add_splits_so_taskwarrior_attributes_parse() {
        assert_eq!(
            add_args("ship the release due:friday priority:H"),
            vec!["ship", "the", "release", "due:friday", "priority:H"]
        );
    }

    #[test]
    fn add_ignores_empty_and_whitespace_only() {
        assert!(add_args("").is_empty());
        assert!(add_args("   ").is_empty());
    }

    /// A description containing `*` is never expanded against the current
    /// directory: argv is passed directly, so there is no glob stage at all.
    #[test]
    fn add_does_not_glob() {
        assert_eq!(add_args("clean up * files"), vec!["clean", "up", "*", "files"]);
    }

    /// Edit and annotate must NOT split — the counterpart to the test above.
    /// There is no `edit_args`; the text goes through as one argument, and this
    /// test exists to pin that intent so nobody "fixes" it later.
    #[test]
    fn edit_and_annotate_keep_text_whole() {
        let typed = "remember due: is not a date here";
        assert_eq!(collapse_whitespace(typed), typed);
    }
}
