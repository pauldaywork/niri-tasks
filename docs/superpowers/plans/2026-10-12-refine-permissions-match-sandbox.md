# Refine's Permissions Match Its Sandbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine, Grill me, Report and the unattended run never prompt for, or auto-deny, an action the Bash sandbox already allows, and an unattended run's log and notification name every action that was denied.

**Architecture:** The permission rules in `refine::session_settings` (`src/refine.rs`) state the fence the OS enforces: a bare `Bash` allow (the sandbox is the fence, and `allowUnsandboxedCommands: false` keeps the unsandboxed retry shut), `Read(//**)` under the existing deny rules, `WebSearch` and `WebFetch`. The tab and the headless run share these settings, so a single change covers both. The unattended run switches to `claude -p --output-format json`, parses `result` and `permission_denials`, writes both to its log and names any denials in the notification. A diagnosis run comes first, so the change answers the measured cause.

**Tech Stack:** Rust (`serde_json`, `anyhow`, `std::process`), Claude Code 2.1.296 (`claude -p`, `--settings`, `--output-format json|stream-json`), Taskwarrior 3.5, herdr, `jq`.

**Spec:** Taskwarrior task `af9e53fc-76e4-4283-9a44-28f6076c5759` (description and notes; read it with `task rc.json.array=on af9e53fc export`). No separate spec file.

## Global Constraints

- Stay in `--permission-mode default` with the sandbox in auto-allow. Not auto mode, not plan mode.
- Fix at the permission layer in `session_settings`, tab and headless alike. No `tool.check` or `PermissionRequest` hook in the mod, and no skill sentences that tell the model which shell shapes to avoid.
- Do not change the sandbox fence: `hidden` paths, `denyWrite`, `allowWrite`, `allowUnsandboxedCommands: false` and `failIfUnavailable: true` stay as they are.
- Out of scope: grilling unattended, the panel, and a timeout on the headless runs.
- Already landed before this plan (commits `9bd3459`, `b2bb4d2`): `Read(//**)` in the allow list, deny rules for every hidden path, and the removal of `skill_dirs`. Do not redo them.
- `82ff74c6` is already completed. `a67de29b` is still pending and is closed in Task 6.
- Task descriptions start with a Conventional Commits type. Commits follow Conventional Commits, mostly `fix(refine): …`.
- Run all commands from the worktree `/home/paul/.worktrees/niri-tasks/task-fix-make-refine-s-permissions-match-its-af9e53fc` unless a step says otherwise.
- Diagnosis artefacts go in `~/.local/share/niri-tasks/reviews/af9e53fc-diagnosis/`, never `/tmp`.
- A refine session loads skills from `~/Projects/niri-tasks`, never from a worktree. Skill edits (Task 4) can only be checked end to end after the branch lands (Task 6).

## File Map

- `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`: a dated addendum with the diagnosis finding and the new rule (Task 1, extended in Task 2).
- `src/refine.rs`
  - `session_settings`: the allow list and its doc comment (Task 2).
  - `claude_unattended_argv`: adds `--output-format json` (Task 3).
  - `run_claude`, `run_unattended`, `unattended_steps`: parse and log denials, then notify (Task 3).
  - Tests at the bottom of the file.
- `src/session/herdr.rs`: the doc comment on `agent_start_claude_refiner` (Task 2).
- `.claude/skills/refine-task/SKILL.md` and `.claude/skills/report-task/SKILL.md`: remove the shell-shape rules (Task 4).
- `README.md` (the Add & all paragraph, around line 406) and `llms.txt` (the `task refine --unattended` row, line 44): say that denials are logged and named (Task 5).

---

### Task 1: Diagnose the headless denials

This task changes no code. It runs the exact headless argv on a throwaway task with `stream-json` output and records what the run reports. **Its finding gates Task 2.**

**Files:**
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (add a dated addendum after the last `*2026-10-…:*` paragraph at the top, before the record's first `##` heading)

**Interfaces:**
- Consumes: nothing.
- Produces: one of three verdicts, written in the addendum and reported to the controller:
  - **A**: the sandbox is active under `-p` and Bash is denied by Claude Code's command-shape checks (variables, quoted text, multiple operations, a directory outside the working directories) despite `autoAllowBashIfSandboxed`. Task 2 proceeds as written.
  - **B**: the sandbox is *not* active under `-p`. **Stop.** Report to the user before any later task. A bare `Bash` allow would then run commands unfenced, and fixing the sandbox comes first.
  - **C**: anything else. Stop and report to the user with the evidence.

- [ ] **Step 1: Build the worktree's binary and set up the diagnosis folder**

```bash
cargo build
D=~/.local/share/niri-tasks/reviews/af9e53fc-diagnosis
mkdir -p "$D/bin"
command -v claude   # note the real path, normally /home/paul/.local/bin/claude
```

- [ ] **Step 2: Write a `claude` shim that records the argv and streams JSON**

`run_claude` executes `claude` from `PATH`. The shim records each argv (NUL-separated, because the settings JSON and the system prompt contain spaces), then runs the real `claude` with stream-json output added. Write `$D/bin/claude`:

```sh
#!/bin/sh
D="$HOME/.local/share/niri-tasks/reviews/af9e53fc-diagnosis"
n=$(ls "$D"/argv-*.bin 2>/dev/null | wc -l)
printf '%s\0' "$@" > "$D/argv-$n.bin"
exec /home/paul/.local/bin/claude "$@" --output-format stream-json --verbose
```

```bash
chmod +x "$D/bin/claude"
```

If Step 1 printed a different path for `claude`, use it in the `exec` line.

- [ ] **Step 3: Add a throwaway task that provokes the shapes that were denied**

Run this from a pane in the niri-tasks herdr session, so the task gets the `+niri_tasks` tag:

```bash
./target/debug/niritasks task add 'docs: Describe how ~/.task and the unattended logs are laid out'
U=$(task rc.json.array=on +niri_tasks status:pending description~'unattended logs are laid out' export | jq -r '.[0].uuid')
./target/debug/niritasks task note "$U" 'Context: list ~/.task and ~/.local/share/niri-tasks/unattended, count the lines of each src/*.rs file in one loop, and read a log under ~/.worktrees'
echo "$U"
```

- [ ] **Step 4: Run the unattended refine through the shim**

```bash
PATH="$D/bin:$PATH" ./target/debug/niritasks task refine "$U" --unattended
```

Expected: the command takes minutes and then ends with a notification, or with an error that names a log under `~/.local/share/niri-tasks/unattended/`. Either outcome is fine: the log is the evidence.

If the run fails with the sandbox being unavailable because the shell you run it from is itself sandboxed (nested bwrap), ask the user to run the Step 4 command with `! ` in their prompt.

- [ ] **Step 5: Read the init event and the denials**

```bash
LOG=$(ls -t ~/.local/share/niri-tasks/unattended/"${U:0:8}"-*.log | head -1)
grep '^{' "$LOG" | jq -c 'select(.type=="system" and .subtype=="init") | {permissionMode, cwd, tools: (.tools|length), keys: keys}'
grep '^{' "$LOG" | jq -c 'select(.type=="system" and .subtype=="init")' | grep -o '"[a-zA-Z]*[Ss]andbox[a-zA-Z]*":[^,}]*' || echo "init names no sandbox field"
grep '^{' "$LOG" | jq -c 'select(.type=="result") | {subtype, is_error, denials: .permission_denials}'
grep '^{' "$LOG" | jq -r 'select(.type=="user") | .message.content[]? | select(.type=="tool_result" and (.is_error==true)) | .content | tostring' | head -40
```

Note `permissionMode` (expected `default`), any sandbox field in init, and every entry in `permission_denials` with its `tool_name` and `tool_input`. Also note the error text of each failed tool result, for example `A variable in this command can't be checked before it runs` or `contains multiple operations`.

- [ ] **Step 6: Probe whether the sandbox is active under `-p`**

The init event may not report the sandbox. Re-run the recorded refine argv with only the prompt changed, asking for a read of a path the sandbox hides from Bash. A `/run/user` listing that succeeds means the sandbox is not in effect. The command also has the `;` and `$?` shapes that were denied:

```bash
cd ~/Projects/niri-tasks
mapfile -d '' -t A < "$D/argv-0.bin"
A[1]='Run exactly this one Bash command, unchanged: ls /run/user; echo rc=$?   Then quote its full output, or the exact refusal if it did not run. Do nothing else.'
/home/paul/.local/bin/claude "${A[@]}" --output-format stream-json --verbose > "$D/probe.jsonl" 2>&1
jq -c 'select(.type=="result") | {result, denials: .permission_denials}' "$D/probe.jsonl"
cd -
```

`A[0]` is `-p` and `A[1]` the prompt, because the shim records `"$@"` without argv0. Read the result:
- `ls: cannot access '/run/user'` or `No such file or directory` (hidden): **the sandbox is active.**
- A listing such as `1000`: **the sandbox is not active (verdict B).**
- A denial in `permission_denials` for this exact command: **the shape check denied it.**

- [ ] **Step 7: Decide the verdict and stop if it is B or C**

Compare Steps 5–6 with the verdicts under **Interfaces**. On B or C, stop here and report the evidence (the jq outputs and the log path) to the controller or user. Do not continue to Task 2.

- [ ] **Step 8: Remove the throwaway task**

The user asked for it to be throwaway, so deleting it is covered:

```bash
./target/debug/niritasks task status "$U" deleted --yes
```

Leave the report it may have written in the reviews folder: it is harmless, and it shows what the run produced.

- [ ] **Step 9: Write the addendum**

Add a dated paragraph to `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, after the last existing `*2026-10-…:*` addendum at the top. Use the measured facts in place of the bracketed parts. For verdict A it reads like this:

```markdown
*2026-10-12:* a headless refine (`claude -p … --permission-prompts none`,
Claude Code 2.1.296) runs in `default` permission mode with the Bash
sandbox active: a probe of `ls /run/user` under the same settings got
[the exact error] from the sandbox. Its denials did not come from the
sandbox. [N] Bash commands were refused by Claude Code's own
command-shape checks ([quote two or three of the messages]), which
`autoAllowBashIfSandboxed` does not override when there is no one to
ask. A bare `Bash` allow rule does, and the sandbox stays the fence, so
`session_settings` now allows `Bash` in place of `Bash(task *)`, for
the tab and the headless run alike. `allowUnsandboxedCommands: false`
keeps a blocked command from being retried outside the sandbox. The
residual risk in Consequences is unchanged: any sandboxed command could
already run unasked under auto-allow, and the write to the task data
was never something a narrower rule could close.
```

- [ ] **Step 10: Commit**

```bash
git add docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "docs(adr): record why headless refine runs denied Bash" -m "A diagnosis run of the unattended argv with stream-json output shows
the sandbox active under -p and the denials coming from Claude Code's
command-shape checks, which auto-allow does not cover headless.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Adjust the body to the measured finding.

---

### Task 2: Allow Bash bare, under the sandbox

**Only after Task 1's verdict is A.**

**Files:**
- Modify: `src/refine.rs`: `session_settings` (the `let allow = …` line, about line 149) and its doc comment (lines 112–145)
- Modify: `src/session/herdr.rs`: the doc comment on `agent_start_claude_refiner` (the "What keeps it from prompting" paragraph, about lines 113–116)
- Test: `src/refine.rs`, test `the_session_may_search_the_web_but_not_read_credentials` (about line 748) and its doc comment

**Interfaces:**
- Consumes: Task 1's verdict A.
- Produces: `session_settings(...)` with the same signature, whose `permissions.allow` is exactly `["WebSearch", "WebFetch", "Bash", "Read(//**)"]`. `permissions.deny` and `sandbox` do not change.

- [ ] **Step 1: Change the test first**

Replace the test and its doc comment. The doc comment starts at the `///` lines just above `#[test] fn the_session_may_search_the_web_but_not_read_credentials`. Read back to the previous test's closing `}` to find where it starts.

```rust
    /// Every shell command runs unasked, of any shape: a bare `Bash` rule,
    /// because the sandbox is the fence, not the rules. Claude Code's
    /// command-shape checks (variables, quoting, `;` chains) otherwise
    /// deny loops and `$( )` when there is no one to ask, even with
    /// auto-allow (ADR 0002, 2026-10-12). No scoped `Bash(…)` rule is left
    /// beside it. Web search and fetch write nothing. The Read tool may
    /// read anywhere (`//` is Claude Code's absolute form; one `/` would be
    /// the project's), as sandboxed Bash can. Credentials still cannot be
    /// read through it, the one reader the Bash sandbox does not cover: a
    /// deny rule wins over an allow. The Read tool is also denied every
    /// path the sandbox hides from Bash, so it reads no further than Bash.
    #[test]
    fn the_session_may_run_any_sandboxed_command_but_not_read_credentials() {
        let hidden = [PathBuf::from("/run"), PathBuf::from("/home/x/.config/herdr")];
        let json = session_settings(Path::new("/p"), Path::new("/t"), &hidden, "u", Path::new("/r"), Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash", "Read(//**)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert_eq!(allow.len(), 4, "nothing else runs unasked");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.starts_with("Bash("))), "no scoped Bash rule beside the bare one");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("skills"))), "one rule for every folder, not a list");
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert!(deny.iter().any(|d| d == "Read(//run/**)"));
        assert!(deny.iter().any(|d| d == "Read(//home/x/.config/herdr/**)"));
        assert_eq!(deny.len(), CREDENTIALS.len() + hidden.len());
        let sandbox = &v["sandbox"];
        assert_eq!(sandbox["enabled"], true);
        assert_eq!(sandbox["allowUnsandboxedCommands"], false, "a blocked command is never retried unfenced");
        assert_eq!(sandbox["failIfUnavailable"], true, "no session at all without the sandbox");
    }
```

- [ ] **Step 2: Run it and check that it fails**

Run: `cargo test --lib refine::tests::the_session_may_run_any_sandboxed_command_but_not_read_credentials`
Expected: FAIL, with the panic message `Bash allowed`.

- [ ] **Step 3: Change the allow list**

In `src/refine.rs`, `session_settings`:

```rust
    let allow = ["WebSearch", "WebFetch", "Bash", "Read(//**)"];
```

- [ ] **Step 4: Rewrite the doc comment's allow paragraph**

In the doc comment above `session_settings`, replace the paragraph from `/// Allowed unasked on top: web search and fetch, which write nothing; and` through `/// already. One rule for every project, not a list of` / `/// folders.` with:

```rust
/// Allowed unasked: every Bash command (`Bash`, bare), since the sandbox
/// is the fence and the rules only say so. Auto-allow alone leaves
/// Claude Code's own command-shape checks in place (a variable, quoted
/// text, a `;` chain), and with no one to ask, as in an unattended run,
/// they deny (ADR 0002, 2026-10-12). Also web search and fetch, which
/// write nothing.
/// The Read tool may read anywhere (`Read(//**)`: `//` is Claude Code's
/// absolute form, one `/` would be the project's), as sandboxed Bash
/// already can. Without it the tool asks outside the project, where the
/// installed skills, the report catalogue, an unattended run's log and the
/// reviews folder all live, and an unattended run has no one to answer.
/// Deny rules win over it: the [`CREDENTIALS`], and every path `hidden`
/// hides from sandboxed Bash, so the tool opens nothing Bash could not read
/// already. One rule for every project, not a list of folders.
```

The first paragraph of that doc comment ("A Bash sandbox, so the session can work without a prompt per command…") stays as it is.

- [ ] **Step 5: Update the herdr doc comment**

In `src/session/herdr.rs`, on `agent_start_claude_refiner`, replace:

```rust
/// What keeps it from prompting instead is `settings` (see
/// `refine::session_settings`): a Bash sandbox whose commands run unasked while
/// the OS stops them writing anywhere but the task database, plus the few
/// allow and deny rules that go with it.
```

with:

```rust
/// What keeps it from prompting instead is `settings` (see
/// `refine::session_settings`): a Bash sandbox that stops commands writing
/// anywhere but the task database, and allow rules that state that fence
/// (any Bash command, the Read tool anywhere but the credentials and
/// hidden paths, web search and fetch), so nothing the sandbox permits is
/// asked about.
```

- [ ] **Step 6: Fix the stale sentence in `claude_unattended_argv`'s doc comment**

In `src/refine.rs`, in the doc comment above `claude_unattended_argv`, replace `/// gives; the sandbox already runs every command unasked.` with:

```rust
/// gives; the settings already allow every sandboxed command and read.
```

- [ ] **Step 7: Run the tests**

Run: `cargo test`
Expected: all pass. Run `grep -rn 'Bash(task' src/` and expect no output.

- [ ] **Step 8: Extend the ADR addendum if needed**

If the Task 1 addendum does not already say that `Bash(task *)` gave way to bare `Bash`, add that sentence to it. Also check the Consequences paragraph that begins "The residual risk A and B leave": the addendum's last sentence should already cover it. Leave the original paragraph's text as it is, because addenda are how this record is updated.

- [ ] **Step 9: Commit**

```bash
git add src/refine.rs src/session/herdr.rs docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "fix(refine): allow every sandboxed Bash command unasked" -m "Auto-allow left Claude Code's command-shape checks in place, so a
headless refine was denied loops, \$( ) and quoted text that the sandbox
would have run safely. A bare Bash rule states the fence the OS already
enforces, for the tab and the unattended run alike, and replaces
Bash(task *).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Log and announce an unattended run's denials

**Files:**
- Modify: `src/refine.rs`
  - `claude_unattended_argv` (about line 423)
  - `run_unattended` (about line 452)
  - `unattended_steps` (about line 527)
  - `run_claude` (about line 594)
  - New items `Denial`, `RunResult`, `parse_run`, `denied_sentence` and `with_denials`, placed beside `run_claude` and `with_log`
- Test: `src/refine.rs` tests
  - `an_unattended_run_is_the_tabs_claude_with_no_one_to_ask`
  - `a_run_logs_its_output_under_its_step_and_fails_on_a_bad_exit`
  - New tests

**Interfaces:**
- Consumes: nothing from Task 2. The two tasks are independent apart from the order of commits.
- Produces (all private to `refine.rs`):
  - `struct Denial { tool: String, input: String }` (`Debug, Clone, PartialEq, Eq`)
  - `struct RunResult { text: String, denials: Vec<Denial> }` (`Debug, Default, PartialEq, Eq`)
  - `fn parse_run(stdout: &str) -> Option<RunResult>`
  - `fn denied_sentence(denials: &[Denial]) -> Option<String>`
  - `fn with_denials(message: &str, denials: &[Denial]) -> String`
  - `fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String], denials: &mut Vec<Denial>) -> Result<()>`
  - `fn unattended_steps(ws: &Workspace, t: &task::Task, dirs: &Dirs, log: &Path, denials: &mut Vec<Denial>) -> Result<()>`
  - `claude_unattended_argv` ends with `["--permission-prompts", "none", "--output-format", "json"]`.

The JSON that `claude -p --output-format json` prints on 2.1.296 is a single object. The fields used here are `"result"` (Claude's final text, a string) and `"permission_denials"` (an array of `{"tool_name": "Bash", "tool_use_id": "…", "tool_input": {…}}`). Everything else is ignored.

- [ ] **Step 1: Write the failing tests**

Add these to the `tests` module:

```rust
    /// The result of `claude -p --output-format json`: the final text, and
    /// each refused action as its tool and its input, compact. Anything
    /// that is not that JSON is `None`, so the caller can log it raw.
    #[test]
    fn a_json_result_gives_its_text_and_its_denials() {
        let out = r#"{"type":"result","subtype":"success","is_error":false,"result":"Planned it.","permission_denials":[{"tool_name":"Bash","tool_use_id":"t1","tool_input":{"command":"ls ~/.task"}},{"tool_name":"Read","tool_use_id":"t2","tool_input":{"file_path":"/x"}}]}"#;
        assert_eq!(
            parse_run(&format!("{out}\n")).unwrap(),
            RunResult {
                text: "Planned it.".into(),
                denials: vec![
                    Denial { tool: "Bash".into(), input: r#"{"command":"ls ~/.task"}"#.into() },
                    Denial { tool: "Read".into(), input: r#"{"file_path":"/x"}"#.into() },
                ],
            }
        );
        assert_eq!(parse_run(r#"{"result":"ok","permission_denials":[]}"#).unwrap(), RunResult { text: "ok".into(), denials: vec![] });
        assert_eq!(parse_run(r#"{"result":"ok"}"#).unwrap().denials, vec![], "no field, no denials");
        assert_eq!(parse_run("Error: something broke\n"), None);
        assert_eq!(parse_run(""), None);
    }

    /// Denials are counted and named by tool, each tool once; none at all
    /// says nothing.
    #[test]
    fn denials_are_counted_and_named_by_tool() {
        let d = |tool: &str| Denial { tool: tool.into(), input: "{}".into() };
        assert_eq!(denied_sentence(&[]), None);
        assert_eq!(denied_sentence(&[d("Bash")]).unwrap(), "Claude was refused 1 action (Bash).");
        assert_eq!(denied_sentence(&[d("Read"), d("Bash"), d("Bash")]).unwrap(), "Claude was refused 3 actions (Bash, Read).");
        assert_eq!(with_denials("Planned and reported: fix: a thing", &[]), "Planned and reported: fix: a thing");
        assert_eq!(
            with_denials("Planned and reported: fix: a thing", &[d("Bash")]),
            "Planned and reported: fix: a thing. Claude was refused 1 action (Bash)."
        );
        assert_eq!(with_denials("It failed.", &[d("Read")]), "It failed. Claude was refused 1 action (Read).");
    }
```

Replace `a_run_logs_its_output_under_its_step_and_fails_on_a_bad_exit` with:

```rust
    /// A run's stderr lands in the log under its step's heading, then the
    /// result's text and one line per refused action, or a line saying
    /// there were none, so a log always answers the question. The
    /// denials also go to the caller. Output that is not the JSON result
    /// is logged as it came. A non-zero exit is the step's failure, named.
    /// `sh` stands in for claude: `argv[..3]` is what the heading shows.
    #[test]
    fn a_run_logs_its_result_and_denials_under_its_step_and_fails_on_a_bad_exit() {
        let dir = std::env::temp_dir().join(format!("niritasks-unattended-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("x.log");
        let mut denials = Vec::new();

        let json = r#"{"result":"Planned it.","permission_denials":[{"tool_name":"Bash","tool_use_id":"t1","tool_input":{"command":"ls ~/.task"}}]}"#;
        let ok = ["sh".to_string(), "-c".to_string(), format!("echo warn >&2; echo '{json}'")];
        run_claude(&dir, &log, Step::Refine, &ok, &mut denials).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("== refine: sh -c echo warn"), "{text}");
        assert_eq!(lines[1..], ["warn", "Planned it.", r#"== denied: Bash {"command":"ls ~/.task"}"#], "{text}");
        assert_eq!(denials, vec![Denial { tool: "Bash".into(), input: r#"{"command":"ls ~/.task"}"#.into() }]);

        let clean = ["sh", "-c", r#"echo '{"result":"Reported.","permission_denials":[]}'"#].map(String::from);
        run_claude(&dir, &log, Step::Report, &clean, &mut denials).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("Reported.\n== denied: none\n"), "{text}");
        assert_eq!(denials.len(), 1, "a clean run adds nothing");

        let raw = ["sh", "-c", "echo not json"].map(String::from);
        run_claude(&dir, &log, Step::Report, &raw, &mut denials).unwrap();
        assert!(std::fs::read_to_string(&log).unwrap().ends_with("== report: sh -c echo not json\nnot json\n"));

        let bad = ["sh", "-c", "exit 3"].map(String::from);
        let err = run_claude(&dir, &log, Step::Report, &bad, &mut denials).unwrap_err().to_string();
        assert!(err.starts_with("The report step failed: claude exited with exit status: 3"), "{err}");
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("== report: sh -c exit 3\n"), "{text}");

        append_line(&log, "== failed: because");
        assert!(std::fs::read_to_string(&log).unwrap().ends_with("== failed: because\n"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

In `an_unattended_run_is_the_tabs_claude_with_no_one_to_ask`, change the tail assertion and the doc comment's first sentence:

```rust
    /// `claude -p` with the tab's own flags between the prompt and
    /// `--permission-prompts none`, which makes anything that would ask
    /// refuse instead, then `--output-format json`, so the run's refusals
    /// can be read back. Default permission mode, as the tab: not plan
    /// mode, whose approval means implement, and not auto mode.
```

```rust
        assert_eq!(argv[3 + flags.len()..], ["--permission-prompts", "none", "--output-format", "json"]);
```

- [ ] **Step 2: Run them and check that they fail**

Run: `cargo test --lib refine::tests`
Expected: compile errors (`parse_run`, `RunResult`, `Denial`, `denied_sentence` and `with_denials` not found, and `run_claude` takes 4 arguments).

- [ ] **Step 3: Add the parsing and wording helpers**

Put these after `run_claude` in `src/refine.rs`:

```rust
/// One action Claude was refused in an unattended run: the tool, and what
/// it was asked to do as compact JSON, as `claude -p --output-format json`
/// lists it under `permission_denials`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Denial {
    tool: String,
    input: String,
}

/// What a `claude -p --output-format json` run printed: Claude's final
/// text and the actions it was refused.
#[derive(Debug, Default, PartialEq, Eq)]
struct RunResult {
    text: String,
    denials: Vec<Denial>,
}

/// The run's JSON result, or `None` when `stdout` is not one: a crash, or
/// a run that never got as far as a result.
fn parse_run(stdout: &str) -> Option<RunResult> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let text = v["result"].as_str().unwrap_or_default().to_string();
    let denials = v["permission_denials"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|d| Denial { tool: d["tool_name"].as_str().unwrap_or("?").to_string(), input: d["tool_input"].to_string() })
                .collect()
        })
        .unwrap_or_default();
    Some(RunResult { text, denials })
}

/// A sentence counting the actions Claude was refused and naming their
/// tools, each once, or `None` when there were none.
fn denied_sentence(denials: &[Denial]) -> Option<String> {
    if denials.is_empty() {
        return None;
    }
    let mut tools: Vec<&str> = denials.iter().map(|d| d.tool.as_str()).collect();
    tools.sort_unstable();
    tools.dedup();
    let n = denials.len();
    let s = if n == 1 { "" } else { "s" };
    Some(format!("Claude was refused {n} action{s} ({}).", tools.join(", ")))
}

/// `message` followed by [`denied_sentence`] as a sentence of its own, or
/// `message` alone when nothing was refused.
fn with_denials(message: &str, denials: &[Denial]) -> String {
    match denied_sentence(denials) {
        None => message.to_string(),
        Some(sentence) => {
            let stop = if message.ends_with('.') { "" } else { "." };
            format!("{message}{stop} {sentence}")
        }
    }
}
```

- [ ] **Step 4: Rewrite `run_claude`**

Replace the whole function and its doc comment:

```rust
/// One `claude -p --output-format json` run in `project`, logged to `log`
/// under a line naming the step and the prompt (not the settings, which
/// are long and the same for both). Its stderr goes to the log as it
/// comes. Its stdout is the JSON result: the log gets the result's text,
/// then a `== denied:` line per refused action or `== denied: none`, and
/// the refusals are added to `denials`. Stdout that is not that JSON is
/// logged as it came. Stdin is closed: there is no one typing.
fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String], denials: &mut Vec<Denial>) -> Result<()> {
    append_line(log, &format!("== {}: {}", step.name(), argv[..3].join(" ")));
    let err = File::options().append(true).create(true).open(log).with_context(|| format!("could not open {}", log.display()))?;
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(project)
        .stdin(Stdio::null())
        .stderr(Stdio::from(err))
        .output()
        .with_context(|| format!("could not run {} for the {} step", argv[0], step.name()))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    match parse_run(&stdout) {
        Some(run) => {
            append_line(log, &run.text);
            if run.denials.is_empty() {
                append_line(log, "== denied: none");
            }
            for d in &run.denials {
                append_line(log, &format!("== denied: {} {}", d.tool, d.input));
            }
            denials.extend(run.denials);
        }
        None => append_raw(log, &stdout),
    }
    anyhow::ensure!(out.status.success(), "The {} step failed: claude exited with {}.", step.name(), out.status);
    Ok(())
}
```

Add this beside `append_line`:

```rust
/// Add `text` to the log as it is, with no line added. Best effort, as
/// [`append_line`].
fn append_raw(log: &Path, text: &str) {
    if let Ok(mut file) = File::options().append(true).create(true).open(log) {
        let _ = file.write_all(text.as_bytes());
    }
}
```

`Command::output` keeps a stdio that was set explicitly, so stderr still goes to the log while stdout is captured.

- [ ] **Step 5: Add the flag to the argv**

In `claude_unattended_argv`:

```rust
    argv.extend(["--permission-prompts", "none", "--output-format", "json"].map(String::from));
```

Add this sentence to its doc comment, after "is refused rather than waited on.": `` `--output-format json` makes the run print its result as JSON, refusals included, for [`run_claude`] to log. ``

- [ ] **Step 6: Thread the denials through `unattended_steps` and `run_unattended`**

In `unattended_steps`, add the parameter `denials: &mut Vec<Denial>` (signature under **Interfaces**) and pass it to both calls:

```rust
    run_claude(project, log, Step::Refine, &claude_unattended_argv(&refine_prompt, &settings, &mod_dir), denials)?;
```

```rust
    run_claude(project, log, Step::Report, &claude_unattended_argv(&report_prompt, &settings, &mod_dir), denials)?;
```

In `run_unattended`, collect the denials and name them in either outcome. A success with denials also gives the log, because that is where the details are:

```rust
    task::set_processing(&t.uuid, true)?;
    let mut denials = Vec::new();
    let outcome = unattended_steps(ws, t, &dirs, &log, &mut denials);
```

```rust
    match settle(outcome, cleared, t) {
        Ok(message) if denials.is_empty() => {
            notify::tasks(&message);
            Ok(())
        }
        Ok(message) => {
            notify::tasks(&with_log(&with_denials(&message, &denials), &log));
            Ok(())
        }
        Err(e) => {
            append_line(&log, &format!("== failed: {e:#}"));
            bail!("{}", with_log(&with_denials(&format!("{e:#}"), &denials), &log))
        }
    }
```

Extend `run_unattended`'s doc comment. After "Both runs' output goes to one log ([`log_path`]), which a failure's error names;", add: `` any action Claude was refused is listed there too and counted in the notification or the error, with the log's path. ``

- [ ] **Step 7: Run the tests**

Run: `cargo test`
Expected: all pass, and `cargo build` gives no warnings about unused items.

- [ ] **Step 8: Commit**

```bash
git add src/refine.rs
git commit -m "fix(refine): log and announce an unattended run's denials" -m "The unattended run now asks claude -p for its JSON result and writes
the final text and every permission denial to its log, or a line saying
there were none. The notification, or the error, counts the refused
actions by tool and gives the log. Before, the log held only Claude's
closing prose, so a denied read showed up only as a weaker plan.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Take the shell-shape rules out of the skills

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md` (the `auto` bullet, about lines 98–113)
- Modify: `.claude/skills/report-task/SKILL.md` (the unattended paragraph, about lines 87–99)

**Interfaces:**
- Consumes: Task 2's bare `Bash` rule (it makes the shape rules unnecessary) and the existing `Read(//**)` rule.
- Produces: nothing for later tasks.

Keep both skills' sandbox paragraph ("Your Bash commands run in a sandbox that can read anything but write only to the task database…") exactly as it is. Add no new shell-shape guidance anywhere.

- [ ] **Step 1: Trim refine-task's `auto` bullet**

In `.claude/skills/refine-task/SKILL.md`, the `auto` bullet ends:

```markdown
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended. Nobody can answer a permission prompt either, so
  anything that would ask is denied: keep every shell command to one plain
  read-only pipeline of `ls`, `cat`, `grep`, `head`, `tail`, `find`, `wc`
  or `task` — no `$( )`, loops, variables (bare or quoted), `;` chains or
  glob paths — and open a file, in or out of the project, with the Read
  tool, which may read anywhere.
```

Replace it with:

```markdown
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended.
```

- [ ] **Step 2: Trim report-task's unattended paragraph**

In `.claude/skills/report-task/SKILL.md`, replace:

```markdown
everything else is the same, and you still end at step 5. Nobody can answer
a permission prompt either, so anything that would ask is denied: keep
every shell command to one plain read-only pipeline of `ls`, `cat`, `grep`,
`head`, `tail`, `find`, `wc` or `task` — no `$( )`, loops, variables (bare
or quoted), `;` chains or glob paths — and open a file, in or out of the
project (the run's own log under `~/.local/share/niri-tasks/unattended/`,
a report under the reviews folder), with the Read tool, which may read
anywhere.
```

with:

```markdown
everything else is the same, and you still end at step 5. The run's own
log is under `~/.local/share/niri-tasks/unattended/`, named by the task's
first eight characters.
```

- [ ] **Step 3: Check that nothing else in either skill is now untrue**

```bash
grep -n -i 'permission prompt\|pipeline\|\$( )\|loops\|Read tool\|denied' .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md
```

Expected: no line still tells the model to avoid a shell shape, or says a read outside the project would be denied. Any remaining hit has to be about something else, such as the write tool's own answers. Read each hit in context, and remove only text the fix makes untrue.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md
git commit -m "fix(refine): drop the shell-shape rules from the skills" -m "With every sandboxed command allowed unasked, an unattended run no
longer needs to keep to plain pipelines, and the skills stop steering
the model around prompts the permission rules now settle.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Say where the denials are shown

**Files:**
- Modify: `README.md` (the Add & all paragraph, about lines 405–416)
- Modify: `llms.txt` (the `niritasks task refine <uuid> --unattended` row, line 44)

**Interfaces:**
- Consumes: Task 3's log lines (`== denied: …`, `== denied: none`) and the notification sentence `Claude was refused N action(s) (Tools).`
- Produces: nothing.

- [ ] **Step 1: README**

In `README.md`, replace:

```markdown
nothing; a notification says `Planned and reported` when both are done, or
which step failed and where its log is
(`$XDG_DATA_HOME/niri-tasks/unattended/`, by default
`~/.local/share/niri-tasks/unattended/`).
```

with:

```markdown
nothing; a notification says `Planned and reported` when both are done, or
which step failed and where its log is
(`$XDG_DATA_HOME/niri-tasks/unattended/`, by default
`~/.local/share/niri-tasks/unattended/`). The log holds each run's final
answer and every action Claude was refused, if any. The notification
counts the refusals and gives the log's path.
```

- [ ] **Step 2: llms.txt**

In `llms.txt` line 44, replace `with a notification `Planned and reported: …`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log`).` with:

```text
with a notification `Planned and reported: …`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log`); the log lists each action Claude was refused (`== denied: <tool> <input>`, or `== denied: none`), and the notification counts them (`Claude was refused 2 actions (Bash, Read).`) and gives the log.
```

Check that the row is still one table line: no line breaks, and the pipe characters at both ends are kept.

- [ ] **Step 3: Commit**

```bash
git add README.md llms.txt
git commit -m "docs: say an unattended run logs and counts its denials" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Verify, land and check end to end

**Files:** none changed, unless a check fails.

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces: the task's **Done when** checked, and `a67de29b` closed.

- [ ] **Step 1: Automated checks in the worktree**

```bash
cargo test
(cd claude/refine-mod && claude plugin test)
```

Expected: both pass. If `claude plugin test` needs other arguments, look at how `tests/all.sh` or the mod's `package.json` calls it, and use the same form.

- [ ] **Step 2: Land the branch**

Use the `finish-worktree` skill. It rebases, tests, fast-forwards `main` and pushes. Its `niritasks task status … completed` step for `af9e53fc` should wait until Steps 3–5 pass: if the skill completes the task first, reopen it with `niritasks task status af9e53fc stopped` until they do. Make sure the installed `niritasks` is rebuilt from `main` (install.sh or the skill's own step), because the refine sessions below run the installed binary and load skills from `~/Projects/niri-tasks`.

- [ ] **Step 3: Attended check (with the user)**

Add a throwaway task from a niri-tasks herdr pane, and open a Refine tab on it:

```bash
niritasks task add 'docs: Describe how ~/.task and the unattended logs are laid out'
U=$(task rc.json.array=on +niri_tasks status:pending description~'unattended logs are laid out' export | jq -r '.[0].uuid')
niritasks task note "$U" 'Context: list ~/.task and ~/.local/share/niri-tasks/unattended, count the lines of each src/*.rs file with a for loop and $( ), and read a log under ~/.worktrees'
niritasks task refine "$U"
```

Ask the user to watch the tab. Pass: it reads files outside the project and runs a loop and `$( )` with no permission prompt, and the only question is the write tool's. Then ask the tab, in it: `Run touch ~/Projects/niri-tasks/.probe and show me the error`. Pass: `Read-only file system`. The standing instruction may make Claude decline to run the command. If it does, run the same check from the unattended log in Step 4 instead, or accept the user's word that the tab ran it.

- [ ] **Step 4: Unattended check**

On the same throwaway task:

```bash
niritasks task refine "$U" --unattended
LOG=$(ls -t ~/.local/share/niri-tasks/unattended/"${U:0:8}"-*.log | head -1)
grep -n '^== ' "$LOG"
```

Pass:
- Both steps show `== denied: none`.
- The notification reads `Planned and reported: …` with no refusal sentence.
- The report step read the run log: the report-step transcript under `~/.claude/projects/-home-paul-Projects-niri-tasks/` shows a Read or a `cat` of a path under `~/.local/share/niri-tasks/unattended/`, or the report quotes the log. If the report step had no reason to read the log, say so instead of forcing it.

- [ ] **Step 5: Clean up and close the superseded task**

```bash
niritasks task status "$U" deleted --yes
niritasks task note a67de29b-8ce4-47db-a1a7-5709d66cfc75 'Superseded by af9e53fc: refine stays in default mode with the sandbox in auto-allow and a bare Bash rule; see docs/adr/0002, 2026-10-12.'
niritasks task status a67de29b-8ce4-47db-a1a7-5709d66cfc75 completed
```

`82ff74c6` is already completed. Add the note on it alone:

```bash
niritasks task note 82ff74c6-f7f3-49d5-95db-b597a16cb6fa 'Superseded by af9e53fc, which replaced the shell-shape skill rules with a bare Bash allow.'
```

Then complete `af9e53fc` itself: `niritasks task status af9e53fc completed`.
