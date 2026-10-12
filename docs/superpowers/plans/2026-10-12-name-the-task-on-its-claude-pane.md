# Name the Task on Its Claude's herdr Pane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Refine or Start working Claude's herdr pane says which task it is on: a "Refine #61 · …" / "Work #61 · …" label in its border, task tokens in its pane metadata, and a live `#id · description · state · branch` Claude Code status line.

**Architecture:** `names` spells the pane's label and tokens (`PaneName`). The `session` port grows two herdr calls, `pane rename` and `pane report-metadata --source niri-tasks`, behind one best-effort `Session::label_pane`. Refine, Start working and the pane link call it once Claude has started. A new `status_line` module prints the status line (`niritasks task status-line <uuid>`, run by Claude Code), and both Claudes get it through their launch `--settings`.

**Tech Stack:** Rust (clap, serde_json, anyhow), herdr 0.9.3 CLI, Claude Code `statusLine` setting, Taskwarrior.

**Spec:** Taskwarrior task 7b4f4c5c-185a-479e-8ab7-c1a6e5dad4b0 (its description and notes). Read it with `task rc.json.array=on 7b4f4c5c-185a-479e-8ab7-c1a6e5dad4b0 export`.

## What step 1 of the spec found (2026-10-12, herdr 0.9.3, live)

The spec's step 1 asked for three checks in a live herdr before any code. They were run against a throwaway, unfocused tab (`herdr tab create --no-focus`, then `pane rename`, then `pane report-metadata --title TITLE-PROBE --token …`), and the user looked at it:

- **(a) `--title` does not draw on the pane border.** It sets the pane's `title`, which only feeds the `terminal_title` sidebar token. The user's sidebar rows don't use that token. So the plan uses `pane rename`. herdr's docs agree: `ui.show_agent_labels_on_pane_borders` talks of "the manual pane label" as the border's text.
- **(b) A `pane rename` label shows in a lone pane's border** once `ui.pane_borders = "always"` is set (`pane_outer_borders` defaults to true, which `always` needs). Confirmed by the user on the probe tab.
- **(c) The user's herdr config** (`~/.config/herdr/config.toml`) sets no `pane_borders`, `pane_outer_borders`, `window_title` or sidebar rows. So the sidebar is herdr's default (`state_icon machine workspace tab` / `agent`), which shows neither the pane label nor `$task` tokens: the sidebar stays exactly as it is. The default `window_title` (`{hostname}: {workspace}`) has no `{pane}`, so `session::find_session_window` is unaffected by the label.
- `pane report-metadata` prints nothing on success (exit 0), which `run_coded` reads as `Null`. Tokens persist on the pane until cleared or the pane closes. `pane get` shows them under `tokens`, and the label under `label`.
- When the plan was written, `pane_borders = "always"` was **not yet in the user's config file**, although the user looked at the probe. Task 6 checks it again.

## Global Constraints

- Pane label: `Refine #<id> · <description>` for every refine mode (Refine, Grill me, Generate report: one agent, `task-<uuid8>`), `Work #<id> · <description>` for Start working and the pane link. herdr caps it at 80 characters, so elide it to 80 ourselves with `…`.
- Metadata: `herdr pane report-metadata <pane> --source niri-tasks --token task_id=<id> --token task=<description> --token task_uuid=<uuid>`. The sidebar config stays unchanged, so `$task` stays opt-in.
- Status line: `niritasks task status-line <uuid>` prints `#id · description · state · branch`. It reads the task again on every refresh, so it never goes stale.
- The status line reaches each Claude only through its launch `--settings`. The user's global Claude settings are not touched.
- niri-tasks never edits `~/.config/herdr/config.toml`. The README documents `ui.pane_borders = "always"` for the user to add.
- Only the `session` module talks to herdr (CONTEXT.md, **Session**).
- Labelling is best effort: a refused rename or report never fails a launch. The Claude is running by then.
- Out of scope: relabelling a pane when its description is edited later (the status line stays current), changing herdr sidebar rows, and editing the user's herdr config.
- Commits: Conventional Commits, `<type>(<scope>): <summary>`, at most 72 characters, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.
- Test command: `cargo test` (whole suite) must pass at the end of every task.

## Decisions this plan makes (not in the spec)

- **State words** in the status line mirror the Update status menu's labels (`task::Status::label`): `Active`, `Waiting`, `Completed`, `Deleted`. A pending task that isn't started reads `Pending`, Taskwarrior's own word, because the menu's `Stopped` would misdescribe a task never started.
- **Branch** is the git branch of the directory Claude Code reports on stdin (`workspace.current_dir`, else `cwd`, else the process's own directory). That's the task branch in a worktree and the project's branch for a refine. It is left out when git gives none (detached, or not a repo).
- **Id 0** (a finished task after gc): the label and the status line drop `#0` rather than show it, and the `task_id` token is left out.
- **Status line errors never surface as a desktop notification.** The command prints `task <uuid8>: not found` or `task <uuid8>: could not read it`, and always exits 0.
- **An unattended refine** (`claude -p`) gets no status line: there is no screen to show it on.
- **The pane link** labels a pane only when it renames its agent (an unnamed or otherwise-named Claude). A refine marking its task active keeps its `Refine …` label.

## File Structure

- `src/names.rs` (modify): `elide_to`, `Role`, `PaneName::for_task`. This is the one spelling of the label and tokens.
- `src/session.rs` (modify): two `Port` methods, `pane_rename` and `pane_report_metadata`, plus `Session::label_pane`. `Claude::Worker` gains `settings` in Task 5.
- `src/session/herdr.rs` (modify): the argv builders `pane_rename` and `pane_report_metadata`, `METADATA_SOURCE`, the `Process` impls, and the Worker's `--settings` in Task 5.
- `src/session/fake.rs` (modify): logs both new calls, and `Claude::Worker { .. }` in Task 5.
- `src/refine.rs` (modify): `launch_in` labels the pane, and `session_settings` takes an optional status-line command.
- `src/work.rs` (modify): `start_working` and `start_and_prompt` take the task and label the pane, and the Worker gets its status line.
- `src/link.rs` (modify): `link_current_pane` takes the task, and a testable `link_pane` renames and labels.
- `src/status_line.rs` (create): the status line's text, the command that prints it, and the `statusLine` setting.
- `src/lib.rs`, `src/main.rs` (modify): the module, the `task status-line` subcommand, and `apply_status` taking the task.
- `README.md`, `CONTEXT.md`, `llms.txt` (modify): docs.

---

### Task 1: The pane's label and tokens, spelled once

**Files:**
- Modify: `src/names.rs` (the `elide` function near line 45, and new items after it; tests in its `mod tests`)

**Interfaces:**
- Consumes: `crate::text::collapse_whitespace(&str) -> String` (exists).
- Produces:
  - `pub fn elide_to(text: &str, max: usize) -> String`
  - `pub const PANE_LABEL_MAX: usize = 80;`
  - `pub enum Role { Refine, Work }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `pub struct PaneName { pub label: String, pub tokens: Vec<(String, String)> }` (`Debug, Clone, PartialEq, Eq`)
  - `impl PaneName { pub fn for_task(role: Role, id: u64, uuid: &str, description: &str) -> PaneName }`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/names.rs`:

```rust
    #[test]
    fn a_pane_label_names_the_role_the_task_number_and_the_description() {
        let n = PaneName::for_task(Role::Refine, 61, "7b4f4c5c-185a", "feat: Name the task  on its\nClaude's pane");
        assert_eq!(n.label, "Refine #61 · feat: Name the task on its Claude's pane");
        let n = PaneName::for_task(Role::Work, 61, "7b4f4c5c-185a", "fix the peek");
        assert_eq!(n.label, "Work #61 · fix the peek");
    }

    /// herdr caps a label at 80 characters; cut here, with an ellipsis, it
    /// at least says it was cut.
    #[test]
    fn a_long_pane_label_is_elided_to_what_herdr_keeps() {
        let n = PaneName::for_task(Role::Work, 61, "u", &"word ".repeat(40));
        assert_eq!(n.label.chars().count(), PANE_LABEL_MAX);
        assert!(n.label.ends_with('…'), "{}", n.label);
    }

    /// A finished task loses its number to gc: no `#0`, no task_id.
    #[test]
    fn a_task_with_no_number_has_none_on_its_pane() {
        let n = PaneName::for_task(Role::Work, 0, "u-1", "fix the peek");
        assert_eq!(n.label, "Work · fix the peek");
        assert!(!n.tokens.iter().any(|(k, _)| k == "task_id"), "{:?}", n.tokens);
    }

    #[test]
    fn the_pane_tokens_carry_the_number_the_description_and_the_uuid() {
        let n = PaneName::for_task(Role::Refine, 61, "7b4f4c5c-185a", "fix  the peek");
        let tokens: Vec<(&str, &str)> = n.tokens.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(tokens, [("task_id", "61"), ("task", "fix the peek"), ("task_uuid", "7b4f4c5c-185a")]);
    }

    #[test]
    fn elide_to_cuts_by_characters() {
        assert_eq!(elide_to("abcdef", 4), "abc…");
        assert_eq!(elide_to("abcd", 4), "abcd");
        assert_eq!(elide("x".repeat(31).as_str()).chars().count(), 30, "elide keeps the tab label's 30");
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib names::`
Expected: a compile failure, because `PaneName`, `Role`, `PANE_LABEL_MAX` and `elide_to` are not found.

- [ ] **Step 3: Implement**

In `src/names.rs`, replace the body of `elide` with a call to a new `elide_to`, and add the rest after it. Keep `LABEL_MAX` as it is.

```rust
/// A description cut to fit a tab label, by characters, with an ellipsis. Its
/// whitespace is collapsed first, so a two-line description reads as one.
pub fn elide(description: &str) -> String {
    elide_to(description, LABEL_MAX)
}

/// `text` with its whitespace collapsed, cut to `max` characters with an
/// ellipsis when longer.
pub fn elide_to(text: &str, max: usize) -> String {
    let d = text::collapse_whitespace(text);
    if d.chars().count() > max {
        let cut: String = d.chars().take(max - 1).collect();
        format!("{cut}…")
    } else {
        d
    }
}

/// Longest pane label, and token value, herdr keeps (herdr 0.9.3 caps both
/// at 80 characters).
pub const PANE_LABEL_MAX: usize = 80;

/// Which of a task's Claudes a pane holds, as its border label says. Every
/// refine mode is `Refine`: they share one agent, `task-<uuid8>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Refine,
    Work,
}

/// What a task's Claude pane is called in herdr: the label drawn in its
/// border (with `ui.pane_borders = "always"`, even on a lone pane), and the
/// display-only tokens a sidebar row can show as `$task_id`, `$task` and
/// `$task_uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneName {
    pub label: String,
    pub tokens: Vec<(String, String)>,
}

impl PaneName {
    /// `Refine #61 · <description>` or `Work #61 · …`, cut to what herdr
    /// keeps. A task with no number, as a finished one has after gc, shows
    /// none rather than `#0`.
    pub fn for_task(role: Role, id: u64, uuid: &str, description: &str) -> PaneName {
        let verb = match role {
            Role::Refine => "Refine",
            Role::Work => "Work",
        };
        let head = if id == 0 { verb.to_string() } else { format!("{verb} #{id}") };
        let description = text::collapse_whitespace(description);
        let mut tokens = Vec::new();
        if id != 0 {
            tokens.push(("task_id".to_string(), id.to_string()));
        }
        tokens.push(("task".to_string(), elide_to(&description, PANE_LABEL_MAX)));
        tokens.push(("task_uuid".to_string(), uuid.to_string()));
        PaneName { label: elide_to(&format!("{head} · {description}"), PANE_LABEL_MAX), tokens }
    }
}
```

Also extend the module doc's first sentence to cover the pane: `//! The names a task carries into herdr: its agents' names, the label its tabs show, and what its Claude's pane is called.`

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --lib names::`
Expected: PASS. Then run `cargo test` and expect everything to pass.

- [ ] **Step 5: Commit**

```bash
git add src/names.rs
git commit -m "feat(names): spell a task Claude's pane label and tokens

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Label a pane through the session port

**Files:**
- Modify: `src/session/herdr.rs` (the argv builders near `pane_run`, the `Process` impl, and tests)
- Modify: `src/session.rs` (the `Port` trait, a `Session::label_pane` after `rename_agent`, and tests)
- Modify: `src/session/fake.rs` (two `Port` methods after `pane_run`)

**Interfaces:**
- Consumes: `crate::names::PaneName` (Task 1).
- Produces:
  - `herdr::pane_rename(session: &str, pane: &str, label: &str) -> Vec<String>`
  - `herdr::pane_report_metadata(session: &str, pane: &str, tokens: &[(String, String)]) -> Vec<String>`
  - `herdr::METADATA_SOURCE: &str = "niri-tasks"`
  - `Port::pane_rename(&self, session: &str, pane: &str, label: &str) -> HerdrResult<()>`
  - `Port::pane_report_metadata(&self, session: &str, pane: &str, tokens: &[(String, String)]) -> HerdrResult<()>`
  - `pub fn Session::label_pane(&self, pane: &str, name: &PaneName)` (best effort, returns nothing)
  - The fake logs `pane_rename <session> <pane> <label>` and `pane_report_metadata <session> <pane> k=v k=v …`.

- [ ] **Step 1: Write the failing tests**

In `src/session/herdr.rs`'s `mod tests`:

```rust
    #[test]
    fn a_pane_is_labelled_by_its_manual_name() {
        assert_eq!(
            pane_rename("alpha", "w2:p1", "Work #61 · fix the peek"),
            vec!["herdr", "--session", "alpha", "pane", "rename", "w2:p1", "Work #61 · fix the peek"]
        );
    }

    /// One report, under niri-tasks' own source, so nothing another reporter
    /// sets is touched.
    #[test]
    fn a_panes_task_tokens_are_reported_under_niri_tasks() {
        let tokens = vec![("task_id".to_string(), "61".to_string()), ("task".to_string(), "fix = peek".to_string())];
        assert_eq!(
            pane_report_metadata("alpha", "w2:p1", &tokens),
            vec![
                "herdr", "--session", "alpha", "pane", "report-metadata", "w2:p1", "--source", "niri-tasks",
                "--token", "task_id=61", "--token", "task=fix = peek",
            ]
        );
    }
```

In `src/session.rs`'s `mod tests` (it already has `session`, `logged` and `Fake`):

```rust
    #[test]
    fn labelling_a_pane_renames_it_and_reports_its_tokens() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        s.label_pane("p1", &crate::names::PaneName::for_task(crate::names::Role::Work, 61, "u-1", "fix the peek"));
        assert_eq!(
            f.log(),
            [
                "pane_rename alpha p1 Work #61 · fix the peek",
                "pane_report_metadata alpha p1 task_id=61 task=fix the peek task_uuid=u-1",
            ]
        );
    }

    /// The Claude is running by the time its pane is labelled: a refusal, or
    /// no herdr at all, is no reason to fail the launch.
    #[test]
    fn labelling_a_pane_is_best_effort() {
        let name = crate::names::PaneName::for_task(crate::names::Role::Refine, 61, "u-1", "x");
        let (s, f) = session(Fake::stopped(0, "alpha"));
        s.label_pane("p1", &name);
        assert!(logged(&f, "pane_report_metadata alpha p1 task_id=61 task=x task_uuid=u-1"), "{:?}", f.log());
        let (s, _) = session(Fake::running(&[]).not_installed());
        s.label_pane("p1", &name);
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib session::`
Expected: a compile failure, because `pane_rename`, `pane_report_metadata` and `label_pane` are not found.

- [ ] **Step 3: Implement**

`src/session/herdr.rs`, after `pane_run`:

```rust
/// Give a pane a manual label, which herdr draws in the pane's border — on a
/// lone pane too with `ui.pane_borders = "always"`. How a task's Claude pane
/// says which task it is on. (`report-metadata --title` sets the pane's
/// terminal title instead, which no border shows.)
pub fn pane_rename(session: &str, pane: &str, label: &str) -> Vec<String> {
    cmd(session, &["pane", "rename", pane, label])
}

/// The source niri-tasks reports pane metadata under, so its tokens are its
/// own to set and clear.
pub const METADATA_SOURCE: &str = "niri-tasks";

/// Report display-only tokens on a pane, which a sidebar row can show as
/// `$name`. They stay until the pane closes.
pub fn pane_report_metadata(session: &str, pane: &str, tokens: &[(String, String)]) -> Vec<String> {
    let mut argv = cmd(session, &["pane", "report-metadata", pane, "--source", METADATA_SOURCE]);
    for (name, value) in tokens {
        argv.push("--token".to_string());
        argv.push(format!("{name}={value}"));
    }
    argv
}
```

In `impl Port for Process`, after `pane_run`:

```rust
    fn pane_rename(&self, s: &str, pane: &str, label: &str) -> HerdrResult<()> {
        self.call(&pane_rename(s, pane, label), |_| Ok(()))
    }
    fn pane_report_metadata(&self, s: &str, pane: &str, tokens: &[(String, String)]) -> HerdrResult<()> {
        self.call(&pane_report_metadata(s, pane, tokens), |_| Ok(()))
    }
```

`src/session.rs`, in `trait Port` after `pane_run`:

```rust
    fn pane_rename(&self, session: &str, pane: &str, label: &str) -> HerdrResult<()>;
    fn pane_report_metadata(&self, session: &str, pane: &str, tokens: &[(String, String)]) -> HerdrResult<()>;
```

In `impl Session`, after `rename_agent`:

```rust
    /// Say on `pane` which task it is on: its label, drawn in its border, and
    /// its task tokens. Best effort, since the Claude in it is running by
    /// now; a failure is only reported, so a terminal caller sees it.
    pub fn label_pane(&self, pane: &str, name: &crate::names::PaneName) {
        let renamed = self.port.pane_rename(&self.name, pane, &name.label);
        let reported = self.port.pane_report_metadata(&self.name, pane, &name.tokens);
        for (what, result) in [("label", renamed), ("task tokens", reported)] {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!("Could not give the pane its {what}: {e}"),
                Err(e) => eprintln!("Could not give the pane its {what}: {e:#}"),
            }
        }
    }
```

`src/session/fake.rs`, in `impl Port for Fake` after `pane_run`:

```rust
    fn pane_rename(&self, session: &str, pane: &str, label: &str) -> HerdrResult<()> {
        self.note(format!("pane_rename {session} {pane} {label}"));
        self.check()
    }
    fn pane_report_metadata(&self, session: &str, pane: &str, tokens: &[(String, String)]) -> HerdrResult<()> {
        let tokens: Vec<String> = tokens.iter().map(|(k, v)| format!("{k}={v}")).collect();
        self.note(format!("pane_report_metadata {session} {pane} {}", tokens.join(" ")));
        self.check()
    }
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --lib session::`
Expected: PASS. Then run `cargo test` and expect everything to pass.

- [ ] **Step 5: Commit**

```bash
git add src/session.rs src/session/herdr.rs src/session/fake.rs
git commit -m "feat(session): label a pane and report its task tokens

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Label the pane from Refine, Start working and the pane link

**Files:**
- Modify: `src/refine.rs` (`launch`, `launch_in` and its tests)
- Modify: `src/work.rs` (`launch`, `set_up`, `start_working`, `start_and_prompt` and their tests)
- Modify: `src/link.rs` (`link_current_pane`, a new `link_pane`, and tests)
- Modify: `src/main.rs` (`apply_status` and its one caller in `TaskCommand::Status`)

**Interfaces:**
- Consumes: `names::{PaneName, Role}` (Task 1), `Session::label_pane` (Task 2).
- Produces:
  - `refine::launch_in(session: &Session, name: &str, label: &str, workspace_label: &str, claude: &Claude, pane_name: &PaneName, text: &str) -> Result<Launched>` (private)
  - `work::start_working(session: &Session, claim: Claim, name: &str, pane: &str, t: &task::Task) -> Result<()>` (private)
  - `work::start_and_prompt(session: &Session, claim: Claim, name: &str, pane: &str, pane_name: &PaneName, text: &str) -> Result<()>` (private; Task 5 adds a `claude: &Claude` parameter)
  - `link::link_current_pane(t: &task::Task) -> Result<()>` (was `uuid: &str`)
  - `link::link_pane(session: &Session, pane: &str, t: &task::Task) -> Result<()>` (private)
  - `main::apply_status(t: &task::Task, status: task::Status) -> Result<()>`

- [ ] **Step 1: Write the failing tests**

`src/refine.rs` tests: change the helper `refine` and add a test.

```rust
    /// Refine's session steps over `fake`, as `launch` runs them.
    fn refine(fake: &Fake) -> Result<Launched> {
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        let pane = crate::names::PaneName::for_task(crate::names::Role::Refine, 61, "abc", "x");
        launch_in(&session, "task-abc", "Refine: x", "alpha", &refiner(), &pane, "/refine-task abc")
    }

    /// The new Claude's pane says which task it is on, once Claude has
    /// started in it and before the prompt; one found running is left as
    /// it is.
    #[test]
    fn refine_labels_its_new_pane_and_only_that() {
        let fake = Fake::running(&[("w1", "alpha")]);
        refine(&fake).unwrap();
        let log = fake.log();
        assert!(at(&log, "agent_start alpha task-abc") < at(&log, "pane_rename alpha"), "{log:?}");
        assert!(at(&log, "pane_rename alpha") < at(&log, "agent_prompt alpha task-abc"), "{log:?}");
        assert!(log.iter().any(|l| l.ends_with(" Refine #61 · x")), "{log:?}");
        refine(&fake).unwrap();
        assert_eq!(fake.log().iter().filter(|l| l.starts_with("pane_rename")).count(), 1);
    }
```

`src/work.rs` tests: change the three `start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", "/plan")` calls to `start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", &pane(), "/plan")`, and add:

```rust
    fn pane() -> crate::names::PaneName {
        crate::names::PaneName::for_task(crate::names::Role::Work, 61, "abc", "x")
    }

    #[test]
    fn the_working_claudes_pane_is_labelled_between_start_and_prompt() {
        let fake = Fake::running(&[("w1", "alpha")]);
        let s = session(&fake);
        start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", &pane(), "/plan").unwrap();
        let log = fake.log();
        assert!(at(&log, "agent_start alpha work-abc") < at(&log, "pane_rename alpha p1 Work #61 · x"), "{log:?}");
        assert!(at(&log, "pane_report_metadata alpha p1") < at(&log, "agent_prompt alpha work-abc"), "{log:?}");
    }
```

(If the existing `a_failed_start_lets_the_claim_go` test asserts no prompt was sent, also add `assert!(!log.iter().any(|l| l.starts_with("pane_rename")), "{log:?}");` to it: a failed start labels nothing.)

`src/link.rs` tests:

```rust
    use crate::session::fake::Fake;
    use std::path::PathBuf;

    fn task(id: u64, uuid: &str, description: &str) -> crate::task::Task {
        serde_json::from_value(serde_json::json!({"id": id, "uuid": uuid, "description": description})).unwrap()
    }

    /// A Claude started by hand takes the task's work- name and its pane the
    /// task's label.
    #[test]
    fn linking_a_hand_started_claude_names_and_labels_its_pane() {
        let fake = Fake::running(&[("w1", "alpha")]).with_agent(None, "p1", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake.clone()));
        link_pane(&s, "p1", &task(61, "7cd9fd3a-d27b", "fix the peek")).unwrap();
        let log = fake.log();
        assert!(log.contains(&"agent_rename alpha p1 work-7cd9fd3a".to_string()), "{log:?}");
        assert!(log.contains(&"pane_rename alpha p1 Work #61 · fix the peek".to_string()), "{log:?}");
    }

    /// A refine marking its task active keeps its name, and so its label.
    #[test]
    fn linking_a_refine_pane_leaves_its_label_alone() {
        let fake = Fake::running(&[("w1", "alpha")]).with_agent(Some("task-7cd9fd3a"), "p1", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake.clone()));
        link_pane(&s, "p1", &task(61, "7cd9fd3a-d27b", "fix the peek")).unwrap();
        assert!(!fake.log().iter().any(|l| l.starts_with("agent_rename") || l.starts_with("pane_rename")), "{:?}", fake.log());
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib refine:: work:: link::`
Expected: a compile failure, because `launch_in` and `start_and_prompt` take the wrong number of arguments and `link_pane` is not found.

- [ ] **Step 3: Implement**

`src/refine.rs`. In `launch`, build the pane's name and pass it:

```rust
    let claude = Claude::Refiner { settings, mod_dir };
    let label = tab_label(mode, &t.description);
    let pane = names::PaneName::for_task(names::Role::Refine, t.id, &t.uuid, &t.description);
    if launch_in(&session, &name, &label, ws.name(), &claude, &pane, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
```

Change `launch_in` to take `pane_name: &names::PaneName` after `claude` and label the pane after the start:

```rust
fn launch_in(session: &Session, name: &str, label: &str, workspace_label: &str, claude: &Claude, pane_name: &names::PaneName, text: &str) -> Result<Launched> {
    // …unchanged up to the start…
    if let Err(e) = session.start_claude(claim, name, &tab.pane, claude) {
        // …unchanged…
    }
    session.label_pane(&tab.pane, pane_name);
    session.prompt(name, text)?;
    Ok(Launched::Started)
}
```

Add a line to `launch_in`'s doc: `Once Claude has started, its pane is labelled with the task (`pane_name`); a Claude found running is left as it is.`

`src/work.rs`. Add `use crate::names::{PaneName, Role};` (or path through `names::`). Change the two callers: in `launch` use `return start_working(&session, claim, &name, &tab.pane, t);`, and in `set_up` use `start_working(&session, claim, &name, &pane, &t)`.

```rust
/// Start the working Claude in `pane`, label the pane with the task, hand
/// it the task to plan, and mark the task active, as Update status → Active
/// would. Other active tasks on the workspace stay active: they are other
/// worktrees' agents at work.
fn start_working(session: &Session, claim: Claim, name: &str, pane: &str, t: &task::Task) -> Result<()> {
    let pane_name = names::PaneName::for_task(names::Role::Work, t.id, &t.uuid, &t.description);
    start_and_prompt(session, claim, name, pane, &pane_name, &plan_prompt(&t.uuid))?;
    task::set_active(&t.uuid)
}

/// Start the working Claude in `pane` under `claim`, label the pane
/// `pane_name`, then send `text`. The start lets the claim go once herdr has
/// the agent, before a blocked start's question or the prompt's retries: a
/// second launch waiting on it then finds the agent and focuses it.
fn start_and_prompt(session: &Session, claim: Claim, name: &str, pane: &str, pane_name: &names::PaneName, text: &str) -> Result<()> {
    session.start_claude(claim, name, pane, &Claude::Worker)?;
    session.label_pane(pane, pane_name);
    session.prompt(name, text)
}
```

`src/link.rs`. Split the herdr part out of `link_current_pane` so it can run against the fake:

```rust
pub fn link_current_pane(t: &crate::task::Task) -> Result<()> {
    let Some(CurrentPane { session, pane: Some(pane), .. }) = current_pane() else {
        return Ok(());
    };
    link_pane(&Session::named(&session), &pane, t)
}

/// [`link_current_pane`]'s steps in `session`'s `pane`: name its agent after
/// `t` as [`pane_link_name`] decides and, when it did, label the pane with
/// the task, as Start working labels its own.
fn link_pane(session: &Session, pane: &str, t: &crate::task::Task) -> Result<()> {
    // Any failure to ask, herdr not installed included, is nothing to link.
    let Ok(Some(agent)) = session.agent(pane) else {
        return Ok(());
    };
    let Some(name) = pane_link_name(agent.name.as_deref(), &t.uuid) else {
        return Ok(());
    };
    session
        .rename_agent(pane, &name)
        .with_context(|| format!("The task is active, but this pane's agent could not be named {name}"))?;
    session.label_pane(pane, &names::PaneName::for_task(names::Role::Work, t.id, &t.uuid, &t.description));
    Ok(())
}
```

Update `link_current_pane`'s doc to say that it also labels the pane. Also update the module doc's sentence about a Claude started by hand: "…gets the same `work-` name, and its pane the task's label, when it marks the task active."

`src/main.rs`:

```rust
            let t = task::get(&uuid)?.context("task not found")?;
            apply_status(&t, state)?;
```

```rust
fn apply_status(t: &task::Task, status: task::Status) -> Result<()> {
    task::set_status(&t.uuid, status)?;
    if status == task::Status::Active {
        if let Err(e) = link::link_current_pane(t) {
            eprintln!("{e:#}");
        }
    }
    notify::tasks(&format!("{}: {}", status.label(), t.description));
    Ok(())
}
```

(Its doc says the pane's agent is named after the task. Add "and the pane labelled with it".)

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test`
Expected: PASS, including the existing refine and work claim-order tests.

- [ ] **Step 5: Commit**

```bash
git add src/refine.rs src/work.rs src/link.rs src/main.rs
git commit -m "feat: label a task Claude's herdr pane with its task

Refine, Start working and the pane link name the pane they take
\"Refine #<id> · <description>\" or \"Work #<id> · …\", drawn in its
border, and report its task_id, task and task_uuid tokens.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `niritasks task status-line <uuid>`

**Files:**
- Create: `src/status_line.rs`
- Modify: `src/lib.rs` (`pub mod status_line;`, in alphabetical order after `speak`)
- Modify: `src/main.rs` (`TaskCommand::StatusLine`, its dispatch, the `use niri_tasks::{…}` list, and a parse test)

**Interfaces:**
- Consumes: `task::{get, now_secs, stamp_secs, Task}` (exist), `text::collapse_whitespace`, `names::uuid8`, `work::sh_quote` (exist).
- Produces (all `pub` in `status_line`):
  - `fn state_word(t: &task::Task, now: i64) -> &'static str`
  - `fn line(t: &task::Task, now: i64, branch: Option<&str>) -> String`
  - `fn cwd_from(input: &str) -> Option<PathBuf>`
  - `fn run(uuid: &str)` (never fails, always prints one line)
  - `fn command(exe: &Path, uuid: &str) -> String`
  - `fn setting(command: &str) -> serde_json::Value`
  - `fn worker_settings(command: &str) -> String`

- [ ] **Step 1: Write the failing tests**

Create `src/status_line.rs` with only the tests first (or write the whole file and confirm the tests fail against stub bodies). The tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn task(v: serde_json::Value) -> task::Task {
        serde_json::from_value(v).unwrap()
    }

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn the_line_is_number_description_state_and_branch() {
        let t = task(serde_json::json!({"id": 61, "uuid": "u", "description": "feat: Name  the task", "status": "pending", "start": "20261012T000000Z"}));
        assert_eq!(line(&t, NOW, Some("task/feat-x-7b4f4c5c")), "#61 · feat: Name the task · Active · task/feat-x-7b4f4c5c");
    }

    #[test]
    fn no_branch_and_no_number_leave_their_place_out() {
        let t = task(serde_json::json!({"id": 0, "uuid": "u", "description": "x", "status": "completed"}));
        assert_eq!(line(&t, NOW, None), "x · Completed");
        assert_eq!(line(&t, NOW, Some("")), "x · Completed");
    }

    /// The Update status menu's words; a task never started is Pending,
    /// Taskwarrior's own, not the menu's Stopped.
    #[test]
    fn the_state_reads_as_the_status_menu_does() {
        let state = |v: serde_json::Value| state_word(&task(v), NOW);
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending"})), "Pending");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "start": "20261012T000000Z"})), "Active");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "wait": "99991229T130000Z"})), "Waiting");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "wait": "20001229T130000Z"})), "Pending", "a wait gone by");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "completed"})), "Completed");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "deleted"})), "Deleted");
    }

    /// Claude Code's status-line input: the workspace's folder first, then
    /// the session's cwd.
    #[test]
    fn the_folder_comes_from_claude_codes_input() {
        assert_eq!(cwd_from(r#"{"cwd":"/a","workspace":{"current_dir":"/b"}}"#), Some(PathBuf::from("/b")));
        assert_eq!(cwd_from(r#"{"cwd":"/a"}"#), Some(PathBuf::from("/a")));
        assert_eq!(cwd_from("not json"), None);
    }

    #[test]
    fn the_command_quotes_the_binary_and_the_uuid() {
        assert_eq!(
            command(Path::new("/home/x/.cargo/bin/niritasks"), "7b4f4c5c-185a"),
            "'/home/x/.cargo/bin/niritasks' task status-line '7b4f4c5c-185a'"
        );
    }

    #[test]
    fn the_worker_settings_hold_the_status_line_alone() {
        let v: serde_json::Value = serde_json::from_str(&worker_settings("niritasks task status-line u")).unwrap();
        assert_eq!(v, serde_json::json!({"statusLine": {"type": "command", "command": "niritasks task status-line u", "padding": 0}}));
    }
}
```

In `src/main.rs`'s `mod tests`:

```rust
    #[test]
    fn status_line_takes_a_uuid() {
        assert!(Cli::try_parse_from(["niritasks", "task", "status-line", "7b4f4c5c"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "status-line"]).is_err());
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test status_line`
Expected: a compile failure, because the items and the subcommand are not found.

- [ ] **Step 3: Implement**

`src/status_line.rs` (above the tests):

```rust
//! The status line a task's Claude shows under its prompt:
//! `#61 · <description> · Active · task/<slug>-<uuid8>`. Refine and Start
//! working pass it in their launch `--settings`, so the user's own Claude
//! settings are never touched. Claude Code runs `niritasks task status-line
//! <uuid>` on each refresh, so the number, description and state are read
//! again every time and never go stale, unlike the pane's label.

use crate::{names, task, text};
use serde_json::{json, Value};
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The task's state in the Update status menu's words (`task::Status`), and
/// `Pending` for a task not started, Taskwarrior's own word: the menu's
/// `Stopped` would say it had been started.
pub fn state_word(t: &task::Task, now: i64) -> &'static str {
    let parked = t.wait.as_deref().and_then(task::stamp_secs).is_some_and(|w| w > now);
    match t.status.as_str() {
        "completed" => "Completed",
        "deleted" => "Deleted",
        _ if parked => "Waiting",
        _ if t.is_active() => "Active",
        _ => "Pending",
    }
}

/// `#id · description · state · branch`, leaving out a number a finished
/// task has lost to gc and a branch git did not name.
pub fn line(t: &task::Task, now: i64, branch: Option<&str>) -> String {
    let mut parts = Vec::new();
    if t.id != 0 {
        parts.push(format!("#{}", t.id));
    }
    parts.push(text::collapse_whitespace(&t.description));
    parts.push(state_word(t, now).to_string());
    if let Some(b) = branch.filter(|b| !b.is_empty()) {
        parts.push(b.to_string());
    }
    parts.join(" · ")
}

/// The folder Claude is working in, from the JSON Claude Code hands a
/// status-line command on stdin: `workspace.current_dir`, else `cwd`.
pub fn cwd_from(input: &str) -> Option<PathBuf> {
    let v: Value = serde_json::from_str(input).ok()?;
    v["workspace"]["current_dir"].as_str().or(v["cwd"].as_str()).map(PathBuf::from)
}

/// The branch checked out in `dir`, or None: detached, no repository, or no
/// git.
fn branch_in(dir: &Path) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(["branch", "--show-current"]).output().ok()?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !branch.is_empty()).then_some(branch)
}

/// Print `uuid`'s status line. Never fails: Claude Code runs it on every
/// refresh, so a failure prints a line saying so instead of reaching
/// `main`'s desktop notification. Reads Claude Code's JSON from stdin, but
/// not from a terminal, so it can be run by hand too.
pub fn run(uuid: &str) {
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        let _ = std::io::stdin().read_to_string(&mut input);
    }
    let dir = cwd_from(&input).or_else(|| std::env::current_dir().ok());
    let branch = dir.as_deref().and_then(branch_in);
    match task::get(uuid) {
        Ok(Some(t)) => println!("{}", line(&t, task::now_secs(), branch.as_deref())),
        Ok(None) => println!("task {}: not found", names::uuid8(uuid)),
        Err(_) => println!("task {}: could not read it", names::uuid8(uuid)),
    }
}

/// The shell command Claude Code runs for `uuid`'s status line, with `exe`
/// the `niritasks` binary: absolute, since a Claude's `$PATH` need not have it.
pub fn command(exe: &Path, uuid: &str) -> String {
    format!(
        "{} task status-line {}",
        crate::work::sh_quote(&exe.display().to_string()),
        crate::work::sh_quote(uuid)
    )
}

/// Claude Code's `statusLine` setting running `command`.
pub fn setting(command: &str) -> Value {
    json!({ "type": "command", "command": command, "padding": 0 })
}

/// The `--settings` JSON a working Claude starts with: its status line and
/// nothing else, so it otherwise runs as the user runs Claude.
pub fn worker_settings(command: &str) -> String {
    json!({ "statusLine": setting(command) }).to_string()
}
```

`src/lib.rs`: add `pub mod status_line;` after `pub mod speak;`.

`src/main.rs`: add `status_line` to the `use niri_tasks::{…}` list. Add to `enum TaskCommand`, after `Speak`:

```rust
    /// Internal: print a task's status line, `#id · description · state ·
    /// branch`, for the Claude Code sessions Refine and Start working start
    ///
    /// Claude Code runs it on each refresh, through the launch settings
    /// those sessions get. It reads Claude Code's JSON on stdin for the
    /// folder whose branch it shows, and it never fails: a task it cannot
    /// read prints a line saying so.
    StatusLine {
        /// The task's uuid
        uuid: String,
    },
```

and to `task_command`'s match:

```rust
        // Claude Code runs this on every refresh: never an error, so never
        // main's desktop notification.
        TaskCommand::StatusLine { uuid } => status_line::run(&uuid),
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test`
Expected: PASS. Then check by hand: `cargo run -q -- task status-line 7b4f4c5c-185a-479e-8ab7-c1a6e5dad4b0 </dev/null`. Expect something like `#61 · feat: Name the task on its Claude's herdr pane · Active · task/feat-name-the-task-on-its-claude-s-7b4f4c5c` (the state depends on the task). Then `echo '{"workspace":{"current_dir":"/home/paul/Projects/niri-tasks"}}' | cargo run -q -- task status-line 7b4f4c5c-185a-479e-8ab7-c1a6e5dad4b0` should show `main` as the branch.

- [ ] **Step 5: Commit**

```bash
git add src/status_line.rs src/lib.rs src/main.rs
git commit -m "feat: add task status-line for a task Claude's status line

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Pass the status line in both Claudes' launch settings

**Files:**
- Modify: `src/session.rs` (`Claude::Worker` becomes `Worker { settings: String }`, and the three `Claude::Worker` uses in its tests)
- Modify: `src/session/herdr.rs` (`agent_start_claude` takes `settings`, the `Process::agent_start` match, and the `a_working_claude_starts_with_no_extra_flags` test)
- Modify: `src/session/fake.rs` (`Claude::Worker { .. }`)
- Modify: `src/work.rs` (`start_working` builds the settings, and `start_and_prompt` takes `claude: &Claude`)
- Modify: `src/refine.rs` (`session_settings` gains `status_line: Option<&str>`, plus `launch`, `run_unattended`'s two calls and the tests' calls)

**Interfaces:**
- Consumes: `status_line::{command, setting, worker_settings}` (Task 4).
- Produces:
  - `Claude::Worker { settings: String }` (the `--settings` JSON)
  - `herdr::agent_start_claude(session: &str, name: &str, pane: &str, settings: &str) -> Vec<String>`
  - `refine::session_settings(project, task_data, hidden, uuid, reports, mode, unattended, status_line: Option<&str>) -> String`
  - `work::start_and_prompt(session, claim, name, pane, claude: &Claude, pane_name: &PaneName, text) -> Result<()>`

- [ ] **Step 1: Write the failing tests**

`src/session/herdr.rs`, replace `a_working_claude_starts_with_no_extra_flags`:

```rust
    /// The working Claude runs as the user runs Claude: the only flag of
    /// ours is its status line, in settings of its own.
    #[test]
    fn a_working_claude_starts_with_only_its_status_line() {
        assert_eq!(
            agent_start_claude("alpha", "work-1234abcd", "w2:p1", r#"{"statusLine":{}}"#),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "work-1234abcd",
                "--kind", "claude", "--pane", "w2:p1", "--timeout", "60000", "--",
                "--settings", r#"{"statusLine":{}}"#,
            ]
        );
    }
```

`src/refine.rs` tests: add `, None` as the last argument to every existing `session_settings(…)` call in the tests (`grep -n "session_settings(" src/refine.rs` lists them), and add:

```rust
    /// A tab's refine shows the task in its status line; an unattended run
    /// has no screen to show it on.
    #[test]
    fn the_status_line_rides_in_the_settings_only_when_given() {
        let with = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"), Mode::Quick, false, Some("niritasks task status-line u"));
        let v: serde_json::Value = serde_json::from_str(&with).unwrap();
        assert_eq!(v["statusLine"]["command"], "niritasks task status-line u");
        assert_eq!(v["sandbox"]["enabled"], true, "the fence is still there");
        let without = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"), Mode::Quick, true, None);
        let v: serde_json::Value = serde_json::from_str(&without).unwrap();
        assert!(v.get("statusLine").is_none(), "{v}");
    }
```

`src/session.rs` tests: replace each `&Claude::Worker` with `&Claude::Worker { settings: "{}".into() }`. `src/work.rs` tests: add a `&Claude::Worker { settings: "{}".into() }` argument after `"p1"` in every `start_and_prompt` call.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test`
Expected: a compile failure on the changed signatures.

- [ ] **Step 3: Implement**

`src/session.rs`:

```rust
    /// Start working's: the user's own defaults, since it is meant to do the
    /// work, with only its status line added.
    Worker {
        /// The settings JSON passed to `claude --settings`: the task's
        /// status line (`status_line::worker_settings`).
        settings: String,
    },
```

`src/session/herdr.rs`:

```rust
/// Claude as a named agent in `pane`, with no flags of ours but `settings`,
/// which hold only the task's status line: this session is meant to do the
/// work, so it runs in whatever mode the user runs Claude in.
pub fn agent_start_claude(session: &str, name: &str, pane: &str, settings: &str) -> Vec<String> {
    cmd(
        session,
        &["agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000", "--", "--settings", settings],
    )
}
```

and in `Process::agent_start`: `Claude::Worker { settings } => agent_start_claude(s, name, pane, settings),`.

`src/session/fake.rs`: `Claude::Worker { .. } => "worker",`.

`src/work.rs`:

```rust
fn start_working(session: &Session, claim: Claim, name: &str, pane: &str, t: &task::Task) -> Result<()> {
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let claude = Claude::Worker {
        settings: crate::status_line::worker_settings(&crate::status_line::command(&exe, &t.uuid)),
    };
    let pane_name = names::PaneName::for_task(names::Role::Work, t.id, &t.uuid, &t.description);
    start_and_prompt(session, claim, name, pane, &claude, &pane_name, &plan_prompt(&t.uuid))?;
    task::set_active(&t.uuid)
}

fn start_and_prompt(session: &Session, claim: Claim, name: &str, pane: &str, claude: &Claude, pane_name: &names::PaneName, text: &str) -> Result<()> {
    session.start_claude(claim, name, pane, claude)?;
    session.label_pane(pane, pane_name);
    session.prompt(name, text)
}
```

(Extend `start_working`'s doc: "…with the task in its status line…".)

`src/refine.rs`, `session_settings`: add the parameter, and set the key after building the JSON:

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, mode: Mode, unattended: bool, status_line: Option<&str>) -> String {
    // …deny and allow unchanged…
    let mut settings = serde_json::json!({ /* …unchanged… */ });
    if let Some(command) = status_line {
        settings["statusLine"] = crate::status_line::setting(command);
    }
    settings.to_string()
}
```

Add a sentence to its doc: `` `status_line`, when given, is the command for the task's status line (`status_line::command`); a tab's refine shows it, an unattended run has no screen to. `` If clippy flags `too_many_arguments` (8 parameters), add `#[allow(clippy::too_many_arguments)]` on the function, as the repo does elsewhere (`grep -rn too_many_arguments src` first; follow what's there).

In `launch`:

```rust
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let settings = session_settings(
        session.dir(),
        &task::data_location()?,
        &hidden,
        &t.uuid,
        &reports_dir(&dirs),
        mode,
        false,
        Some(&crate::status_line::command(&exe, &t.uuid)),
    );
```

In `run_unattended`'s two `session_settings(…)` calls, add `None` as the last argument.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test` and `cargo clippy --all-targets`
Expected: tests PASS, and no new clippy warnings.

- [ ] **Step 5: Commit**

```bash
git add src/session.rs src/session/herdr.rs src/session/fake.rs src/work.rs src/refine.rs
git commit -m "feat: show the task in Refine and Start working's status line

Both Claudes get niritasks task status-line through their launch
--settings; the user's own Claude settings are left alone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Docs, the user's config, and a live check

**Files:**
- Modify: `README.md` (the herdr paragraph near line 205, the paragraph on linking a hand-started Claude near line 231, and the CLI list near line 360)
- Modify: `CONTEXT.md` (**Session**, near line 31)
- Modify: `llms.txt` (rule 4, near line 12)

- [ ] **Step 1: README**

After the paragraph that ends "…asking in a herdr tab the first time a repo's hooks need approving.", add:

```markdown
A Refine or Start working Claude's pane says which task it is on: its border
reads `Refine #61 · <description>` or `Work #61 · <description>`, and Claude's
status line, under its prompt, `#61 · <description> · <state> · <branch>`,
read afresh on each refresh. herdr draws a border, and the label in it, only
around split panes unless you add this to `~/.config/herdr/config.toml` and
run `herdr server reload-config` (niri-tasks never edits that file):

    [ui]
    pane_borders = "always"

The pane also carries `task_id`, `task` and `task_uuid` tokens, which an Agent
row in `[ui.sidebar.agents]` can show as `$task_id`, `$task` and `$task_uuid`;
the default sidebar shows none of them.
```

In the hand-started Claude paragraph, change "its agent is named `work-<uuid8>`, …" to also say "and its pane labelled `Work #<id> · <description>`".

In the CLI list, after the `task start <uuid> --here --workspace <name>` lines, add:

```
niritasks task status-line <uuid>  # internal: the status line Refine's and Start working's Claude show,
                                   #   #id · description · state · branch, run by Claude Code on each refresh
```

- [ ] **Step 2: CONTEXT.md and llms.txt**

In CONTEXT.md's **Session** entry, after "…open a task's tab or agent in it,", add: "label that agent's pane with the task (`Refine #61 · …` or `Work #61 · …`, in its border),".

In llms.txt rule 4, change "`active` also links the herdr pane you run it in to the task, so its card's Go to list can find you" to "`active` also links the herdr pane you run it in to the task, naming its agent and labelling its border, so its card's Go to list can find you".

- [ ] **Step 3: Run the whole suite**

Run: `cargo test` and `cargo clippy --all-targets`
Expected: PASS, with no new warnings.

- [ ] **Step 4: Commit**

```bash
git add README.md CONTEXT.md llms.txt
git commit -m "docs: say how a task Claude's pane names its task

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Check the user's herdr config**

Run: `grep -n "pane_borders" ~/.config/herdr/config.toml`
Expected: `pane_borders = "always"` under `[ui]`. If it's missing, ask the user to add it and run `herdr server reload-config`. Don't edit the file yourself. The spec's "Done when" needs it there.

- [ ] **Step 6: Live check (the user looks)**

A live launch only picks up the new code once it's installed, and Start working from the panel runs the installed `niritasks`. So this check runs **after the branch lands** (finish-worktree installs it). Until then, check what you can from this worktree's build:

1. `cargo build`. From a pane of this herdr session, run `./target/debug/niritasks task refine <uuid of a throwaway pending task>`, with a task filed for the purpose and deleted afterwards via `niritasks task status <uuid> deleted --yes`.
2. Run `herdr pane get <the new pane>`. Its `label` should be `Refine #<id> · <description>` and its `tokens` should hold `task_id`, `task` and `task_uuid`.
3. Ask the user to confirm three things: the pane's border shows the label; Claude's status line shows `#id · description · Pending · main`; the sidebar looks exactly as before.
4. Close that tab, and delete the throwaway task.

Record what was confirmed in the hand-off, and say plainly what wasn't.
