//! `niritasks export` and `niritasks import`, run as the binary against
//! sandboxed task stores.
//!
//! Each sandbox has its own `TASKRC`, `TASKDATA` and `HOME`, so nothing here
//! reads or writes the real `~/.task` or reviews folder. Every test makes its
//! own sandboxes under its own names, so they can run in parallel.

use serde_json::Value;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-backup-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["data", "home", "bin"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        // So a refusal's notification does not pop up on the desktop.
        let fake = dir.join("bin/notify-send");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }

    fn reviews(&self) -> PathBuf {
        self.dir.join("home/.local/share/niri-tasks/reviews")
    }

    fn task(&self, args: &[&str]) -> Output {
        Command::new("task")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .args(["rc.verbose=nothing", "rc.confirmation=off", "rc.hooks=0"])
            .args(args)
            .output()
            .expect("task")
    }

    /// Add a task from word-split `args`, answering its uuid.
    fn add(&self, args: &[&str]) -> String {
        assert!(self.task(&[&["add"][..], args].concat()).status.success(), "task add");
        let uuid = String::from_utf8(self.task(&["+LATEST", "_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(uuid.len(), 36, "one new task, by uuid: {uuid:?}");
        uuid
    }

    fn niritasks(&self, args: &[&str]) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        Command::new(env!("CARGO_BIN_EXE_niritasks"))
            .args(args)
            .env("PATH", path)
            .env("HOME", self.dir.join("home"))
            .env_remove("XDG_DATA_HOME")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .output()
            .expect("run niritasks")
    }

    /// Every task as taskwarrior exports it, by uuid, less `id` and
    /// `urgency`: the working-set number and a figure that drifts with age.
    fn tasks(&self) -> BTreeMap<String, Value> {
        let out = self.task(&["rc.json.array=on", "rc.gc=off", "export"]);
        let list: Vec<Value> = serde_json::from_slice(&out.stdout).expect("task export JSON");
        list.into_iter()
            .map(|mut t| {
                let o = t.as_object_mut().unwrap();
                o.remove("id");
                o.remove("urgency");
                let uuid = o["uuid"].as_str().unwrap().to_string();
                (uuid, t)
            })
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A store of three tasks, one per status, the pending one tagged, with a
/// priority and two report notes: one whose file is there behind an anchor,
/// one whose file is gone. Two report files in its reviews folder, one in a
/// subfolder. Answers the sandbox and the pending task's uuid.
fn filled(name: &str) -> (Sandbox, String) {
    let s = Sandbox::new(name);
    std::fs::create_dir_all(s.reviews().join("sub")).unwrap();
    std::fs::write(s.reviews().join("kept.html"), "backed up").unwrap();
    std::fs::write(s.reviews().join("sub/deep.md"), "# deep").unwrap();

    let pending = s.add(&["feat:", "pending", "one", "+alpha", "priority:H"]);
    let notes = [
        "Report: ~/.local/share/niri-tasks/reviews/kept.html#slide-2",
        "Report (earlier draft): gone.html#s3",
    ];
    for note in notes {
        assert!(s.task(&[&pending, "annotate", "--", note]).status.success(), "annotate");
    }
    let done = s.add(&["fix:", "finished", "one"]);
    assert!(s.task(&[&done, "done"]).status.success(), "done");
    let gone = s.add(&["chore:", "removed", "one"]);
    assert!(s.task(&[&gone, "delete"]).status.success(), "delete");
    (s, pending)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn export_writes_one_archive_and_lists_the_missing_report() {
    let (s, pending) = filled("export");
    let archive = s.dir.join("backup.tar.gz");
    let out = s.niritasks(&["export", archive.to_str().unwrap()]);
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    assert!(out.status.success(), "export failed: {stderr}");
    assert!(stdout.contains(&format!("Backed up 3 tasks to {}", archive.display())), "{stdout}");

    // The gone report is listed by uuid8, status and path, anchor dropped;
    // the one that is there is not.
    assert!(stderr.contains(&format!("{}  pending    gone.html", &pending[..8])), "{stderr}");
    assert!(!stderr.contains("kept.html"), "{stderr}");

    let listing = text(&Command::new("tar").arg("-tzf").arg(&archive).output().unwrap().stdout);
    for member in ["tasks.json", "taskchampion.sqlite3", "reviews/kept.html", "reviews/sub/deep.md"] {
        assert!(listing.lines().any(|l| l.trim_end_matches('/') == member), "{member} not in:\n{listing}");
    }

    // The sqlite copy is a whole store, not a main file missing its WAL.
    let unpacked = s.dir.join("unpacked");
    std::fs::create_dir_all(&unpacked).unwrap();
    assert!(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(&unpacked).status().unwrap().success());
    let count = Command::new("sqlite3")
        .arg(unpacked.join("taskchampion.sqlite3"))
        .arg("select count(*) from tasks")
        .output()
        .expect("sqlite3");
    assert_eq!(text(&count.stdout).trim(), "3");
    let json: Vec<Value> = serde_json::from_slice(&std::fs::read(unpacked.join("tasks.json")).unwrap()).unwrap();
    assert_eq!(json.len(), 3);

    // A second export to the same file is refused, the first left alone.
    let again = s.niritasks(&["export", archive.to_str().unwrap()]);
    assert!(!again.status.success(), "a second export overwrote the archive");
    assert!(text(&again.stderr).contains("already there"), "{}", text(&again.stderr));
}

/// Export the `filled` store, answering the archive's path.
fn exported(s: &Sandbox) -> PathBuf {
    let archive = s.dir.join("backup.tar.gz");
    let out = s.niritasks(&["export", archive.to_str().unwrap()]);
    assert!(out.status.success(), "export failed: {}", text(&out.stderr));
    archive
}

#[test]
fn import_into_an_empty_store_gives_back_every_task_and_report() {
    let (src, _) = filled("roundtrip-src");
    let archive = exported(&src);
    let dst = Sandbox::new("roundtrip-dst");
    // A report the new machine already has is not overwritten.
    std::fs::create_dir_all(dst.reviews()).unwrap();
    std::fs::write(dst.reviews().join("kept.html"), "mine").unwrap();

    let out = dst.niritasks(&["import", archive.to_str().unwrap()]);
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "import failed: {}", text(&out.stderr));
    assert!(
        stdout.contains("Imported 3 tasks; restored 1 report files, kept 1 already there"),
        "{stdout}"
    );

    // Same uuids, statuses, tags, notes and dates: every field but id and urgency.
    assert_eq!(dst.tasks(), src.tasks());
    assert_eq!(std::fs::read_to_string(dst.reviews().join("kept.html")).unwrap(), "mine");
    assert_eq!(std::fs::read_to_string(dst.reviews().join("sub/deep.md")).unwrap(), "# deep");
}

#[test]
fn import_refuses_a_store_with_tasks_unless_forced() {
    let (s, pending) = filled("guard");
    let archive = exported(&s);
    // An edit made after the backup, which the guard is there to keep.
    assert!(s.task(&[&pending, "modify", "feat: newer edit"]).status.success(), "modify");
    let description = |s: &Sandbox| s.tasks()[&pending]["description"].as_str().unwrap().to_string();

    let out = s.niritasks(&["import", archive.to_str().unwrap()]);
    assert!(!out.status.success(), "import into a store with tasks was not refused");
    assert!(text(&out.stderr).contains("--force"), "{}", text(&out.stderr));
    assert_eq!(description(&s), "feat: newer edit");

    let forced = s.niritasks(&["import", "--force", archive.to_str().unwrap()]);
    assert!(forced.status.success(), "import --force failed: {}", text(&forced.stderr));
    assert_eq!(description(&s), "feat: pending one");
}
