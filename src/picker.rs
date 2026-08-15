//! fuzzel, in dmenu mode.
//!
//! Kept as a subprocess rather than reimplemented: fuzzel is a well-tested
//! fuzzy picker and drawing our own would buy nothing.
//!
//! Sizing rules are ported from the scripts and matter more than they look.
//! `--width` is in characters, but at this font each costs roughly 30px, so the
//! old cap of 90 asked for ~2700px on a 1920px screen; fuzzel clamped that to
//! the display and the popup lost its margins and rounded corners to the screen
//! edge. The caps below keep the widest case comfortably inside 1080p.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The stripped-down fuzzel theme shared by the project and task pickers.
pub fn picker_config() -> Option<PathBuf> {
    let path = dirs_config()?.join("fuzzel/picker.ini");
    path.is_file().then_some(path)
}

fn dirs_config() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

/// Clamp a line count to fuzzel's usable range: as many rows as entries, up to
/// 12, after which the list scrolls.
pub fn clamp_lines(count: usize) -> usize {
    count.min(12)
}

/// Width for the task picker: longest entry + 6, clamped to 40..=52.
pub fn clamp_task_width(longest: usize) -> usize {
    (longest + 6).clamp(40, 52)
}

/// Width for the project picker: longest entry + 4, minimum 20, no upper clamp
/// (folder names are short).
pub fn clamp_project_width(longest: usize) -> usize {
    (longest + 4).max(20)
}

pub struct Picker {
    args: Vec<String>,
}

impl Picker {
    pub fn new() -> Self {
        let mut args = vec!["--dmenu".to_string()];
        if let Some(cfg) = picker_config() {
            args.push(format!("--config={}", cfg.display()));
        }
        Self { args }
    }

    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    pub fn lines(self, n: usize) -> Self {
        self.arg(format!("--lines={n}"))
    }

    pub fn width(self, n: usize) -> Self {
        self.arg(format!("--width={n}"))
    }

    pub fn prompt(self, p: &str) -> Self {
        self.arg(format!("--prompt={p}"))
    }

    /// Show `entries` and return what was accepted.
    ///
    /// Returns `None` when the picker was cancelled. Note fuzzel echoes typed
    /// text verbatim when it matches no entry — that is what lets the project
    /// picker create folders, and why the task list validates the result
    /// against its own rows before treating it as a uuid.
    pub fn run(self, entries: &[String]) -> Result<Option<String>> {
        let mut child = Command::new("fuzzel")
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("could not run `fuzzel` — is it installed?")?;

        {
            let stdin = child.stdin.as_mut().context("fuzzel stdin")?;
            // An empty list must send nothing at all: writing one blank line
            // would show up as an empty row.
            for e in entries {
                writeln!(stdin, "{e}")?;
            }
        }

        let out = child.wait_with_output().context("fuzzel failed")?;
        let selected = String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string();
        Ok((!selected.is_empty()).then_some(selected))
    }
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_cap_at_twelve() {
        assert_eq!(clamp_lines(0), 0);
        assert_eq!(clamp_lines(5), 5);
        assert_eq!(clamp_lines(12), 12);
        assert_eq!(clamp_lines(100), 12);
    }

    /// The 52 cap is what keeps the popup off the screen edge at 1080p.
    #[test]
    fn task_width_is_clamped_both_ends() {
        assert_eq!(clamp_task_width(0), 40, "short lists still get a usable box");
        assert_eq!(clamp_task_width(30), 36.max(40));
        assert_eq!(clamp_task_width(40), 46);
        assert_eq!(clamp_task_width(100), 52, "long descriptions elide, not widen");
    }

    #[test]
    fn project_width_has_a_floor_but_no_cap() {
        assert_eq!(clamp_project_width(0), 20);
        assert_eq!(clamp_project_width(30), 34);
    }
}
