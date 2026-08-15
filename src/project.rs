//! Project-name normalisation for the `~/Projects` picker.
//!
//! fuzzel echoes typed text verbatim when it matches no entry, which is what
//! turns the picker into a "new project" box. That text becomes a directory
//! name, so it is normalised and then guarded.
//!
//! Ported from `open_project_workspace.sh:70-91`. The re-check after
//! normalisation matters: typing "my project" may well have just become an
//! existing "my-project", in which case the picker should open that rather than
//! fail to create it.

/// What the picker decided to do with the text the user accepted.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    /// Matches a folder that already exists — open it.
    Existing(String),
    /// A new name, already normalised and validated — create it.
    Create(String),
    /// Nothing usable was typed.
    Nothing,
    /// Normalised into something that must not become a directory.
    Rejected(String),
}

/// Runs of whitespace become a single dash, so new folders never carry spaces
/// and stay painless to type at a shell prompt.
pub fn normalize(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Resolve accepted picker text against the list of existing project folders.
pub fn resolve(typed: &str, existing: &[String]) -> Resolved {
    let trimmed = typed.trim();
    if trimmed.is_empty() {
        return Resolved::Nothing;
    }

    // An exact match on what was typed is opened as-is, before any rewriting —
    // a folder that genuinely contains a space is still selectable.
    if existing.iter().any(|p| p == trimmed) {
        return Resolved::Existing(trimmed.to_string());
    }

    let normalized = normalize(trimmed);
    if normalized.is_empty() {
        return Resolved::Nothing;
    }

    // Re-check after normalising: "my project" may already exist as "my-project".
    if existing.iter().any(|p| *p == normalized) {
        return Resolved::Existing(normalized);
    }

    // Guard the cases that would write outside ~/Projects, or make a folder the
    // picker can never show again (it filters dotfiles from its list).
    if normalized.contains('/') {
        return Resolved::Rejected(format!("Project name can't contain '/': {normalized}"));
    }
    if normalized.starts_with('.') {
        return Resolved::Rejected(format!("Project name can't start with '.': {normalized}"));
    }

    Resolved::Create(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn existing() -> Vec<String> {
        ["alpha", "my-project", "with space"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn whitespace_runs_become_a_single_dash() {
        assert_eq!(normalize("my project"), "my-project");
        assert_eq!(normalize("a   b   c"), "a-b-c");
        assert_eq!(normalize("  padded  "), "padded");
    }

    #[test]
    fn exact_match_opens_existing() {
        assert_eq!(
            resolve("alpha", &existing()),
            Resolved::Existing("alpha".into())
        );
    }

    /// A folder that really does contain a space stays selectable — it is
    /// matched before normalisation would rewrite it.
    #[test]
    fn existing_folder_with_a_space_is_matched_before_rewriting() {
        assert_eq!(
            resolve("with space", &existing()),
            Resolved::Existing("with space".into())
        );
    }

    /// The re-check: typing the spaced form of an existing dashed folder opens
    /// it rather than trying to create a duplicate.
    #[test]
    fn normalizing_can_land_on_an_existing_folder() {
        assert_eq!(
            resolve("my project", &existing()),
            Resolved::Existing("my-project".into())
        );
    }

    #[test]
    fn genuinely_new_names_are_created_normalized() {
        assert_eq!(
            resolve("brand new thing", &existing()),
            Resolved::Create("brand-new-thing".into())
        );
    }

    #[test]
    fn empty_input_does_nothing() {
        assert_eq!(resolve("", &existing()), Resolved::Nothing);
        assert_eq!(resolve("   ", &existing()), Resolved::Nothing);
    }

    #[test]
    fn rejects_names_that_would_escape_the_projects_dir() {
        assert!(matches!(
            resolve("../etc", &existing()),
            Resolved::Rejected(_)
        ));
        assert!(matches!(
            resolve("a/b", &existing()),
            Resolved::Rejected(_)
        ));
    }

    #[test]
    fn rejects_dotfiles_the_picker_could_never_show_again() {
        assert!(matches!(
            resolve(".hidden", &existing()),
            Resolved::Rejected(_)
        ));
    }
}
