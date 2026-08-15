//! Workspace name -> taskwarrior tag.
//!
//! A taskwarrior tag is a single bare word: dashes read as operators inside
//! filters and spaces split the argument, so neither survives `task +<tag>`.
//! Everything else folds to underscores and lowercase, which also means
//! "Ubuntu-Setup", "ubuntu setup" and "ubuntu-setup" all resolve to one tag
//! rather than three tags holding a third of the project's tasks each.
//!
//! Ported from `task-lib.sh:workspace_tag`, which was:
//!     tr '[:upper:]' '[:lower:]'
//!     | sed -e 's/[^a-z0-9_]\+/_/g' -e 's/^_\+//' -e 's/_\+$//'
//!
//! Note the order: lowercasing happens *first*, so an uppercase letter becomes
//! its lowercase self rather than an underscore.

/// Derive the taskwarrior tag for a workspace name. Returns an empty string
/// when the name has no usable characters — callers treat that as "no tag".
pub fn workspace_tag(name: &str) -> String {
    let lowered = name.to_lowercase();

    // Runs of disallowed characters collapse to a single underscore.
    let mut out = String::with_capacity(lowered.len());
    let mut in_run = false;
    for ch in lowered.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' {
            out.push(ch);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }

    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_rather_than_replacing_uppercase() {
        assert_eq!(workspace_tag("Ubuntu"), "ubuntu");
        assert_eq!(workspace_tag("UBUNTU"), "ubuntu");
    }

    /// The whole point of the rule: three spellings, one tag.
    #[test]
    fn spellings_converge_on_one_tag() {
        assert_eq!(workspace_tag("Ubuntu-Setup"), "ubuntu_setup");
        assert_eq!(workspace_tag("ubuntu setup"), "ubuntu_setup");
        assert_eq!(workspace_tag("ubuntu-setup"), "ubuntu_setup");
    }

    #[test]
    fn runs_of_disallowed_chars_collapse_to_one_underscore() {
        assert_eq!(workspace_tag("a   b"), "a_b");
        assert_eq!(workspace_tag("a---b"), "a_b");
        assert_eq!(workspace_tag("a - b"), "a_b");
    }

    #[test]
    fn leading_and_trailing_underscores_are_stripped() {
        assert_eq!(workspace_tag("-hello-"), "hello");
        assert_eq!(workspace_tag("___hello___"), "hello");
        assert_eq!(workspace_tag("  hello  "), "hello");
    }

    #[test]
    fn existing_underscores_survive() {
        assert_eq!(workspace_tag("my_project"), "my_project");
        assert_eq!(workspace_tag("a__b"), "a__b");
    }

    #[test]
    fn digits_survive() {
        assert_eq!(workspace_tag("project2"), "project2");
        assert_eq!(workspace_tag("2026 plans"), "2026_plans");
    }

    /// An unusable name yields an empty tag; callers must refuse to write a
    /// task rather than guess, since an untagged task is invisible to every list.
    #[test]
    fn unusable_names_yield_empty() {
        assert_eq!(workspace_tag(""), "");
        assert_eq!(workspace_tag("---"), "");
        assert_eq!(workspace_tag("   "), "");
        assert_eq!(workspace_tag("!!!"), "");
    }

    #[test]
    fn non_ascii_folds_to_underscore() {
        // The shell version's [^a-z0-9_] is ASCII-only; keep that behaviour so
        // tags stay typeable at a shell prompt. The trailing underscore the
        // substitution leaves is then stripped by the trim.
        assert_eq!(workspace_tag("café"), "caf");
        assert_eq!(workspace_tag("día 2"), "d_a_2");
        assert_eq!(workspace_tag("日本"), "");
    }
}
