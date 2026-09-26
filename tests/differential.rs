//! Differential tests: the Rust ports vs the shell pipelines they replace.
//!
//! The unit tests in each module assert what the behaviour *should* be. These
//! assert it matches what the shell scripts *actually did*, which is the thing
//! that has been in daily use and is therefore the real specification.
//!
//! The pipelines below are copied verbatim from the scripts:
//!   tag     -> config/niri/task-lib.sh:39-41   (workspace_tag)
//!   project -> config/niri/open_project_workspace.sh:77
//!
//! If any of these fail, the port has changed behaviour the user relies on.

use std::process::{Command, Stdio};
use std::io::Write;

/// Run a shell pipeline with `input` on stdin and return trimmed stdout.
fn sh(pipeline: &str, input: &str) -> String {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(pipeline)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn bash");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");

    let out = child.wait_with_output().expect("wait bash");
    String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string()
}

/// The inputs worth checking: real workspace names, plus the awkward cases.
fn corpus() -> Vec<&'static str> {
    vec![
        "general",
        "ubuntu-setup",
        "Ubuntu-Setup",
        "ubuntu setup",
        "UBUNTU SETUP",
        "my_project",
        "project2",
        "2026 plans",
        "a   b",
        "a---b",
        "a - b",
        "-hello-",
        "___hello___",
        "  hello  ",
        "a__b",
        "",
        "---",
        "   ",
        "!!!",
        "with.dots",
        "with:colons",
        "trailing-",
        "-leading",
        "MiXeD CaSe-Thing",
        "tabs\tand spaces",
        "under_score-dash mix",
    ]
}

#[test]
fn workspace_tag_matches_the_shell_pipeline() {
    // task-lib.sh:39-41
    let pipeline = r#"tr '[:upper:]' '[:lower:]' | sed -e 's/[^a-z0-9_]\+/_/g' -e 's/^_\+//' -e 's/_\+$//'"#;

    let mut mismatches = Vec::new();
    for input in corpus() {
        let shell = sh(pipeline, input);
        let rust = niri_tasks::tag::workspace_tag(input);
        if shell != rust {
            mismatches.push(format!("{input:?}: shell={shell:?} rust={rust:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "workspace_tag diverged from task-lib.sh:\n  {}",
        mismatches.join("\n  ")
    );
}

#[test]
fn project_normalize_matches_the_shell_pipeline() {
    // open_project_workspace.sh:70 (trim) then :77 (whitespace runs -> dash)
    let pipeline = r#"sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' | sed 's/[[:space:]]\+/-/g'"#;

    let mut mismatches = Vec::new();
    for input in corpus() {
        let shell = sh(pipeline, input);
        let rust = niri_tasks::project::normalize(input.trim());
        if shell != rust {
            mismatches.push(format!("{input:?}: shell={shell:?} rust={rust:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "project::normalize diverged from open_project_workspace.sh:\n  {}",
        mismatches.join("\n  ")
    );
}

#[test]
fn description_trim_matches_the_shell_pipeline() {
    // task-add-text.sh:22, task-list.sh:136
    let pipeline = r#"sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'"#;

    let mut mismatches = Vec::new();
    for input in corpus() {
        let shell = sh(pipeline, input);
        let rust = niri_tasks::text::trim(input);
        if shell != rust {
            mismatches.push(format!("{input:?}: shell={shell:?} rust={rust:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "text::trim diverged:\n  {}",
        mismatches.join("\n  ")
    );
}

/// The word-splitting that makes `due:friday` reach taskwarrior as its own
/// argument. The shell did this with `set -f` and an unquoted expansion; this
/// checks the resulting argv is identical.
#[test]
fn add_args_matches_shell_word_splitting() {
    let cases = [
        "ship the release due:friday priority:H",
        "  leading and trailing  ",
        "single",
        "tabs\tbetween\twords",
        "multiple   spaces",
    ];

    for input in cases {
        // `printf '%s\n' $VAR` unquoted under `set -f` == the shell's splitting.
        let pipeline = r#"read -r -d '' VAR || true; set -f; printf '%s\n' $VAR"#;
        let shell = sh(pipeline, input);
        let shell_args: Vec<&str> = if shell.is_empty() {
            vec![]
        } else {
            shell.split('\n').collect()
        };

        let rust = niri_tasks::text::add_args(input);
        assert_eq!(
            shell_args, rust,
            "add_args diverged for {input:?}"
        );
    }
}
