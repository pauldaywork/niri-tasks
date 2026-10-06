//! `niritasks task start`, `task start --here` and `task refine` name what
//! they make after the task's own uuid, not the form it was typed in. A task
//! number finds the same task, but a branch or agent named from it ends in
//! `-1`, not the uuid's first eight characters, so the task panel, which
//! passes full uuids, never finds it again and makes a second worktree, a
//! second Claude or a second refine session.
//!
//! Checked by running the real binary against a scratch task database, fake
//! `herdr`, `wt` and `notify-send` on `PATH` that log what they are asked, and
//! a fake niri socket that reports one focused workspace, `alpha`. No herdr
//! server, worktree or real task is touched.
//!
//! herdr's pane variables are scrubbed from the child's environment first:
//! this test is as likely as not to run inside a herdr pane itself, and must
//! never act on the pane running it.
//!
//! One test function, run in order, because writing an executable and then
//! spawning it from parallel test threads can fail with "Text file busy".

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Logs every call. `agent get` finds only the agent named `$FAKE_AGENT`
/// and fails as herdr does on any other. Every other call answers with the
/// fields the launchers read: a workspace, its root pane, and an empty
/// workspace list, so no niri window is looked for.
const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  *" agent get $FAKE_AGENT")
    printf '{"result":{"agent":{"agent":"claude","name":"%s","agent_status":"idle"}}}' "$FAKE_AGENT"
    exit 0 ;;
  *" agent get "*)
    printf '%s' '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2
    exit 1 ;;
esac
printf '%s' '{"result":{"workspace":{"workspace_id":"w1"},"root_pane":{"pane_id":"w1:p1"},"workspaces":[]}}'
"#;

/// Logs every call. `wt list` answers `$FAKE_WT_LIST`, `wt switch` answers
/// `$FAKE_WT_SWITCH`.
const FAKE_WT: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_WT_LOG"
case "$*" in
  *" list "*) printf '%s' "$FAKE_WT_LIST" ;;
  *" switch "*) printf '%s' "$FAKE_WT_SWITCH" ;;
esac
"#;

/// A repository with no task worktrees.
const NO_WORKTREES: &str = r#"{"items":[]}"#;

fn write_exe(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A niri IPC socket that answers every request with one focused workspace,
/// `alpha`. Directly under `/tmp` rather than the test's temp folder: Refine
/// refuses to start while a socket sits where its sandbox could reach it,
/// and its sandbox keeps `/tmp/claude-<uid>`, where a Claude session's
/// `TMPDIR` may point, in reach.
fn fake_niri() -> PathBuf {
    let path = PathBuf::from(format!("/tmp/niritasks-number-test-niri-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind the fake niri socket");
    let alpha = niri_ipc::Workspace {
        id: 1,
        idx: 1,
        name: Some("alpha".to_string()),
        output: None,
        is_urgent: false,
        is_active: true,
        is_focused: true,
        active_window_id: None,
    };
    let reply: niri_ipc::Reply = Ok(niri_ipc::Response::Workspaces(vec![alpha]));
    let line = format!("{}\n", serde_json::to_string(&reply).unwrap());
    // One request per connection, as niri_ipc's Socket sends them.
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            if reader.read_line(&mut request).is_ok() {
                let _ = reader.get_mut().write_all(line.as_bytes());
            }
        }
    });
    path
}

struct Sandbox {
    dir: PathBuf,
    niri_socket: PathBuf,
    uuid: String,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-number-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["data", "bin", "home/Projects/alpha/.git", "worktree", "xdg/niri-tasks/refine-mod/.claude-plugin"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        // Refine refuses to start without the mod, and the real one is
        // wherever install.sh put it, not something a test may rely on.
        std::fs::write(dir.join("xdg/niri-tasks/refine-mod/.claude-plugin/plugin.json"), "{}").unwrap();
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        write_exe(&dir.join("bin/herdr"), FAKE_HERDR);
        write_exe(&dir.join("bin/wt"), FAKE_WT);
        // So the notifications do not pop up on the desktop.
        write_exe(&dir.join("bin/notify-send"), "#!/bin/sh\nexit 0\n");

        let niri_socket = fake_niri();
        let mut s = Self { dir, niri_socket, uuid: String::new() };
        assert!(s.task(&["add", "start me"]).status.success(), "task add");
        s.uuid = String::from_utf8(s.task(&["1", "_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(s.uuid.len(), 36, "task 1, by uuid");
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

    fn herdr_log(&self) -> PathBuf {
        self.dir.join("herdr.log")
    }

    fn wt_log(&self) -> PathBuf {
        self.dir.join("wt.log")
    }

    /// What a fake was asked since the last call, one call per line.
    fn take(&self, log: &Path) -> String {
        let calls = std::fs::read_to_string(log).unwrap_or_default();
        let _ = std::fs::remove_file(log);
        calls
    }

    /// `niritasks <args>` with `home` as `HOME`, the fakes first on `PATH`,
    /// `agent` the one herdr agent that exists, and `wt_list` what `wt list`
    /// says. `wt switch` always reports the task's branch, made in
    /// `worktree/`.
    fn niritasks(&self, args: &[&str], home: &Path, agent: &str, wt_list: &str) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        let switched = format!(
            r#"{{"branch":"task/start-me-{}","path":"{}"}}"#,
            &self.uuid[..8],
            self.dir.join("worktree").display()
        );
        Command::new(env!("CARGO_BIN_EXE_niritasks"))
            .args(args)
            .env("PATH", path)
            .env("HOME", home)
            .env("XDG_DATA_HOME", self.dir.join("xdg"))
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .env("NIRI_SOCKET", &self.niri_socket)
            .env("FAKE_HERDR_LOG", self.herdr_log())
            .env("FAKE_WT_LOG", self.wt_log())
            .env("FAKE_AGENT", agent)
            .env("FAKE_WT_LIST", wt_list)
            .env("FAKE_WT_SWITCH", switched)
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SOCKET_PATH")
            .env_remove("HERDR_TAB_ID")
            .env_remove("HERDR_WORKSPACE_ID")
            .output()
            .expect("run niritasks")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = std::fs::remove_file(&self.niri_socket);
    }
}

#[test]
fn a_task_number_names_everything_after_the_tasks_uuid() {
    let s = Sandbox::new();
    let uuid8 = &s.uuid[..8];
    let branch = format!("task/start-me-{uuid8}");
    let work = format!("work-{uuid8}");
    let home = s.dir.join("home");

    // ---- Start goes back to the worktree the task already has -----------
    // Made as the panel's Start working makes it: on a branch that ends in
    // the uuid's first eight characters, with its Claude still running.
    let has_worktree = format!(
        r#"{{"items":[{{"branch":"{branch}","worktree":{{"path":"{}"}}}}]}}"#,
        s.dir.join("worktree").display()
    );
    let out = s.niritasks(&["task", "start", "1"], &home, &work, &has_worktree);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent focus {work}")), "{calls}");
    assert!(!calls.contains("pane run"), "no second worktree: {calls}");

    // ---- with no worktree yet, Start hands --here the full uuid ---------
    let out = s.niritasks(&["task", "start", "1"], &home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains("--session alpha pane run w1:p1 "), "{calls}");
    assert!(calls.contains(&format!("--here --workspace 'alpha' '{}'", s.uuid)), "{calls}");

    // ---- --here names the branch and the agent after the uuid -----------
    let out = s.niritasks(&["task", "start", "1", "--here", "--workspace", "alpha"], &home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let made = s.take(&s.wt_log());
    assert!(made.contains(&format!("switch --create {branch} --no-cd")), "{made}");
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent start {work} --kind claude")), "{calls}");
    assert!(
        calls.contains(&format!("agent prompt {work} /superpowers:writing-plans Plan Taskwarrior task {}.", s.uuid)),
        "{calls}"
    );

    // Refine with the real HOME: it refuses to start while a socket sits
    // outside its sandbox's hidden folders, and the real herdr sessions'
    // sockets are hidden only as $HOME/.config/herdr. The fakes on PATH
    // still keep it away from the real herdr. If Refine refuses here with
    // "Refine would leave these sockets in reach of its sandbox", the
    // machine has a real exposed socket, not a regression in this code.
    let real_home = PathBuf::from(std::env::var("HOME").unwrap());
    let refiner = format!("task-{uuid8}");

    // ---- Refine goes back to the session the task already has -----------
    let out = s.niritasks(&["task", "refine", "1"], &real_home, &refiner, NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent focus {refiner}")), "{calls}");
    assert!(!calls.contains("agent start"), "no second session: {calls}");

    // ---- a new refine session is named and prompted with the uuid -------
    let out = s.niritasks(&["task", "refine", "1"], &real_home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent start {refiner} ")), "{calls}");
    assert!(calls.contains(&format!("agent prompt {refiner} /refine-task {}", s.uuid)), "{calls}");
}
