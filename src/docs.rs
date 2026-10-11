//! A task's documents, as `niritasks task docs` lists them for herdr's
//! task-docs plugin: its refine reports, its own description and notes, and
//! the plans written since it was added. Each is a Markdown file
//! plannotator-tui can open beside the task's Claude, bar a report made
//! before reports had a Markdown twin, which opens in the browser.

use crate::{dirs::Dirs, names, task, task::Task, text};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One document: what the list shows, and the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// Never holds a tab: [`Doc::line`] splits on the first one.
    pub label: String,
    pub path: PathBuf,
}

impl Doc {
    /// One line of `task docs`: the label, a tab, the path.
    pub fn line(&self) -> String {
        format!("{}\t{}", self.label, self.path.display())
    }
}

/// The Markdown twin the refine mod writes beside a report.
pub fn twin(report: &Path) -> PathBuf {
    report.with_extension("md")
}

/// The task's reports, newest first: each one's Markdown twin, or its HTML
/// when it has none. A report whose files are both gone is left out.
pub fn reports(t: &Task) -> Vec<Doc> {
    t.report_notes()
        .into_iter()
        .filter_map(|n| {
            let kind = if n.draft { "Report, earlier draft" } else { "Report" };
            let html = PathBuf::from(&n.path);
            let md = twin(&html);
            if md.is_file() {
                Some(Doc { label: format!("{kind} · {}", n.date), path: md })
            } else if html.is_file() {
                Some(Doc { label: format!("{kind} · {} · opens in the browser", n.date), path: html })
            } else {
                None
            }
        })
        .collect()
}

/// The task's description and notes as Markdown: the description as the
/// heading, then each note, dated, as a bullet.
pub fn notes_markdown(t: &Task) -> String {
    let mut out = format!(
        "# {}\n\nTask `{}`: its description and notes.\n\n## Notes\n\n",
        text::collapse_whitespace(&t.description),
        t.uuid
    );
    if t.annotations.is_empty() {
        out.push_str("No notes.\n");
    }
    for a in &t.annotations {
        out.push_str(&format!("- {}: {}\n", a.date(), text::collapse_whitespace(&a.description)));
    }
    out
}

/// Write [`notes_markdown`] to `<dir>/task-<uuid8>.md`, making `dir`, and
/// answer the file's path.
pub fn write_notes(dir: &Path, t: &Task) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("could not make {}", dir.display()))?;
    let path = dir.join(format!("task-{}.md", names::uuid8(&t.uuid)));
    std::fs::write(&path, notes_markdown(t)).with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

/// The Markdown files directly in `folder` last changed at or after `since`
/// (Unix seconds), each with that time. A folder that is not there has none.
fn plans_in(folder: &Path, kind: &str, since: i64) -> Vec<(i64, Doc)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Some(changed) = path
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
        else {
            continue;
        };
        if changed < since {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        found.push((changed, Doc { label: format!("{kind} · {name}"), path }));
    }
    found
}

/// The plans written since `since`: the project's `docs/superpowers/plans`
/// and `~/.claude/plans`, newest first. A plan for a task cannot predate it,
/// and both folders hold plans for much else.
pub fn plans(dirs: &Dirs, project: &Path, since: i64) -> Vec<Doc> {
    let mut found = plans_in(&project.join("docs/superpowers/plans"), "Plan", since);
    found.extend(plans_in(&dirs.home().join(".claude/plans"), "Claude plan", since));
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
    found.into_iter().map(|(_, doc)| doc).collect()
}

/// Everything `task docs` lists, in order: the reports, then the notes file
/// when there is one, then the plans since the task was added. A task whose
/// `entry` does not parse counts every plan.
pub fn list(t: &Task, dirs: &Dirs, project: &Path, notes: Option<&Path>) -> Vec<Doc> {
    let mut docs = reports(t);
    if let Some(path) = notes {
        docs.push(Doc { label: "Task · description and notes".into(), path: path.to_path_buf() });
    }
    docs.extend(plans(dirs, project, task::stamp_secs(&t.entry).unwrap_or(0)));
    docs
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// A scratch folder of this test's own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-docs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A file holding `text`, last changed at `secs` (Unix seconds).
    fn file_at(path: &Path, text: &str, secs: u64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
    }

    fn task(json: &str) -> Task {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_reports_twin_is_beside_it() {
        assert_eq!(twin(Path::new("/r/refine-a-1.html")), PathBuf::from("/r/refine-a-1.md"));
    }

    /// The twin when it is there, the HTML when only that is (a report made
    /// before twins), and nothing for a report whose files are both gone.
    #[test]
    fn reports_prefer_the_twin_and_skip_what_is_gone() {
        let dir = scratch("reports");
        file_at(&dir.join("new.html"), "<html>", 1);
        file_at(&dir.join("new.md"), "# new", 1);
        file_at(&dir.join("old.html"), "<html>", 1);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","annotations":[
                {{"entry":"20261005T010000Z","description":"Report: {d}/gone.html"}},
                {{"entry":"20261006T010000Z","description":"Report (earlier draft): {d}/old.html"}},
                {{"entry":"20261007T010000Z","description":"Report: {d}/new.html"}}
            ]}}"#,
            d = dir.display()
        ));
        assert_eq!(
            reports(&t),
            vec![
                Doc { label: "Report · 2026-10-07".into(), path: dir.join("new.md") },
                Doc {
                    label: "Report, earlier draft · 2026-10-06 · opens in the browser".into(),
                    path: dir.join("old.html"),
                },
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_notes_are_the_description_then_one_bullet_each() {
        let t = task(
            r#"{"uuid":"35735579-9f97","description":"feat: Annotate docs","annotations":[
                {"entry":"20261011T064839Z","description":"Goal: one key"},
                {"entry":"20261011T064840Z","description":"Context:\n  herdr 0.9.3"}
            ]}"#,
        );
        assert_eq!(
            notes_markdown(&t),
            "# feat: Annotate docs\n\nTask `35735579-9f97`: its description and notes.\n\n## Notes\n\n\
             - 2026-10-11: Goal: one key\n- 2026-10-11: Context: herdr 0.9.3\n"
        );
        let bare = task(r#"{"uuid":"u","description":"d"}"#);
        assert!(notes_markdown(&bare).ends_with("## Notes\n\nNo notes.\n"));
    }

    #[test]
    fn the_notes_are_written_as_task_uuid8_md() {
        let dir = scratch("notes").join("state");
        let t = task(r#"{"uuid":"35735579-9F97","description":"d"}"#);
        let path = write_notes(&dir, &t).unwrap();
        assert_eq!(path, dir.join("task-35735579.md"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), notes_markdown(&t));
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    /// Only Markdown changed since the task was added, from the project's
    /// plans and Claude's, newest first.
    #[test]
    fn plans_since_the_task_newest_first() {
        let home = scratch("plans");
        let project = home.join("Projects/alpha");
        let ours = project.join("docs/superpowers/plans");
        let claudes = home.join(".claude/plans");
        file_at(&ours.join("before.md"), "x", 99);
        file_at(&ours.join("first.md"), "x", 100);
        file_at(&ours.join("notes.txt"), "x", 300);
        file_at(&claudes.join("second.md"), "x", 200);
        let found = plans(&Dirs::at(&home), &project, 100);
        assert_eq!(
            found,
            vec![
                Doc { label: "Claude plan · second.md".into(), path: claudes.join("second.md") },
                Doc { label: "Plan · first.md".into(), path: ours.join("first.md") },
            ]
        );
        // Neither folder there is no plans, not an error.
        assert!(plans(&Dirs::at(&home.join("none")), &home.join("none"), 0).is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_list_is_reports_then_notes_then_plans() {
        let home = scratch("list");
        let project = home.join("p");
        file_at(&home.join("r.html"), "<html>", 1);
        file_at(&home.join("r.md"), "# r", 1);
        let added = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 60;
        file_at(&project.join("docs/superpowers/plans/plan.md"), "x", added + 30);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","entry":"{}","annotations":[
                {{"entry":"20261007T010000Z","description":"Report: {}/r.html"}}]}}"#,
            crate::task::stamp(added as i64),
            home.display()
        ));
        let notes = home.join("task-u.md");
        let lines: Vec<String> =
            list(&t, &Dirs::at(&home), &project, Some(&notes)).iter().map(Doc::line).collect();
        assert_eq!(
            lines,
            vec![
                format!("Report · 2026-10-07\t{}/r.md", home.display()),
                format!("Task · description and notes\t{}", notes.display()),
                format!("Plan · plan.md\t{}/docs/superpowers/plans/plan.md", project.display()),
            ]
        );
        std::fs::remove_dir_all(&home).ok();
    }
}
