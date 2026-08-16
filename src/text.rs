//! Text handling for task descriptions and notes.
//!
//! Two rules here look similar but are deliberately opposite, and getting them
//! backwards is the most likely way to regress this port:
//!
//! * **Adding** a task word-splits the description, so taskwarrior's own
//!   attribute syntax works — "ship the release due:friday priority:H" sets a
//!   due date and a priority rather than becoming part of the description.
//!   (`task-add-text.sh:30`, which used `set -f` + unquoted expansion.)
//! * **Editing** and **annotating** do *not* split. The text is passed as one
//!   argument after `--`, so a typed `due:` stays literal text.
//!   (`task-edit-text.sh`, `task-annotate-text.sh`.)
//!
//! In Rust the split is explicit rather than a shell side effect, which also
//! retires the `set -f` glob guard the shell version needed: arguments never
//! hit a glob expander here.

/// Trim leading and trailing whitespace. Matches the
/// `sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'` the scripts all ran.
pub fn trim(s: &str) -> &str {
    s.trim()
}

/// Collapse any run of whitespace to a single space, then trim.
///
/// Taskwarrior descriptions are single-line, and the box is multi-line, so a
/// pasted newline has to go somewhere. `TaskBoxModal.qml` did this with
/// `.replace(/\s+/g, " ").trim()` before submitting.
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

/// Split the add box's notes area into one annotation per line.
///
/// The third rule, and the only place in the box where a newline carries
/// meaning: a description collapses its newlines away, because taskwarrior
/// descriptions are one line, but notes are a list and the line breaks are what
/// say where one note ends and the next begins. Blank lines are dropped rather
/// than becoming empty annotations, and each line is collapsed and trimmed on
/// its own.
pub fn note_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(collapse_whitespace)
        .filter(|line| !line.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("\t hello \n"), "hello");
        assert_eq!(trim(""), "");
    }

    #[test]
    fn collapses_newlines_for_single_line_descriptions() {
        assert_eq!(collapse_whitespace("a\nb"), "a b");
        assert_eq!(collapse_whitespace("a  \n\t  b"), "a b");
        assert_eq!(collapse_whitespace("  padded  "), "padded");
        assert_eq!(collapse_whitespace(""), "");
        assert_eq!(collapse_whitespace("   "), "");
    }

    /// The behaviour `set -f` + unquoted expansion bought in the shell version:
    /// attributes reach `task` as their own arguments.
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

    /// The shell version needed `set -f` so a description containing `*` was not
    /// expanded against the current directory. Passing argv directly means there
    /// is no glob stage at all — assert the character survives untouched.
    #[test]
    fn add_does_not_glob() {
        assert_eq!(add_args("clean up * files"), vec!["clean", "up", "*", "files"]);
    }

    /// One line in, one annotation out — and the line breaks survive, which is
    /// the opposite of what a description does with them.
    #[test]
    fn notes_split_one_per_line() {
        assert_eq!(
            note_lines("first note\nsecond note"),
            vec!["first note".to_string(), "second note".to_string()]
        );
    }

    /// Blank lines are the normal shape of typed input — a trailing newline,
    /// or a gap left between two notes — and none of them is a note.
    #[test]
    fn notes_drop_blank_lines() {
        assert_eq!(
            note_lines("first\n\n   \nsecond\n"),
            vec!["first".to_string(), "second".to_string()]
        );
        assert!(note_lines("").is_empty());
        assert!(note_lines("\n  \n").is_empty());
    }

    /// Each line is collapsed on its own, so a note keeps its own words tidy
    /// without being merged into its neighbour.
    #[test]
    fn notes_collapse_within_a_line_but_not_across_lines() {
        assert_eq!(
            note_lines("  spaced   out  \nsecond"),
            vec!["spaced out".to_string(), "second".to_string()]
        );
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
