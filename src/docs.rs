//! A task's documents, as `niritasks task docs` lists them for herdr's
//! task-docs plugin: its refine reports, its own description and notes, and
//! the plans written since it was added. Each is a Markdown file
//! plannotator-tui can open beside the task's Claude, bar a report made
//! before reports had a Markdown twin, which opens in the browser.

use crate::{dirs::Dirs, names, task, task::Task, text};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
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
/// A file in `times` counts from that time, any other from when it was last
/// changed on disk.
fn plans_in(folder: &Path, kind: &str, since: i64, times: &HashMap<PathBuf, i64>) -> Vec<(i64, Doc)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Some(changed) = times.get(&path).copied().or_else(|| {
            path.metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
        }) else {
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

/// Each path's last commit time from `git log --format=%ct --name-only`,
/// which lists the newest commit first: a path's first appearance is its
/// last commit.
pub fn parse_commit_times(log: &str) -> HashMap<String, i64> {
    let mut times = HashMap::new();
    let mut now = None;
    for line in log.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        match line.parse::<i64>() {
            Ok(secs) if !line.contains('/') => now = Some(secs),
            _ => {
                if let Some(secs) = now {
                    times.entry(line.to_string()).or_insert(secs);
                }
            }
        }
    }
    times
}

/// `git` run in `project`; None when git is missing or fails.
fn git(project: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(project).args(["-c", "core.quotepath=off"]).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The last commit time of each plan git tracks in the project's plans
/// folder, bar those changed or untracked since, which keep their file times.
/// A checkout stamps every file with when it was made, so a fresh worktree
/// would otherwise show every plan as new. No git, no repo or a failure is
/// an empty map: every plan keeps its file time.
fn commit_times(project: &Path) -> HashMap<PathBuf, i64> {
    const PLANS: &str = "docs/superpowers/plans";
    let Some(log) = git(project, &["log", "--format=%ct", "--name-only", "--relative", "--", PLANS]) else {
        return HashMap::new();
    };
    let Some(local) = git(project, &["ls-files", "-m", "-o", "--exclude-standard", "--", PLANS]) else {
        return HashMap::new();
    };
    let local: HashSet<&str> = local.lines().collect();
    parse_commit_times(&log)
        .into_iter()
        .filter(|(path, _)| !local.contains(path.as_str()))
        .map(|(path, secs)| (project.join(path), secs))
        .collect()
}

/// The plans written since `since`: the project's `docs/superpowers/plans`
/// and `~/.claude/plans`, newest first. A plan for a task cannot predate it,
/// and both folders hold plans for much else. A project plan counts from its
/// last commit when git tracks it unchanged, else from its file time; Claude's
/// from their file time.
pub fn plans(dirs: &Dirs, project: &Path, since: i64) -> Vec<Doc> {
    let mut found = plans_in(&project.join("docs/superpowers/plans"), "Plan", since, &commit_times(project));
    found.extend(plans_in(&dirs.home().join(".claude/plans"), "Claude plan", since, &HashMap::new()));
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
    found.into_iter().map(|(_, doc)| doc).collect()
}

/// The Markdown files the task's other notes name by path (a handoff, say),
/// newest note first, each once and only when it is a file. A `~/` path is
/// under the home folder. A report already listed is not listed again.
pub fn linked(t: &Task, dirs: &Dirs, listed: &[Doc]) -> Vec<Doc> {
    let mut seen: HashSet<PathBuf> = listed.iter().map(|d| d.path.clone()).collect();
    t.linked_markdown()
        .into_iter()
        .map(|p| match p.strip_prefix("~/") {
            Some(rest) => dirs.home().join(rest),
            None => PathBuf::from(p),
        })
        .filter(|p| p.is_file() && seen.insert(p.clone()))
        .map(|path| {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            Doc { label: format!("Linked · {name}"), path }
        })
        .collect()
}

/// Everything `task docs` lists, in order: the reports, then the Markdown
/// the notes name, then the notes file when there is one, then the plans since the task was added. A task whose
/// `entry` does not parse counts every plan.
pub fn list(t: &Task, dirs: &Dirs, project: &Path, notes: Option<&Path>) -> Vec<Doc> {
    let mut docs = reports(t);
    docs.extend(linked(t, dirs, &docs));
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

    /// A handoff a note names: listed when it is a file, once, after the
    /// reports and before the notes; `~/` is the home folder.
    #[test]
    fn linked_markdown_is_listed_once_when_it_exists() {
        let home = scratch("linked");
        file_at(&home.join("reviews/handoff.md"), "# h", 1);
        file_at(&home.join("reviews/page.html"), "<html>", 1);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","annotations":[
                {{"entry":"20261006T010000Z","description":"Handoff: ~/reviews/handoff.md"}},
                {{"entry":"20261007T010000Z","description":"Again `{h}/reviews/handoff.md`, not {h}/reviews/gone.md or {h}/reviews/page.html"}}
            ]}}"#,
            h = home.display()
        ));
        let notes = home.join("task-u.md");
        let lines: Vec<String> =
            list(&t, &Dirs::at(&home), &home.join("none"), Some(&notes)).iter().map(Doc::line).collect();
        assert_eq!(
            lines,
            vec![
                format!("Linked · handoff.md\t{}/reviews/handoff.md", home.display()),
                format!("Task · description and notes\t{}", notes.display()),
            ]
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn commit_times_come_from_the_first_log_mention() {
        let log = "1800\n\ndocs/superpowers/plans/b.md\n\n1700\n\ndocs/superpowers/plans/a.md\ndocs/superpowers/plans/b.md\n";
        let times = parse_commit_times(log);
        assert_eq!(times["docs/superpowers/plans/b.md"], 1800);
        assert_eq!(times["docs/superpowers/plans/a.md"], 1700);
        assert_eq!(times.len(), 2);
        assert!(parse_commit_times("").is_empty());
    }

    fn git_in(dir: &Path, date: &str, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .env("GIT_COMMITTER_DATE", date)
            .env("GIT_AUTHOR_DATE", date)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    /// A checkout gives every file a fresh time; the plans count from their
    /// commits, bar an untracked one, which keeps its file time. From a
    /// subfolder of the repo too.
    #[test]
    fn plans_count_from_their_commit_not_the_checkout() {
        let home = scratch("gitplans");
        let repo = home.join("repo");
        let project = repo.join("sub");
        let plans = project.join("docs/superpowers/plans");
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        std::fs::create_dir_all(&plans).unwrap();
        git_in(&repo, "1600000000 +0000", &["init", "-q"]);
        file_at(&plans.join("early.md"), "x", now);
        git_in(&repo, "1600000000 +0000", &["add", "."]);
        git_in(&repo, "1600000000 +0000", &["commit", "-q", "-m", "early"]);
        file_at(&plans.join("late.md"), "x", now);
        git_in(&repo, "1800000000 +0000", &["add", "."]);
        git_in(&repo, "1800000000 +0000", &["commit", "-q", "-m", "late"]);
        file_at(&plans.join("new.md"), "x", now);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","entry":"{}"}}"#,
            crate::task::stamp(1_700_000_000)
        ));
        let names: Vec<String> = list(&t, &Dirs::at(&home), &project, None).into_iter().map(|d| d.label).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"Plan · late.md".to_string()) && names.contains(&"Plan · new.md".to_string()));
        std::fs::remove_dir_all(&home).ok();
    }
}
