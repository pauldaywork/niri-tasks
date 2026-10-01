//! `niritasks task status <uuid> active`, run inside a herdr pane, names that
//! pane's agent after the task. Checked by running the real binary against a
//! fake `herdr` on `PATH` that logs what it is asked, and a scratch task
//! database, so no herdr server and no real task is touched.
//!
//! herdr's pane variables are scrubbed from the child's environment first:
//! this test is as likely as not to run inside a herdr pane itself, and must
//! never rename the agent running it.
//!
//! One test function, run in order, because writing an executable and then
//! spawning it from parallel test threads can fail with "Text file busy".

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Logs every call; `agent get` answers `$FAKE_AGENT_GET`, or fails as herdr
/// does on a pane with no agent when that is empty; `agent rename` fails as
/// herdr does on a taken name when `$FAKE_RENAME_FAILS` is set.
const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  *" agent get "*)
    if [ -n "$FAKE_AGENT_GET" ]; then printf '%s' "$FAKE_AGENT_GET"; exit 0; fi
    printf '%s' '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2
    exit 1 ;;
  *" agent rename "*)
    if [ -n "$FAKE_RENAME_FAILS" ]; then
      printf '%s' '{"error":{"code":"agent_name_taken","message":"agent name is already in use"}}' >&2
      exit 1
    fi ;;
esac
printf '{}'
"#;

const UNNAMED_CLAUDE: &str = r#"{"result":{"agent":{"agent":"claude","pane_id":"w1:p2"}}}"#;

fn write_exe(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Sandbox {
    dir: PathBuf,
    uuid: String,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-link-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        write_exe(&dir.join("bin/herdr"), FAKE_HERDR);
        // So the status notification does not pop up on the desktop.
        write_exe(&dir.join("bin/notify-send"), "#!/bin/sh\nexit 0\n");

        let mut s = Self { dir, uuid: String::new() };
        assert!(s.task(&["add", "link me"]).status.success(), "task add");
        s.uuid = String::from_utf8(s.task(&["_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(s.uuid.len(), 36, "one task, by uuid");
        s
    }

    fn task(&self, args: &[&str]) -> Output {
        Command::new("task")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .args(["rc.verbose=nothing", "rc.confirmation=off"])
            .args(args)
            .output()
            .expect("task")
    }

    fn log(&self) -> PathBuf {
        self.dir.join("herdr.log")
    }

    /// What the fake herdr was asked since the last call, one call per line.
    fn take_calls(&self) -> String {
        let calls = std::fs::read_to_string(self.log()).unwrap_or_default();
        let _ = std::fs::remove_file(self.log());
        calls
    }

    /// `niritasks task status <uuid> active`, in pane `w1:p2` of session
    /// `alpha` when `in_pane`, with every other herdr variable removed.
    fn mark_active(&self, in_pane: bool, agent_get: &str, rename_fails: bool) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_niritasks"));
        cmd.args(["task", "status", &self.uuid, "active"])
            .env("PATH", path)
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .env("FAKE_HERDR_LOG", self.log())
            .env("FAKE_AGENT_GET", agent_get)
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SOCKET_PATH")
            .env_remove("HERDR_TAB_ID")
            .env_remove("HERDR_WORKSPACE_ID")
            .env_remove("FAKE_RENAME_FAILS");
        if in_pane {
            cmd.env("HERDR_SESSION", "alpha").env("HERDR_PANE_ID", "w1:p2");
        }
        if rename_fails {
            cmd.env("FAKE_RENAME_FAILS", "1");
        }
        cmd.output().expect("run niritasks")
    }

    fn is_active(&self) -> bool {
        let out = self.task(&["rc.json.array=on", &self.uuid, "export"]);
        String::from_utf8_lossy(&out.stdout).contains("\"start\"")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn marking_a_task_active_links_the_panes_agent() {
    let s = Sandbox::new();
    let work_name = format!("work-{}", &s.uuid[..8]);

    // ---- outside herdr: herdr is never asked ----------------------------
    let out = s.mark_active(false, UNNAMED_CLAUDE, false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(s.is_active());
    assert_eq!(s.take_calls(), "", "no pane, no herdr");

    // ---- a Claude started by hand takes the task's work- name -----------
    let out = s.mark_active(true, UNNAMED_CLAUDE, false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take_calls();
    assert!(calls.contains("--session alpha agent get w1:p2"), "{calls}");
    assert!(calls.contains(&format!("--session alpha agent rename w1:p2 {work_name}")), "{calls}");

    // ---- a pane Refine or Start working named is left alone -------------
    for name in ["task-0000aaaa", "work-0000bbbb"] {
        let named = format!(r#"{{"result":{{"agent":{{"agent":"claude","name":"{name}","pane_id":"w1:p2"}}}}}}"#);
        let out = s.mark_active(true, &named, false);
        assert!(out.status.success());
        let calls = s.take_calls();
        assert!(!calls.contains("agent rename"), "{name} kept: {calls}");
    }

    // ---- a plain shell pane has no agent to name ------------------------
    let out = s.mark_active(true, "", false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!s.take_calls().contains("agent rename"));

    // ---- a refused rename is reported, and the task is still active -----
    let out = s.mark_active(true, UNNAMED_CLAUDE, true);
    assert!(out.status.success(), "the status change stands");
    assert!(s.is_active());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("already in use"), "{stderr}");
}
