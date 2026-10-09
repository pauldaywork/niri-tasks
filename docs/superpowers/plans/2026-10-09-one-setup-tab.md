# One Setup Tab per Start Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two quick Start working presses on a task with no worktree open one `Start:` setup tab; the second press notifies "Already being set up — see its tab." instead of opening its own.

**Architecture:** A per-task `flock`, `task-<uuid8>.setup.lock` in the claims folder, goes through the session `Port` beside `claim` (real in `session/herdr.rs`, logged in `session/fake.rs`). The setup tab's process (`work::set_up_here`) holds it for its whole life, error wait included. `work::launch`, under the session claim, looks at it before opening a tab and, after `run_in_pane`, keeps the claim until it sees the tab holding it (a few seconds at most). The no-worktree half of `launch` moves into `open_setup_tab(&Session, …)` so the fake can test it.

**Tech Stack:** Rust 2021, `std::fs::File::try_lock` (flock), anyhow; the in-crate fake `Port`.

**Spec:** Taskwarrior task `a09a7da2-db93-49e7-94d9-2699e3281e08` (`task rc.json.array=on a09a7da2-db93-49e7-94d9-2699e3281e08 export`). Its notes, verbatim in substance:
- Goal: two quick Start working presses on a task with no worktree open one 'Start:' setup tab; the second press points at the first tab instead of opening its own.
- Context: since 21d6f58's session claim (bdcbbf7 after it), `work::launch` drops the claim once `run_in_pane` returns, while the first tab is still in `wt switch --create` (`set_up` claims again only after it). A second press then opens a second tab, which either finds the worktree (`focus_agent` stops a second Claude) or fails `wt switch` on the existing branch and sits with an error.
- Decided: `set_up_here` holds a per-task flock, `task-<uuid8>.setup.lock` in the claims dir (`$XDG_RUNTIME_DIR/niri-tasks`), for its whole life, including the 'Press Enter' error wait, so a press while an error shows points at that tab; the kernel drops it if the tab dies, so it can never go stale.
- Decided: `work::launch`, under the session claim and before `new_tab`, tries the setup lock without waiting; if it is held it notifies 'Already being set up — see its tab.' and returns Ok.
- Decided: to close the gap between `run_in_pane` and the tab's process starting, launch keeps the session claim after `run_in_pane` until it sees the setup lock held (polling, a few seconds at most, then lets go anyway); `set_up` waits briefly for the lock instead of failing at once, because launch's check takes it for a moment.
- Decided: the lock goes through the session Port, next to claim (flock in herdr.rs, modelled on `claim_in`; logged in the fake), so launch's refusal can be tested against the fake.
- Done when: a fake-session test shows a launch that finds the setup lock held opens no tab, runs nothing and notifies; a herdr.rs test shows the lock is exclusive and gone once its holder drops; two quick presses in real herdr give one Start tab and one Claude; `cargo test` passes.
- Out of scope: Refine's launch (no setup tab) and the worktree-already-exists path, which the session claim already covers.

## Global Constraints

- Notification text, exactly: `Already being set up — see its tab.` (an em dash).
- Lock file, exactly: `task-<uuid8>.setup.lock` in the claims folder (`claims_dir()`, i.e. `$XDG_RUNTIME_DIR/niri-tasks`), where `<uuid8>` is `names::uuid8` of the task's uuid.
- Launch's wait for the tab to hold the lock: "a few seconds at most" — here 50 looks, 100 ms apart (5 s), through the port's `sleep`.
- The setup tab's brief wait for the lock: 2 s, polled every `CLAIM_POLL` (100 ms) for real, like `claim_in`.
- Conventional Commits, scope `work` or `session`; subject ≤ 72 chars; body wrapped at 72 says what and why; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `cargo test` passes and `cargo build` and `cargo test --no-run` show no warnings after every task.
- Work in this worktree (`/home/paul/.worktrees/niri-tasks/task-fix-keep-two-quick-start-presses-to-one-a09a7da2`). Never run `install.sh`, `cargo install` or `systemctl --user restart niri-tasks`; never start herdr sessions — Task 3's real-herdr check is the user's.
- Docs on every new item; comments are full sentences that say why, matching the surrounding style. Line numbers below are anchors; match on quoted text.

---

### Task 1: The setup lock, held by the setup tab

The lock end to end on the tab's side: the flock primitive, the port method on both adapters, `Session::hold_setup`, and `set_up_here` holding it.

**Files:**
- Modify: `src/session/herdr.rs` (split `lock_in` out of `claim_in` ~L383-438; add `setup_lock_in`; `Process::setup_lock` after `claim` ~L350; tests after `a_claims_file_stays_in_its_folder` ~L540)
- Modify: `src/session.rs` (`Claim` doc ~L190; `Port::setup_lock` after `claim` ~L242; `SETUP_TAKE_WAIT` beside `SESSION_WAIT` ~L246; `Session::hold_setup` after `claim` ~L350; a test in `mod tests`)
- Modify: `src/session/fake.rs` (`Release` ~L188-200 and `claim` ~L372; `setup_lock`)
- Modify: `src/work.rs` (`set_up_here` ~L196-223, `set_up`'s claim comment ~L240-246)

**Interfaces:**
- Produces:
  - `pub(crate) fn setup_lock_in(dir: &Path, task: &str, wait: Duration, poll: Duration) -> Result<Option<File>>` in `session/herdr.rs`
  - `Port::setup_lock(&self, task: &str, wait: Duration) -> Result<Option<Claim>>` — `None` when still held after `wait`; a zero `wait` tries once.
  - `Session::hold_setup(&self, task: &str) -> Result<Claim>`
  - Fake log lines: `setup_lock <task> <wait>ms` on every try, `setup_release <task>` when a granted lock is dropped. (Task 2 adds `setup_held <task>`.)

- [ ] **Step 1: Write the failing herdr.rs tests**

Add to `mod tests` in `src/session/herdr.rs`, after `a_claims_file_stays_in_its_folder`:

```rust
    /// A setup lock is one holder's at a time; a zero wait tries once and
    /// does not wait; another task's is its own; and it is free again once
    /// its holder goes, as when a setup tab's process ends.
    #[test]
    fn a_setup_lock_is_exclusive_and_gone_with_its_holder() {
        let dir = scratch("setup");
        let task = "7cd9fd3a-d27b-4387-8249-aaf0d6785f90";
        let poll = Duration::from_millis(10);
        let first = setup_lock_in(&dir, task, Duration::ZERO, poll).unwrap().expect("a free lock is had");
        assert!(dir.join("task-7cd9fd3a.setup.lock").is_file());
        let started = Instant::now();
        assert!(setup_lock_in(&dir, task, Duration::ZERO, poll).unwrap().is_none(), "held by the first");
        assert!(started.elapsed() < Duration::from_millis(100), "a zero wait does not wait");
        assert!(setup_lock_in(&dir, "11111111-0000", Duration::ZERO, poll).unwrap().is_some());
        drop(first);
        assert!(setup_lock_in(&dir, task, Duration::ZERO, poll).unwrap().is_some(), "free once its holder goes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A setup tab that waits briefly gets the lock once a launch's quick
    /// look at it lets go.
    #[test]
    fn a_setup_lock_waited_for_is_had_once_let_go() {
        let dir = scratch("setup-wait");
        let task = "7cd9fd3a-0000";
        let first = setup_lock_in(&dir, task, Duration::ZERO, Duration::from_millis(10)).unwrap().unwrap();
        let dir2 = dir.clone();
        let second = std::thread::spawn(move || {
            setup_lock_in(&dir2, task, Duration::from_secs(5), Duration::from_millis(10)).unwrap().is_some()
        });
        std::thread::sleep(Duration::from_millis(100));
        drop(first);
        assert!(second.join().unwrap(), "the waiting tab got the lock");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A task that is not a uuid is folded by herdr's naming rule, so its
    /// lock file never reaches outside the claims' folder.
    #[test]
    fn a_setup_lock_file_stays_in_its_folder() {
        let dir = scratch("setup-name");
        let held = setup_lock_in(&dir, "../escape", Duration::ZERO, Duration::from_millis(10)).unwrap();
        assert!(held.is_some());
        assert!(dir.join("task-.._escap.setup.lock").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib setup_lock 2>&1 | tail -20`
Expected: compile error, `cannot find function setup_lock_in`.

- [ ] **Step 3: Split `lock_in` out of `claim_in` and add `setup_lock_in`**

In `src/session/herdr.rs`, replace `claim_in` (doc comment and body, from `/// Take \`session\`'s claim:` through its closing `}`) with:

```rust
/// Take `session`'s claim: an exclusive lock on `session-<name>.lock` in
/// `dir` (see [`lock_in`]), calling `waiting` once, the first time it is
/// found held; an error that says why once `wait` has passed.
pub(crate) fn claim_in(
    dir: &Path,
    session: &str,
    wait: Duration,
    poll: Duration,
    waiting: impl FnOnce(),
) -> Result<File> {
    // Through herdr's own naming rule, so a name from the environment can
    // never carry a `/` out of the folder; a name herdr accepts is unchanged.
    let name = format!("session-{}.lock", super::herdr_session_name(session));
    lock_in(dir, &name, wait, poll, waiting)?.with_context(|| {
        format!(
            "Another Refine or Start working in herdr session {session} has still not started its \
             Claude after {}s; it may be waiting for an answer in its tab. Try again once it has.",
            wait.as_secs()
        )
    })
}

/// Try `task`'s setup lock, which a setup tab holds for its whole life: an
/// exclusive lock on `task-<uuid8>.setup.lock` in `dir` (see [`lock_in`]),
/// or None when it is still held once `wait` has passed. A zero `wait`
/// tries once, which is how a launch looks to see whether it is held.
pub(crate) fn setup_lock_in(dir: &Path, task: &str, wait: Duration, poll: Duration) -> Result<Option<File>> {
    // Through herdr's naming rule as a session's name is, so a task that is
    // not a uuid can never carry a `/` out of the folder.
    let name = format!("task-{}.setup.lock", super::herdr_session_name(&crate::names::uuid8(task)));
    lock_in(dir, &name, wait, poll, || ())
}

/// An exclusive `flock` on `name` in `dir`, tried every `poll` until it is
/// had or `wait` has passed, calling `waiting` once, the first time it is
/// found held: the open, locked file, or None when it is still held at the
/// end. The lock goes with the returned file, when it is closed or this
/// process exits.
///
/// `flock` and not a lock file's mere existence, because the kernel drops it
/// with a holder that crashes: nothing stale is ever left to clean up. Tried
/// rather than blocked on, so the wait has an end.
///
/// It reads the clock and sleeps for real (`Instant`, `thread::sleep`)
/// rather than through the port's `sleep`, a conscious exception to the
/// design's Q16: this is the process adapter's own lock, under the port
/// rather than above it, and its tests race it on real threads in real time.
fn lock_in(dir: &Path, name: &str, wait: Duration, poll: Duration, waiting: impl FnOnce()) -> Result<Option<File>> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .with_context(|| format!("could not make {}", dir.display()))?;
    let path = dir.join(name);
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("could not open {}", path.display()))?;
    let deadline = Instant::now() + wait;
    let mut waiting = Some(waiting);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Some(file)),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                if let Some(say) = waiting.take() {
                    say();
                }
                std::thread::sleep(poll)
            }
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Error(e)) => {
                return Err(e).with_context(|| format!("could not lock {}", path.display()));
            }
        }
    }
}
```

The `has still not started its Claude` wording and the four existing claim tests must keep passing unchanged.

- [ ] **Step 4: Run the herdr tests**

Run: `cargo test --lib session::herdr 2>&1 | tail -20`
Expected: the three new tests and the four `claim` tests pass. (The crate may warn that `setup_lock_in` is unused outside tests until Step 6; that warning is gone by Step 9.)

- [ ] **Step 5: Write the failing Session test**

In `src/session.rs` `mod tests`, after the `logged_start` helper:

```rust
    /// A setup tab holds its task's setup lock, waiting briefly for it,
    /// until the claim it was given is dropped.
    #[test]
    fn a_setup_tab_holds_its_tasks_lock_until_it_goes() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        let held = s.hold_setup("7cd9fd3a-0000").unwrap();
        assert!(logged(&f, "setup_lock 7cd9fd3a-0000 2000ms"), "{:?}", f.log());
        assert!(!logged(&f, "setup_release 7cd9fd3a-0000"));
        drop(held);
        assert!(logged(&f, "setup_release 7cd9fd3a-0000"), "{:?}", f.log());
    }
```

Run: `cargo test --lib a_setup_tab_holds 2>&1 | tail -5`
Expected: compile error, no method `hold_setup`.

- [ ] **Step 6: Add the port method, both adapters, and `Session::hold_setup`**

`src/session.rs`, the `Claim` doc: replace its first paragraph with

```rust
/// A hold on a session's launches, taken by [`Session::claim`], or on a
/// task's setup, taken by [`Session::hold_setup`]; given up when dropped.
/// Whoever holds a session's claim is the one launch checking the session
/// for its agent and starting it, so a second press of Refine or Start
/// working waits, then finds the first one's agent instead of starting
/// another.
```

`Port`, after `fn claim(&self, session: &str) -> Result<Claim>;`:

```rust
    /// Try `task`'s setup lock, which a setup tab holds for its whole life,
    /// for up to `wait`: the lock, or None when another holds it still. A
    /// zero `wait` tries once.
    fn setup_lock(&self, task: &str, wait: Duration) -> Result<Option<Claim>>;
```

After `const PROMPT_RETRY …;`:

```rust
/// How long a setup tab waits for its task's setup lock: a launch looking to
/// see whether it is held takes it for a moment, so a tab starting just then
/// must not fail. Far longer than a look, far shorter than any real setup.
const SETUP_TAKE_WAIT: Duration = Duration::from_secs(2);
```

`Session`, after `pub fn claim`:

```rust
    /// Hold `task`'s setup lock until the returned claim is dropped: what a
    /// setup tab does for its whole life, so that a press of Start working
    /// meanwhile finds it held and points at that tab rather than opening
    /// another. Waits briefly ([`SETUP_TAKE_WAIT`]) since a launch's look
    /// takes it for a moment; held past that, another tab is setting the
    /// task up, and this one says so.
    pub fn hold_setup(&self, task: &str) -> Result<Claim> {
        self.port
            .setup_lock(task, SETUP_TAKE_WAIT)?
            .context("This task is already being set up in another tab.")
    }
```

`src/session/herdr.rs`, `impl Port for Process`, after `fn claim`:

```rust
    fn setup_lock(&self, task: &str, wait: Duration) -> Result<Option<Claim>> {
        Ok(setup_lock_in(&claims_dir(), task, wait, CLAIM_POLL)?.map(Claim::new))
    }
```

and change the `claims_dir` doc's first line to `/// Where the claims' and setup locks' files live: …`.

`src/session/fake.rs`: make `Release` log any line, replacing the struct and its `Drop`:

```rust
/// What the fake's claims hold: the fake itself, to log `line` when the
/// claim is dropped, which is the moment the process adapter's lock goes.
struct Release {
    fake: Fake,
    line: String,
}

impl Drop for Release {
    fn drop(&mut self) {
        self.fake.note(std::mem::take(&mut self.line));
    }
}
```

`claim`'s body becomes `Ok(Claim::new(Release { fake: self.clone(), line: format!("release {session}") }))` (its `self.note(format!("claim {session}"));` stays). Add after `claim`:

```rust
    fn setup_lock(&self, task: &str, wait: Duration) -> anyhow::Result<Option<Claim>> {
        self.note(format!("setup_lock {task} {}ms", wait.as_millis()));
        Ok(Some(Claim::new(Release { fake: self.clone(), line: format!("setup_release {task}") })))
    }
```

Also add to the fake's module doc's last line: `and every claim's and setup lock's release.`

- [ ] **Step 7: Run the Session test**

Run: `cargo test --lib a_setup_tab_holds 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 8: Hold the lock in `set_up_here`**

`src/work.rs`: replace `set_up_here`'s doc and body with:

```rust
/// The setup step, run inside the tab [`launch`] opened: make the worktree
/// (approval and hook output land here), open it as its own workspace, start
/// Claude there, then close this tab. On failure the tab stays, with the
/// error, until the user has read it.
///
/// It holds the task's setup lock from the start until this process ends,
/// the error's wait included, so that a press of Start working meanwhile
/// finds it held and points at this tab rather than opening another. The
/// kernel drops it with the process, so a closed tab never leaves it held.
/// `uuid` is the full one [`launch`] passes, whose first eight characters
/// name the lock.
///
/// `workspace` is the raw `--workspace` name, checked here rather than by the
/// caller so that a name [`Workspace::named`] refuses also holds the tab open
/// with its error, like any other failure.
pub fn set_up_here(workspace: &str, uuid: &str) -> Result<()> {
    // Kept only to be dropped when this function returns, after the wait for
    // Enter below.
    let mut _setup = None;
    let done = Workspace::named(workspace).and_then(|ws| {
        _setup = Some(ws.session()?.hold_setup(uuid)?);
        set_up(&ws, uuid).map(|()| ws)
    });
    match done {
        Ok(ws) => {
            // Best effort: closing our own tab ends this process, and a tab
            // left open is only untidy. It is closed through the workspace's
            // session, the one launch opened it in.
            if let Some(tab) = current_pane().and_then(|p| p.tab) {
                if let Ok(session) = ws.session() {
                    session.close_tab(&tab);
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("\n{e:#}\n\nPress Enter to close this tab.");
            let _ = std::io::stdin().read_line(&mut String::new());
            Err(e)
        }
    }
}
```

In `set_up`, extend the comment above `let name = names::work_agent(uuid);` — replace its last sentence ("Under the claim, the agent is looked for … instead of starting another.") with:

```rust
    // would wait with it. A second press meanwhile finds this tab's setup
    // lock held and opens no tab of its own; under the claim, the agent is
    // still looked for before one is started, in case one was started by
    // hand or by a tab from before the lock.
```

- [ ] **Step 9: Build clean and run every test**

Run: `cargo build 2>&1 | grep -c warning; cargo test --no-run 2>&1 | grep -c warning; cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `0`, `0`, and every `test result: ok`.

- [ ] **Step 10: Commit**

```bash
git add src/session.rs src/session/herdr.rs src/session/fake.rs src/work.rs
git commit -m "fix(work): hold a task's setup lock for its setup tab's life

A Start working setup tab now holds a per-task flock,
task-<uuid8>.setup.lock in the claims folder, from its start until
its process ends, its error wait included. The next change has launch
look at it, so a second quick press can point at this tab instead of
opening another. The lock goes through the session port beside the
claim, sharing its flock loop, so the fake can stand in for it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Launch points at a setup under way

**Files:**
- Modify: `src/session.rs` (`SETUP_POLLS`, `SETUP_POLL`; `Session::setup_under_way`, `wait_for_setup`, `notify`; tests)
- Modify: `src/session/fake.rs` (`State::setup_free_tries`, `with_setup_held_after`, `setup_lock` honours it)
- Modify: `src/work.rs` (`launch` ~L126-177 and its doc; new `open_setup_tab` and `SETUP_UNDER_WAY`; tests)

**Interfaces:**
- Consumes: `Port::setup_lock`, `Session::hold_setup`, fake lines `setup_lock <task> <n>ms` / `setup_release <task>` (Task 1).
- Produces:
  - `Session::setup_under_way(&self, task: &str) -> Result<bool>`
  - `Session::wait_for_setup(&self, task: &str) -> Result<()>`
  - `Session::notify(&self, text: &str)`
  - `Fake::with_setup_held_after(self, tries: u32) -> Fake` — the lock is granted `tries` times, then found held on every try after; fake logs `setup_held <task>` for a held try.
  - `fn open_setup_tab(session: &Session, claim: Claim, workspaces: &[crate::session::Workspace], repo: &Path, workspace_label: &str, t: &task::Task, command: &str) -> Result<()>` (private, in `work.rs`)
  - `const SETUP_UNDER_WAY: &str = "Already being set up — see its tab.";` (private, in `work.rs`)

- [ ] **Step 1: Script a held lock in the fake**

`src/session/fake.rs`, in `struct State` after `prompt_codes`:

```rust
    /// How many more setup-lock tries are granted before the lock is found
    /// held on every try after, as once a setup tab has taken it; None, the
    /// default, grants every one.
    setup_free_tries: Option<u32>,
```

After `with_prompt_codes`:

```rust
    /// A setup lock granted `tries` times, then held by someone else: 0 is a
    /// setup tab already under way.
    pub fn with_setup_held_after(self, tries: u32) -> Fake {
        self.state.borrow_mut().setup_free_tries = Some(tries);
        self
    }
```

Replace the fake's `setup_lock` with:

```rust
    fn setup_lock(&self, task: &str, wait: Duration) -> anyhow::Result<Option<Claim>> {
        self.note(format!("setup_lock {task} {}ms", wait.as_millis()));
        let held = match self.state.borrow_mut().setup_free_tries.as_mut() {
            None => false,
            Some(0) => true,
            Some(n) => {
                *n -= 1;
                false
            }
        };
        if held {
            self.note(format!("setup_held {task}"));
            return Ok(None);
        }
        Ok(Some(Claim::new(Release { fake: self.clone(), line: format!("setup_release {task}") })))
    }
```

- [ ] **Step 2: Write the failing Session tests**

`src/session.rs` `mod tests`, after `a_setup_tab_holds_its_tasks_lock_until_it_goes`:

```rust
    /// A second setup tab for a task finds the lock held past its brief wait
    /// and says another tab has it.
    #[test]
    fn a_second_setup_tab_says_another_has_it() {
        let (s, _f) = session(Fake::running(&[("w1", "alpha")]).with_setup_held_after(0));
        let err = s.hold_setup("7cd9fd3a-0000").map(drop).unwrap_err().to_string();
        assert!(err.contains("already being set up in another tab"), "{err}");
    }

    /// A look at the setup lock tries once, without waiting, and lets a free
    /// lock go at once so the setup tab can take it.
    #[test]
    fn a_look_at_the_setup_lock_lets_it_go_at_once() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        assert!(!s.setup_under_way("7cd9fd3a-0000").unwrap());
        assert_eq!(f.log(), ["setup_lock 7cd9fd3a-0000 0ms", "setup_release 7cd9fd3a-0000"]);
        let (s, _f) = session(Fake::running(&[("w1", "alpha")]).with_setup_held_after(0));
        assert!(s.setup_under_way("7cd9fd3a-0000").unwrap());
    }

    /// Waiting for a setup tab looks until it holds the lock, sleeping
    /// between looks, and stops there.
    #[test]
    fn waiting_for_a_setup_tab_stops_once_it_holds_the_lock() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_setup_held_after(2));
        s.wait_for_setup("7cd9fd3a-0000").unwrap();
        let log = f.log();
        assert_eq!(log.iter().filter(|l| *l == "sleep 100ms").count(), 2, "{log:?}");
        assert_eq!(log.last().map(String::as_str), Some("setup_held 7cd9fd3a-0000"), "{log:?}");
    }

    /// A setup tab that never takes the lock is waited for five seconds at
    /// most.
    #[test]
    fn waiting_for_a_setup_tab_gives_up_after_a_few_seconds() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        s.wait_for_setup("7cd9fd3a-0000").unwrap();
        let slept: u128 = f
            .log()
            .iter()
            .filter_map(|l| l.strip_prefix("sleep ")?.strip_suffix("ms")?.parse::<u128>().ok())
            .sum();
        assert_eq!(slept, 5000);
    }
```

Run: `cargo test --lib session::tests 2>&1 | tail -5`
Expected: compile error, no method `setup_under_way` / `wait_for_setup`.

- [ ] **Step 3: Add the Session methods**

`src/session.rs`, after `SETUP_TAKE_WAIT`:

```rust
/// How many times, [`SETUP_POLL`] apart, a launch looks for the setup tab it
/// opened to hold its task's setup lock before letting its claim go anyway:
/// five seconds, against the moment a shell takes to start a command.
const SETUP_POLLS: u32 = 50;
/// The pause between those looks.
const SETUP_POLL: Duration = Duration::from_millis(100);
```

`Session`, after `hold_setup`:

```rust
    /// Whether a setup tab holds `task`'s setup lock now. Looks once, without
    /// waiting, and lets a free lock go at once, so a setup tab starting just
    /// then gets it within its brief wait.
    pub fn setup_under_way(&self, task: &str) -> Result<bool> {
        Ok(self.port.setup_lock(task, Duration::ZERO)?.is_none())
    }

    /// Wait for the setup tab a launch just opened to hold `task`'s setup
    /// lock, for a few seconds at most: a tab whose command never starts must
    /// not keep the launch's claim, and every launch in the session with it.
    pub fn wait_for_setup(&self, task: &str) -> Result<()> {
        for _ in 0..SETUP_POLLS {
            if self.setup_under_way(task)? {
                return Ok(());
            }
            self.port.sleep(SETUP_POLL);
        }
        Ok(())
    }

    /// Tell the user something, as the port does: for a step whose outcome
    /// is only a notification, such as a press that finds its work under way.
    pub fn notify(&self, text: &str) {
        self.port.notify(text)
    }
```

Run: `cargo test --lib session::tests 2>&1 | tail -5`
Expected: PASS. (`setup_under_way`, `wait_for_setup` and `notify` warn as unused until Step 6.)

- [ ] **Step 4: Write the failing launch tests**

`src/work.rs` `mod tests`, after `a_failed_start_lets_the_claim_go`:

```rust
    fn task(uuid: &str, description: &str) -> task::Task {
        serde_json::from_value(serde_json::json!({"uuid": uuid, "description": description})).unwrap()
    }

    fn workspaces() -> Vec<crate::session::Workspace> {
        vec![crate::session::Workspace { id: "w1".into(), label: "alpha".into() }]
    }

    const U: &str = "7cd9fd3a-d27b-4387-8249-aaf0d6785f90";

    /// A press that finds a setup tab already under way for its task opens
    /// no tab, runs nothing and points the user at the tab there is.
    #[test]
    fn a_launch_finding_its_setup_under_way_opens_no_tab() {
        let fake = Fake::running(&[("w1", "alpha")]).with_setup_held_after(0);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert!(
            !log.iter().any(|l| l.starts_with("tab_create") || l.starts_with("workspace_create") || l.starts_with("pane_run")),
            "{log:?}"
        );
        assert!(log.contains(&format!("notify {SETUP_UNDER_WAY}")), "{log:?}");
    }

    /// A launch keeps its claim past running the setup command until it sees
    /// the tab holding the setup lock, so a second press waiting on the
    /// claim finds the lock held rather than opening a tab of its own.
    #[test]
    fn a_launch_keeps_its_claim_until_its_setup_tab_holds_the_lock() {
        // One free try for the look before the tab, two while the tab starts.
        let fake = Fake::running(&[("w1", "alpha")]).with_setup_held_after(3);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert!(at(&log, "setup_release") < at(&log, "tab_create alpha w1 /p/alpha Start: Fix it"), "{log:?}");
        assert!(at(&log, "pane_run alpha") < at(&log, "setup_held"), "{log:?}");
        assert!(at(&log, "setup_held") < at(&log, "release alpha"), "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("notify")), "{log:?}");
    }

    /// A setup tab that never takes the lock is waited for a few seconds,
    /// then the claim goes anyway.
    #[test]
    fn a_launch_lets_its_claim_go_when_its_setup_tab_never_shows() {
        let fake = Fake::running(&[("w1", "alpha")]);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert_eq!(log.last().map(String::as_str), Some("release alpha"), "{log:?}");
        assert!(at(&log, "pane_run alpha") < at(&log, "release alpha"), "{log:?}");
    }
```

Run: `cargo test --lib work::tests 2>&1 | tail -5`
Expected: compile error, `cannot find function open_setup_tab` and `SETUP_UNDER_WAY`.

- [ ] **Step 5: Add `open_setup_tab`**

`src/work.rs`, after `launch`:

```rust
/// What a press of Start working says when a setup tab for its task, from an
/// earlier press, is still open.
const SETUP_UNDER_WAY: &str = "Already being set up — see its tab.";

/// Open `t`'s setup tab and run `command` in it, under the launch's `claim`
/// — unless a setup tab for it is under way already, from an earlier press,
/// in which case the user is pointed at that one and nothing opens.
///
/// The claim is kept past `run_in_pane` until the tab's process is seen
/// holding the setup lock: in between, a second press waiting on the claim
/// would find neither a worktree nor a held lock, and open a second tab. A
/// tab whose command never starts is waited for a few seconds at most.
fn open_setup_tab(
    session: &Session,
    claim: Claim,
    workspaces: &[crate::session::Workspace],
    repo: &Path,
    workspace_label: &str,
    t: &task::Task,
    command: &str,
) -> Result<()> {
    if session.setup_under_way(&t.uuid)? {
        session.notify(SETUP_UNDER_WAY);
        return Ok(());
    }
    let label = format!("Start: {}", names::elide(&t.description));
    let tab = session.new_tab(workspaces, repo, &label, workspace_label)?;
    session.run_in_pane(&tab.pane, command)?;
    session.wait_for_setup(&t.uuid)?;
    drop(claim);
    Ok(())
}
```

- [ ] **Step 6: Call it from `launch`**

In `launch`, replace everything from the comment `// The claim goes when this returns: …` to the end of the function with:

```rust
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let command = format!(
        "{} task start --here --workspace {} {}",
        sh_quote(&exe.display().to_string()),
        sh_quote(ws.name()),
        sh_quote(&t.uuid)
    );
    open_setup_tab(&session, claim, &workspaces, &repo, ws.name(), t, &command)
}
```

and replace the last paragraph of `launch`'s doc comment ("Everything from opening the session … rather than starting its own.") with:

```rust
/// Everything from opening the session to Claude's start, or to the setup
/// tab's process holding the task's setup lock, runs under the session's
/// claim, so a second press waits and then finds this one's Claude,
/// worktree or setup tab rather than starting its own. A press that finds a
/// setup tab under way, even one showing an error, points at it instead.
```

- [ ] **Step 7: Run the tests**

Run: `cargo test --lib work::tests 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 8: Build clean, clippy, every test**

Run: `cargo build 2>&1 | grep -c warning; cargo test --no-run 2>&1 | grep -c warning; cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c; cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `0`, `0`, no clippy line pointing into code this plan added, and every `test result: ok`. `git diff main --stat` lists only `src/session.rs`, `src/session/herdr.rs`, `src/session/fake.rs`, `src/work.rs` and this plan.

- [ ] **Step 9: Commit**

```bash
git add src/session.rs src/session/fake.rs src/work.rs
git commit -m "fix(work): keep two quick Start presses to one setup tab

Launch dropped the session claim as soon as it had typed the setup
command, while the tab was still making the worktree, so a second
quick press opened a second Start tab that started a second Claude or
failed on the existing branch. Launch now looks at the task's setup
lock first and, when a tab holds it, notifies \"Already being set up
— see its tab.\" and opens nothing. After running the command it
keeps the claim until the new tab holds the lock, five seconds at
most, closing the gap before the tab's process starts.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Check it in real herdr

This is the user's to run (the plan forbids installing or starting herdr sessions); the executor asks for it and records the result.

- [ ] **Step 1: Ask the user to install the branch build and try it**

Ask the user to run, in a project with a git repo and a pending task that has no worktree:

1. `! cargo install --path . --locked` (or their usual `install.sh`), then `! systemctl --user restart niri-tasks`.
2. On the task's card, press Start working twice in quick succession (and, separately, once more while the first tab is still waiting on worktrunk's hook approval).

Expected: one `Start:` tab; the extra presses each notify "Already being set up — see its tab."; once approved, one worktree workspace and one Claude (`work-<uuid8>`). `ls $XDG_RUNTIME_DIR/niri-tasks/` shows `task-<uuid8>.setup.lock`.

- [ ] **Step 2: Record the outcome**

If it behaves as expected, the task is done; land the branch with the `finish-worktree` skill. If not, take the user's observations to `superpowers:systematic-debugging` before changing code.
