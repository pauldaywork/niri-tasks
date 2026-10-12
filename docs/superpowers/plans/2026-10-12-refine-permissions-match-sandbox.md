# Refine's Permissions Match Its Sandbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Block what is unsafe and stop asking about what is safe. Refine, Grill me, Report and the unattended run keep every way to inspect the codebase and gather what the plan needs, with no prompt and no denial:
- reading any file but secrets,
- any shell command of any shape, run in the sandbox,
- the web, including `curl` and a shallow `git clone` of public source from a fixed list of hosts.

The two existing escapes out of the sandbox are closed first. An unattended run's log and notification name every action that was denied, so a safe action that is still blocked shows up instead of quietly producing a weaker plan.

**Architecture:** Everything lives in `refine::session_settings` and its helpers in `src/refine.rs`. A new `prepare_fence` builds a `Fence` (the hidden paths and a private temp folder per task), which the tab (`launch`) and the headless run (`unattended_steps`) share.
- **Escapes:** the fence closes the two existing escapes. Writes to `~/.task/hooks` are denied, and each session gets its own `CLAUDE_CODE_TMPDIR` in place of the shared `/tmp/claude-<uid>`. It also hides more secret stores and two reachable sockets.
- **Rules:** the permission rules then state the fence: bare `Bash`, `Read(//**)` under the deny rules, `WebSearch` and `WebFetch`.
- **Network:** a strict allowlist of read-only source and docs hosts on port 443.
- **Visibility:** the unattended run reads `claude -p --output-format json` and logs and announces `permission_denials`.

**Tech Stack:** Rust (`serde_json`, `anyhow`, `std::process`), Claude Code 2.1.296 (`claude -p`, `--settings`, `--output-format json|stream-json`, the Bash sandbox on bubblewrap and socat), Taskwarrior 3.5, herdr, `jq`.

**Spec:** Taskwarrior task `af9e53fc-76e4-4283-9a44-28f6076c5759` (description and notes). On 2026-10-12 the user widened the scope to fold in network access and scratch writes. Read the evidence and reviews this plan rests on, which are in `~/.local/share/niri-tasks/reviews/`:
- `af9e53fc-sandbox-widening-proposal.md`: the proposal the reviewers checked.
- The two safety reviews, summarised under **Review findings** below.

## Global Constraints

- **Aim:** block unsafe actions and stop asking about safe ones. No change may take away a way to read the codebase or gather information that the session has today, except reading secrets.
- **Mode:** stay in `--permission-mode default` with the sandbox in auto-allow. Not auto mode, not plan mode.
- **No hooks:** no `tool.check` or `PermissionRequest` hook in the mod, and no skill sentences that tell the model which shell shapes to avoid.
- **Order:** the escapes are closed (Task 2) and the secrets hidden (Task 3) **before** bare `Bash` (Task 4) or any network (Task 5). Bare `Bash` makes writing arbitrary content into a writable path trivial, and network lets injected web content deliver a payload.
- **Not adopted, and do not add:**
  - `WebFetch(domain:*)` or any allow-every-host rule: it reaches loopback, Docker and LAN services by IP literal, and no setting can close that.
  - Making `~/.cargo` or any folder on `PATH` writable.
  - A build or test scratch folder (`CARGO_TARGET_DIR`): no refine has ever needed one.
  - `permissions.additionalDirectories` or `--add-dir`: they add sandbox write access.
  - `excludedCommands`.
- **Kept as they are:** `allowUnsandboxedCommands: false`, `failIfUnavailable: true`, the project in `denyWrite`, and `allowWrite` on the task data.
- **Out of scope:** auto or plan mode, grilling unattended, the panel, and a timeout on the headless runs.
- **Already landed before this plan** (commits `9bd3459`, `b2bb4d2`): `Read(//**)` in the allow list, a Read deny for every hidden path, and the removal of `skill_dirs`. Do not redo them.
- **Superseded tasks:** `82ff74c6` is already completed. `a67de29b` is pending, and `b268d068` (`~/.task/hooks`) is fixed by Task 2. Task 9 closes both.
- **Writing:** task descriptions start with a Conventional Commits type. Commits follow Conventional Commits, mostly `fix(refine): …`.
- **Where to work:** run all commands from the worktree `/home/paul/.worktrees/niri-tasks/task-fix-make-refine-s-permissions-match-its-af9e53fc` unless a step says otherwise. Diagnosis and probe artefacts go in `~/.local/share/niri-tasks/reviews/af9e53fc-diagnosis/`, never `/tmp`.
- **Skills:** a refine session loads skills from `~/Projects/niri-tasks`, never from a worktree. Skill edits (Task 7) can only be checked end to end after the branch lands (Task 9).

## Review findings this plan acts on

Two adversarial reviews, run 2026-10-12, checked the docs, the sandbox-runtime source and this machine.

1. **`~/.task/hooks` is writable from a refine today.** Taskwarrior hooks are on, and the daemon's own `task` calls (`src/task.rs:383`) run with hooks. A planted hook therefore runs as the user, unsandboxed. This is task `b268d068`. **Fix:** `denyWrite` on the folder, with the folder made sure to exist (Task 2).
2. **`$TMPDIR` is `/tmp/claude-1000`, which is shared with the user's unsandboxed Claude sessions.** A refine can write into it: plant a `.git` with `core.fsmonitor` above other sessions' scratchpads, edit their scripts, or rewrite a concurrent session's `claude-settings-<hash>.json`. **Fix:** a private `CLAUDE_CODE_TMPDIR` per task (Task 2).
3. **Readable secrets that are not in the deny list:**
   - `~/.codex` (an OpenAI token and its IPC socket)
   - `~/.local/share/com.vercel.cli`
   - `~/.t3`
   - the Firefox snap profile (saved logins, including github.com)
   - the browser profiles
   - project `.env` files (SMTP, Twitter and database secrets)

   **Fix:** hide them from Bash and from the Read tool (Task 3).
4. **`~/.cache/ibus` holds a reachable bus socket.** **Fix:** hide it (Task 3).
5. **Network:**
   - Allowing every host is unsafe (IP literals reach loopback and the LAN).
   - A strict allowlist on `:443` is safe once 1–4 are fixed. Domain fronting through a CDN host remains a low-to-moderate exfiltration path, comparable to what bare `WebFetch` already allows (any URL, GET).
   - `github.com` stays on the list, since a shallow `git clone` needs it, and the one credential usable there (the Firefox login) is hidden by 3.
6. **Bare `Bash`** opens no unsandboxed path while `allowUnsandboxedCommands` is false and no `excludedCommands` exist. It does make arbitrary file content easy to write into writable paths, hence the order above.

## File Map

- `src/refine.rs`
  - `CREDENTIALS` (line 64): more secret stores, plus a project `.env` rule.
  - `hidden_paths`: adds `~/.cache/ibus`.
  - A new `Fence` struct, with `prepare_fence` and `session_tmp`.
  - `ensure_no_exposed_sockets` / `sandbox_temp`: take the session's temp folder.
  - `session_settings`: `Fence` in place of `hidden`, `denyWrite` on the hooks folder, `env.CLAUDE_CODE_TMPDIR`, the allow list, and `network`.
  - `launch` and `unattended_steps`: use `prepare_fence`.
  - `claude_unattended_argv`, `run_claude` and `run_unattended`: JSON output and denials.
  - Tests at the bottom of the file.
- `src/session/herdr.rs`: the doc comment on `agent_start_claude_refiner`.
- `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`: a dated addendum, plus the Consequences residual-risk paragraph now partly closed.
- `.claude/skills/refine-task/SKILL.md`, `.claude/skills/report-task/SKILL.md`: remove the shell-shape rules, and mention `$TMPDIR` and the hosts.
- `README.md`, `llms.txt`: the denials, and what the fence now allows.

---

### Task 1: Diagnose, and probe the two unknowns

This task changes no code. It answers three questions:
- why headless runs denied safe commands;
- whether `CLAUDE_CODE_TMPDIR` can be set through the `--settings` `env` key, which Task 2 relies on;
- whether a `denyRead` glob hides project `.env` files, which Task 3 relies on.

**Files:**
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`. Add a dated addendum after the last `*2026-10-…:*` paragraph at the top, before the first `##` heading.

**Interfaces:**
- Consumes: nothing.
- Produces three findings, reported to the controller.
  - **Verdict:**
    - **A:** the sandbox is active under `-p`, and the Bash denials come from Claude Code's command-shape or working-directory checks. Continue.
    - **B:** the sandbox is not active. **Stop** and report to the user.
    - **C:** anything else. Stop and report.
  - **`ENV_TMPDIR`:**
    - `works`: `$TMPDIR` inside a sandboxed command is the folder set in `env`. Task 2 uses the settings `env` key.
    - `ignored`: Task 2 Step 6's fallback applies.
  - **`ENV_GLOB`:**
    - `works`: a `denyRead` entry `~/Projects/**/.env*` hides a project `.env` from `cat`. Task 3 uses it.
    - `ignored`: Task 3 Step 5's fallback applies.

- [ ] **Step 1: Build, and set up the diagnosis folder and a `claude` shim**

```bash
cargo build
D=~/.local/share/niri-tasks/reviews/af9e53fc-diagnosis
mkdir -p "$D/bin"
command -v claude   # normally /home/paul/.local/bin/claude
```

`run_claude` executes `claude` from `PATH`. The shim records each argv (NUL-separated, since the settings JSON contains spaces) and streams JSON. Write it to `$D/bin/claude`, and replace the `exec` path if `command -v` printed another:

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

- [ ] **Step 2: Add a throwaway task, and run the unattended refine through the shim**

Run this from a pane in the niri-tasks herdr session, so the task gets `+niri_tasks`:

```bash
./target/debug/niritasks task add 'docs: Describe how ~/.task and the unattended logs are laid out'
U=$(task rc.json.array=on +niri_tasks status:pending description~'unattended logs are laid out' export | jq -r '.[0].uuid')
./target/debug/niritasks task note "$U" 'Context: list ~/.task and ~/.local/share/niri-tasks/unattended, count the lines of each src/*.rs file in one loop, read a log under ~/.worktrees, and fetch https://herdr.dev/llms.txt with curl'
PATH="$D/bin:$PATH" ./target/debug/niritasks task refine "$U" --unattended
```

It takes minutes and ends with a notification or an error naming a log. Either outcome is fine. If the run fails because the sandbox is unavailable inside your own sandboxed shell (nested bwrap), ask the user to run the last command with `! `.

- [ ] **Step 3: Read the init event, the denials and the failed tool results**

```bash
LOG=$(ls -t ~/.local/share/niri-tasks/unattended/"${U:0:8}"-*.log | head -1)
grep '^{' "$LOG" | jq -c 'select(.type=="system" and .subtype=="init") | {permissionMode, cwd}'
grep '^{' "$LOG" | jq -c 'select(.type=="result") | {subtype, is_error, denials: .permission_denials}'
grep '^{' "$LOG" | jq -r 'select(.type=="user") | .message.content[]? | select(.type=="tool_result" and (.is_error==true)) | .content | tostring' | head -40
```

Sort every denial and failed result into two groups:
- **Safe but blocked:** reads, read-only commands, fetches.
- **Rightly blocked:** project writes, credentials, hidden sockets.

The curl to herdr.dev should be denied (`SandboxNetworkAccess`) until Task 5.

- [ ] **Step 4: Probe the sandbox, the `env` key and the `.env` glob in one run**

Re-run the recorded refine argv with the prompt changed and the settings JSON extended. The probe covers four things:
- a read of a hidden path (whether the sandbox is active);
- `$TMPDIR` (whether the `env` key reaches it);
- a `.env` file (the glob);
- the `;` and `$?` shapes.

Make the probe `.env` file in a throwaway folder under `~/Projects`, so no real secret is read:

```bash
mkdir -p ~/Projects/.af9e53fc-probe && echo 'PROBE=visible' > ~/Projects/.af9e53fc-probe/.env
mkdir -p ~/.cache/niri-tasks/refine-tmp/probe
cd ~/Projects/niri-tasks
mapfile -d '' -t A < "$D/argv-0.bin"
for i in "${!A[@]}"; do [ "${A[$i]}" = "--settings" ] && S=$((i+1)); done
A[$S]=$(printf '%s' "${A[$S]}" | jq -c --arg t "$HOME/.cache/niri-tasks/refine-tmp/probe" --arg g "$HOME/Projects/**/.env*" \
  '.env = {CLAUDE_CODE_TMPDIR: $t} | .sandbox.filesystem.denyRead += [$g]')
A[1]='Run exactly this one Bash command, unchanged, then quote its full output and nothing else: ls /run/user; echo rc=$?; echo TMPDIR=$TMPDIR; cat ~/Projects/.af9e53fc-probe/.env; echo rc=$?'
/home/paul/.local/bin/claude "${A[@]}" --output-format stream-json --verbose > "$D/probe.jsonl" 2>&1
jq -c 'select(.type=="result") | {result, denials: .permission_denials}' "$D/probe.jsonl"
cd -
```

`A[0]` is `-p` and `A[1]` the prompt. Read the result:

| Check | Result | Means |
|---|---|---|
| `ls /run/user` | fails (hidden) | the sandbox is active |
| `ls /run/user` | lists `1000` | the sandbox is not active: **verdict B** |
| `TMPDIR=` | `…/refine-tmp/probe` | **`ENV_TMPDIR` works** |
| `TMPDIR=` | `/tmp/claude-1000` | `ENV_TMPDIR` is ignored |
| `cat` of the probe `.env` | fails | **`ENV_GLOB` works** |
| `cat` of the probe `.env` | prints `PROBE=visible` | `ENV_GLOB` is ignored |
| `permission_denials` | holds this command | the shape check denied it. Re-ask with `bash -c '…'` only if needed to read the other answers |

- [ ] **Step 5: Stop on verdict B or C**

Report the evidence (the jq outputs and the log path) and stop. Otherwise go on.

- [ ] **Step 6: Clean up**

```bash
./target/debug/niritasks task status "$U" deleted --yes
rm -r ~/Projects/.af9e53fc-probe ~/.cache/niri-tasks/refine-tmp/probe
```

Keep `$D`: Task 8 reuses the shim.

- [ ] **Step 7: Write the addendum**

Add a dated paragraph to ADR 0002 after the last `*2026-10-…:*` paragraph, filled in with the measured facts. For verdict A:

```markdown
*2026-10-12:* a headless refine (`claude -p … --permission-prompts none`,
Claude Code 2.1.296) runs in `default` mode with the Bash sandbox active
(a probe of `ls /run/user` got [error]). Its Bash denials came from
Claude Code's own command-shape and working-directory checks ([quote
two]), which `autoAllowBashIfSandboxed` does not override with no one
to ask, and its network calls were refused because no host was allowed.
Two reviews the same day also found two escapes this record's
Consequences did not weigh: a refine can write `~/.task/hooks`, which
the daemon's `task` calls then run unsandboxed (task `b268d068`), and
its `$TMPDIR`, `/tmp/claude-<uid>`, is shared with the user's
unsandboxed Claude sessions. Both are closed, more secret stores are
hidden, and only then are the rules widened to state the fence: bare
`Bash` in place of `Bash(task *)`, and a strict allowlist of read-only
source and docs hosts on port 443. Allowing every host was rejected:
IP literals bypass the sandbox's local-address guard. See
`docs/superpowers/plans/2026-10-12-refine-permissions-match-sandbox.md`.
```

- [ ] **Step 8: Commit**

```bash
git add docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "docs(adr): record why headless refine runs were denied" -m "A diagnosis run of the unattended argv shows the sandbox active under
-p and the denials coming from Claude Code's command-shape checks and
an empty network allowlist; reviews found two escapes to close first.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Close the two escapes: `~/.task/hooks` and the shared temp folder

**Files:**
- Modify: `src/refine.rs`
  - New `Fence`, `session_tmp` and `prepare_fence`, next to `hidden_paths`.
  - `sandbox_temp` and `ensure_no_exposed_sockets`.
  - `session_settings`: its signature, `filesystem.denyWrite`, the new `env`, and the doc comment.
  - `launch` (about line 279) and `unattended_steps` (about line 527).
- Test: `src/refine.rs`. Every test that calls `session_settings` (they pass `&hidden` or `&[]` as the third argument), plus new tests.

**Interfaces:**
- Consumes: Task 1's `ENV_TMPDIR`.
- Produces:
  - `pub struct Fence { pub home: PathBuf, pub hidden: Vec<PathBuf>, pub tmp: PathBuf }` (`Debug, Clone, PartialEq, Eq`). `home` is there for Task 3's `.env` globs.
  - `pub fn session_tmp(dirs: &Dirs, uuid: &str) -> PathBuf`, which is `<cache>/niri-tasks/refine-tmp/<uuid8>`
  - `fn prepare_fence(dirs: &Dirs, task_data: &Path, uuid: &str) -> Result<Fence>`
  - `pub fn session_settings(project: &Path, task_data: &Path, fence: &Fence, uuid: &str, reports: &Path, mode: Mode, unattended: bool) -> String`
    - `sandbox.filesystem.denyWrite == [project, task_data/hooks]`
    - `sandbox.filesystem.denyRead == fence.hidden`
    - `env == {"CLAUDE_CODE_TMPDIR": fence.tmp}`
  - `fn ensure_no_exposed_sockets(hidden: &[PathBuf], tmp: &Path) -> Result<()>`

- [ ] **Step 1: Write the failing tests**

Add a helper to the `tests` module:

```rust
    /// A fence for the settings tests: these hidden paths and a fixed
    /// private temp folder.
    fn fence(hidden: &[PathBuf]) -> Fence {
        Fence { home: PathBuf::from("/home/x"), hidden: hidden.to_vec(), tmp: PathBuf::from("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94") }
    }
```

Change every `session_settings(…, &hidden, …)` call in the tests to `&fence(&hidden)`, and every `session_settings(…, &[], …)` call to `&fence(&[])`. Find them with `grep -n 'session_settings(' src/refine.rs`.

Replace the `filesystem` assertions in `the_sandbox_fences_the_session_to_the_task_database` with these:

```rust
        assert_eq!(
            sb["filesystem"]["denyWrite"],
            serde_json::json!(["/home/x/Projects/alpha", "/home/x/.task/hooks"]),
            "the project, and the hooks the user's own task commands would run"
        );
        assert_eq!(sb["filesystem"]["allowWrite"], serde_json::json!(["/home/x/.task"]));
        assert_eq!(sb["filesystem"]["denyRead"], serde_json::json!(["/run/user/1000", "/run/docker.sock"]));
        assert_eq!(
            v["env"]["CLAUDE_CODE_TMPDIR"], "/home/x/.cache/niri-tasks/refine-tmp/d9f76b94",
            "a temp folder of its own, not the one the user's other sessions share"
        );
```

Add these:

```rust
    /// Each task's session gets its own temp folder under the cache, short
    /// and named by the task, so nothing unsandboxed shares it.
    #[test]
    fn a_session_gets_a_temp_folder_of_its_own() {
        let dirs = Dirs::at(Path::new("/home/x"));
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(session_tmp(&dirs, u), PathBuf::from("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94"));
        assert_eq!(
            session_tmp(&dirs.with_cache(Path::new("/c")), u),
            PathBuf::from("/c/niri-tasks/refine-tmp/d9f76b94")
        );
    }

    /// The fence makes the folders it denies or hands out, since the
    /// sandbox drops an entry whose path does not exist, and a `denyWrite`
    /// that was dropped would let the session create the hooks folder and
    /// write a hook into it.
    #[test]
    fn preparing_the_fence_makes_the_hooks_and_temp_folders() {
        let root = std::env::temp_dir().join(format!("niritasks-fence-{}", std::process::id()));
        let home = root.join("home");
        let task_data = root.join("task");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&task_data).unwrap();
        let dirs = Dirs::at(&home);
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let f = prepare_fence(&dirs, &task_data, u).unwrap();
        assert!(task_data.join("hooks").is_dir());
        assert_eq!(f.tmp, session_tmp(&dirs, u));
        assert!(f.tmp.is_dir());
        std::fs::remove_dir_all(&root).unwrap();
    }
```

`prepare_fence` also calls `ensure_no_exposed_sockets` against the live socket table. If that makes the test fail on a machine with a stray socket, split it: test a `fn make_fence_dirs(task_data: &Path, tmp: &Path) -> Result<()>` that `prepare_fence` calls, and leave the socket check untested here (it already has its own tests).

- [ ] **Step 2: Run them and check that they fail**

Run: `cargo test --lib refine::tests`
Expected: compile errors, because `Fence`, `session_tmp` and `prepare_fence` don't exist yet.

- [ ] **Step 3: Implement `Fence`, `session_tmp` and `prepare_fence`**

Put these after `hidden_paths` in `src/refine.rs`:

```rust
/// What a refine session is fenced in by beyond its project: the home
/// its secrets are found under, the paths hidden from it ([`hidden_paths`]) and its own temp folder
/// ([`session_tmp`]), made ready by [`prepare_fence`] and written into
/// the settings by [`session_settings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    pub home: PathBuf,
    pub hidden: Vec<PathBuf>,
    pub tmp: PathBuf,
}

/// The temp folder a refine of `uuid` gets as `$TMPDIR`, in place of the
/// `/tmp/claude-<uid>` the user's own, unsandboxed Claude sessions share:
/// a sandboxed command may write its temp folder, and one it shares is a
/// place to leave a trap for a process that is not sandboxed. Under the
/// cache, named by the task's first eight characters, kept short because
/// Claude Code falls back to the shared folder for a long one.
pub fn session_tmp(dirs: &Dirs, uuid: &str) -> PathBuf {
    dirs.cache().join("niri-tasks/refine-tmp").join(names::uuid8(uuid))
}

/// The fence for a refine of `uuid`, ready to start a session behind:
/// the hooks folder under `task_data` and the session's temp folder made,
/// because the sandbox drops a path that does not exist (a dropped
/// `denyWrite` on the hooks would let the session make the folder and
/// write a hook), then refused if any socket is left in reach.
fn prepare_fence(dirs: &Dirs, task_data: &Path, uuid: &str) -> Result<Fence> {
    let hooks = task_data.join("hooks");
    std::fs::create_dir_all(&hooks).with_context(|| format!("could not make {}", hooks.display()))?;
    let tmp = session_tmp(dirs, uuid);
    std::fs::create_dir_all(&tmp).with_context(|| format!("could not make {}", tmp.display()))?;
    let hidden = hidden_paths(dirs.home());
    ensure_no_exposed_sockets(&hidden, &tmp)?;
    Ok(Fence { home: dirs.home().to_path_buf(), hidden, tmp })
}
```

- [ ] **Step 4: Point the socket check at the session's temp folder**

Delete `sandbox_temp` and its doc comment. Change `ensure_no_exposed_sockets` to take the folder:

```rust
fn ensure_no_exposed_sockets(hidden: &[PathBuf], tmp: &Path) -> Result<()> {
```

and inside it:

```rust
    let exposed = exposed_sockets(&sockets, hidden, &[tmp.to_path_buf()]);
```

Add this sentence to its doc comment: `` `tmp` is the session's own temp folder, which the sandbox keeps visible even under a hidden path. ``

- [ ] **Step 5: Change `session_settings`**

The signature is listed under **Interfaces**. Inside it, use `fence.hidden` wherever it used `hidden`, and change the filesystem block. Add `env` too:

```rust
        "sandbox": {
            …
            "filesystem": {
                "denyWrite": [project, task_data.join("hooks")],
                "allowWrite": [task_data],
                "denyRead": &fence.hidden,
            },
        },
        "env": { "CLAUDE_CODE_TMPDIR": &fence.tmp },
```

Add a paragraph to the doc comment after the one about `task_data`:

```rust
/// Not the hooks under `task_data`, though: Taskwarrior runs them, hooks
/// are on, and the daemon's own `task` calls are not sandboxed, so a hook
/// written there would run as the user. And `$TMPDIR` is the fence's own
/// folder (`CLAUDE_CODE_TMPDIR`, [`session_tmp`]), not the temp folder
/// the user's unsandboxed Claude sessions share.
```

- [ ] **Step 6: Set `CLAUDE_CODE_TMPDIR` in the process environment (Task 1 found `ENV_TMPDIR` ignored)**

Task 1 measured that the settings `env` key does not move Claude Code's temp folder. Ruling (ledger): set it in the environment Claude starts in, on both paths. Do **not** add `env` to `session_settings`. Drop the `env` assertion from the Step 1 test, and the `env` line from Step 5.

- **Unattended run:** `run_claude` gains a `tmp: &Path` parameter (after `argv`) and adds `.env("CLAUDE_CODE_TMPDIR", tmp)` to its `Command`. `unattended_steps` passes `&fence.tmp`. Update the `run_claude` test to pass a temp path, and assert it reaches the child: run `sh -c 'echo "$CLAUDE_CODE_TMPDIR"'` and check the log.
- **Tab:** `Claude::Refiner` in `src/session.rs` gains `tmp: PathBuf`, documented as "the session's own temp folder, exported as `CLAUDE_CODE_TMPDIR` in the pane's shell before Claude starts". In `Session::start_claude` (`src/session.rs`), just before `self.port.agent_start(…)`, a `Claude::Refiner` first gets `self.port.pane_run(&self.name, pane, &herdr::export_tmpdir(tmp))`, with its errors handled the way that function already handles `agent_start`'s. It goes at the session layer, not in the herdr port, so the fake port logs it too. Put `export_tmpdir` in `src/session/herdr.rs`, beside `refiner_flags`. herdr starts Claude in that same interactive shell, so Claude inherits the variable. Add:

```rust
/// The shell line that sets a refine's temp folder in its pane before
/// Claude starts there: herdr's `agent start` launches Claude in the
/// pane's own shell, which then passes the variable on. Single-quoted,
/// with any `'` in the path closed, escaped and reopened.
pub fn export_tmpdir(tmp: &Path) -> String {
    format!("export CLAUDE_CODE_TMPDIR='{}'", tmp.display().to_string().replace('\'', "'\\''"))
}
```

  Test it: `/home/x/.cache/niri-tasks/refine-tmp/d9f76b94` gives `export CLAUDE_CODE_TMPDIR='/home/x/.cache/niri-tasks/refine-tmp/d9f76b94'`, and a path containing `a'b` gives `'…a'\''b…'`.
- **The fake port** (`src/session/fake.rs`) already logs every `pane_run`. Update the session tests that assert the exact refiner start log (`agent_start alpha task-abc p1 refiner`) so they also expect a `pane_run alpha p1 export CLAUDE_CODE_TMPDIR=…` line just before it. Every `Claude::Refiner { … }` literal in tests gains `tmp: PathBuf::from("/c/tmp")`. `launch` builds it with `tmp: fence.tmp.clone()`.
- **Doc:** in `session_settings`'s new doc paragraph, say `$TMPDIR` comes from `CLAUDE_CODE_TMPDIR`, which the tab's shell exports and the unattended run sets on the process, because the settings `env` key does not reach Claude Code's temp folder (measured 2026-10-12).

- [ ] **Step 7: Use `prepare_fence` in `launch` and `unattended_steps`**

In `launch`, replace:

```rust
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
```

with the following. `ws.session()` comes later, so the task data is read here. If `home` is now unused, drop it.

```rust
    let task_data = task::data_location()?;
    let fence = prepare_fence(&dirs, &task_data, &t.uuid)?;
```

Then build the settings with these:

```rust
    let settings = session_settings(session.dir(), &task_data, &fence, &t.uuid, &reports_dir(&dirs), mode, false);
```

In `unattended_steps`, make the same replacement. `let task_data = task::data_location()?;` moves above `prepare_fence`, and both `session_settings` calls pass `&fence` in place of `&hidden`.

- [ ] **Step 8: Run the tests**

Run: `cargo test`
Expected: all pass, with no warnings.

- [ ] **Step 9: Commit**

```bash
git add src/refine.rs
git commit -m "fix(refine): keep refine sessions out of hooks and shared temp" -m "A refine could write ~/.task/hooks, which the daemon's unsandboxed
task calls run, and its \$TMPDIR was /tmp/claude-<uid>, shared with the
user's unsandboxed Claude sessions. Deny writes to the hooks and give
each task's session a temp folder of its own under the cache.

Closes b268d068.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Hide the remaining secret stores and sockets

**Files:**
- Modify: `src/refine.rs`
  - `CREDENTIALS` (line 64)
  - `hidden_paths`
  - `session_settings`, for the `.env` rule
  - Doc comments
- Test: `src/refine.rs`
  - `credential_paths_resolve_under_home_and_skip_what_is_absent`
  - `the_session_may_search_the_web_but_not_read_credentials`
  - A new test

**Interfaces:**
- Consumes: `Fence` (with its `home`) from Task 2, and Task 1's `ENV_GLOB`.
- Produces:
  - `CREDENTIALS` with the new entries below.
  - `pub const SECRET_GLOBS: &[&str] = &["~/Projects/**/.env*"];`. Each entry becomes a `Read(...)` deny rule and a `sandbox.filesystem.denyRead` entry (the latter with `~` expanded to home).
  - `hidden_paths` includes `~/.cache/ibus` when it exists.

- [ ] **Step 1: Write the failing test**

```rust
    /// Secret stores beyond the classic credentials are hidden too: the
    /// Codex and Vercel CLIs' tokens, t3's keys, the browsers' profiles
    /// (saved logins and cookies), and project `.env` files. Reviews on
    /// 2026-10-12 found each readable from a refine.
    #[test]
    fn app_tokens_browser_profiles_and_env_files_are_secrets() {
        for c in [
            "~/.codex/**",
            "~/.local/share/com.vercel.cli/**",
            "~/.t3/**",
            "~/.mozilla/**",
            "~/snap/firefox/common/.mozilla/**",
            "~/.config/google-chrome/**",
            "~/.config/chromium/**",
        ] {
            assert!(CREDENTIALS.contains(&c), "{c}");
        }
        assert_eq!(SECRET_GLOBS, ["~/Projects/**/.env*"]);
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert!(v["permissions"]["deny"].as_array().unwrap().iter().any(|d| d == "Read(~/Projects/**/.env*)"));
        let deny_read = v["sandbox"]["filesystem"]["denyRead"].as_array().unwrap();
        assert!(deny_read.iter().any(|d| d.as_str().is_some_and(|s| s.ends_with("/Projects/**/.env*") && s.starts_with('/'))), "{deny_read:?}");
    }
```

In `the_session_may_search_the_web_but_not_read_credentials`, change the length assertion to:

```rust
        assert_eq!(deny.len(), CREDENTIALS.len() + SECRET_GLOBS.len() + hidden.len());
```

In `the_sandbox_fences_the_session_to_the_task_database`, the `denyRead` assertion becomes the hidden paths followed by the expanded globs. Assert that it starts with `["/run/user/1000", "/run/docker.sock"]` and has `2 + SECRET_GLOBS.len()` entries.

- [ ] **Step 2: Run it and check that it fails**

Run: `cargo test --lib refine::tests::app_tokens_browser_profiles_and_env_files_are_secrets`
Expected: FAIL to compile, because `SECRET_GLOBS` doesn't exist yet.

- [ ] **Step 3: Extend `CREDENTIALS` and add `SECRET_GLOBS`**

```rust
pub const CREDENTIALS: &[&str] = &[
    "~/.ssh/**",
    "~/.gnupg/**",
    "~/.aws/**",
    "~/.config/gh/**",
    "~/.docker/**",
    "~/.kube/**",
    "~/.netrc",
    "~/.git-credentials",
    "~/.claude/.credentials.json",
    "~/.local/share/keyrings/**",
    "~/.codex/**",
    "~/.local/share/com.vercel.cli/**",
    "~/.t3/**",
    "~/.mozilla/**",
    "~/snap/firefox/common/.mozilla/**",
    "~/.config/google-chrome/**",
    "~/.config/chromium/**",
];

/// Secrets found by pattern, not by a fixed path: the `.env` files of
/// every project under `~/Projects`, which hold API keys and database
/// URLs. Hidden from the Read tool by a deny rule and from sandboxed
/// Bash by `denyRead`, which takes globs on Linux (it does not for
/// writes). A refine loses nothing it needs: a project's `.env.example`
/// goes too, and the code that reads the variables still shows their names.
pub const SECRET_GLOBS: &[&str] = &["~/Projects/**/.env*"];
```

Extend the doc comment on `CREDENTIALS` to say that it covers app tokens and browser profiles, which hold saved logins and session cookies.

`credential_paths` already skips paths that don't exist, so the sandbox never gets a missing folder. `~/.codex` also holds the Codex IPC socket, so hiding it closes that socket too.

- [ ] **Step 4: Add `~/.cache/ibus` to `hidden_paths`**

```rust
        home.join(".config/herdr"),
        home.join(".cache/ibus"),
```

Extend `hidden_paths`'s doc comment: `` `~/.cache/ibus` holds the input method's private bus, which can type into other windows. ``

- [ ] **Step 5: Put `SECRET_GLOBS` into `session_settings`**

After the `deny` vector is built:

```rust
    deny.extend(SECRET_GLOBS.iter().map(|g| format!("Read({g})")));
```

`denyRead` becomes the hidden paths plus the globs expanded under `fence.home`:

```rust
    let mut deny_read: Vec<String> = fence.hidden.iter().map(|p| p.display().to_string()).collect();
    deny_read.extend(SECRET_GLOBS.iter().map(|g| fence.home.join(g.trim_start_matches("~/")).display().to_string()));
```

Then use `"denyRead": deny_read` in place of `&fence.hidden`.

**If `ENV_GLOB` was `ignored`:** leave the glob out of `denyRead`. Keep only the `Read(...)` deny, which still covers the Read tool. Name the residual in the ADR addendum: sandboxed Bash can still `cat` a project `.env`. Report it to the user as an open gap. Do not walk `~/Projects` to list `.env` files at launch, because the list would go stale.

- [ ] **Step 6: Run the tests**

Run: `cargo test`
Expected: all pass. `credential_paths_resolve_under_home_and_skip_what_is_absent` still passes unchanged, because the new entries don't exist in its temp home.

- [ ] **Step 7: Check the live machine**

```bash
cargo run --quiet -- task refine --help >/dev/null   # builds
```

Then start and close a real Refine tab, or wait for Task 8's probe, to confirm that `ensure_no_exposed_sockets` doesn't refuse to start. If it refuses on some new socket, hide that socket's folder in `hidden_paths` the same way and note it in the commit.

- [ ] **Step 8: Commit**

```bash
git add src/refine.rs
git commit -m "fix(refine): hide app tokens, browser profiles and .env files" -m "Reviews found the Codex and Vercel CLIs' tokens, t3's keys, Firefox's
saved logins and project .env secrets readable from a refine, and the
ibus bus socket in reach. Hide them from the Read tool and from
sandboxed Bash before the session is given wider network access.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Allow Bash bare, under the sandbox

**Files:**
- Modify: `src/refine.rs`: the `let allow = …` line in `session_settings`, and its doc comment.
- Modify: `src/session/herdr.rs`: the "What keeps it from prompting" paragraph of `agent_start_claude_refiner`.
- Test: `src/refine.rs`: `the_session_may_search_the_web_but_not_read_credentials`.

**Interfaces:**
- Consumes: Tasks 2 and 3, which must be committed first.
- Produces: `permissions.allow` is exactly `["WebSearch", "WebFetch", "Bash", "Read(//**)"]`.

- [ ] **Step 1: Change the test first**

Rename the test to `the_session_may_run_any_sandboxed_command_but_not_read_secrets`, and replace its doc comment and allow assertions:

```rust
    /// Every shell command runs unasked, of any shape: a bare `Bash` rule,
    /// because the sandbox is the fence, not the rules. Claude Code's
    /// command-shape checks otherwise deny loops and `$( )` when there is
    /// no one to ask, even with auto-allow (ADR 0002, 2026-10-12). No
    /// scoped `Bash(…)` rule is left beside it. Web search and fetch, and
    /// the Read tool anywhere (`//` is the absolute form), as sandboxed
    /// Bash can. Deny rules win over the allows: secrets, and every path
    /// the sandbox hides from Bash.
```

```rust
        for tool in ["WebSearch", "WebFetch", "Bash", "Read(//**)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert_eq!(allow.len(), 4, "nothing else runs unasked");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.starts_with("Bash("))), "no scoped Bash rule beside the bare one");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.starts_with("WebFetch(domain:"))), "hosts come from the sandbox's own list");
```

Keep the existing deny assertions, and the `python3` and `skills` ones.

- [ ] **Step 2: Run it and check that it fails**

Run: `cargo test --lib refine::tests::the_session_may_run_any_sandboxed_command_but_not_read_secrets`
Expected: FAIL, with the panic message `Bash allowed`.

- [ ] **Step 3: Change the allow list**

```rust
    let allow = ["WebSearch", "WebFetch", "Bash", "Read(//**)"];
```

- [ ] **Step 4: Rewrite the doc comment's allow paragraph**

In `session_settings`'s doc comment, replace the paragraph from `/// Allowed unasked on top: web search and fetch, which write nothing; and` through `/// folders.` with:

```rust
/// Allowed unasked: every Bash command (`Bash`, bare), since the sandbox
/// is the fence and the rules only say so. Auto-allow alone leaves
/// Claude Code's own command-shape checks in place (a variable, quoted
/// text, a `;` chain), and with no one to ask, as in an unattended run,
/// they deny (ADR 0002, 2026-10-12). Bare `Bash` opens no way out of the
/// sandbox: `allowUnsandboxedCommands` is false and there are no
/// `excludedCommands`. Also web search and fetch.
/// The Read tool may read anywhere (`Read(//**)`: `//` is Claude Code's
/// absolute form, one `/` would be the project's), as sandboxed Bash
/// already can. Deny rules win over it: the [`CREDENTIALS`], the
/// [`SECRET_GLOBS`], and every path `hidden` hides from sandboxed Bash,
/// so the tool opens nothing Bash could not read already.
```

In `claude_unattended_argv`'s doc comment, replace `/// gives; the sandbox already runs every command unasked.` with `/// gives; the settings already allow every sandboxed command and read.`

- [ ] **Step 5: Update the herdr doc comment**

In `src/session/herdr.rs`, replace:

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
/// anywhere but the task database and their own temp folder, and allow
/// rules that state that fence (any Bash command, the Read tool anywhere
/// but secrets and hidden paths, the web), so nothing the sandbox permits
/// is asked about.
```

- [ ] **Step 6: Run the tests, and commit**

Run: `cargo test`. Expected: all pass. `grep -rn 'Bash(task' src/` should print nothing.

```bash
git add src/refine.rs src/session/herdr.rs
git commit -m "fix(refine): allow every sandboxed Bash command unasked" -m "Auto-allow left Claude Code's command-shape checks in place, so a
headless refine was denied loops, \$( ) and quoted text that the sandbox
would have run safely. A bare Bash rule states the fence the OS already
enforces, for the tab and the unattended run alike, and replaces
Bash(task *).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Let Bash reach a fixed list of source and docs hosts

**Files:**
- Modify: `src/refine.rs`: a new `NETWORK_HOSTS` const, and `session_settings`'s `network` block and doc comment.
- Test: `src/refine.rs`: `the_sandbox_fences_the_session_to_the_task_database`, plus a new test.

**Interfaces:**
- Consumes: Tasks 2–4.
- Produces:
  - `pub const NETWORK_HOSTS: &[&str]`
  - `sandbox.network == {"allowAllUnixSockets": true, "allowedDomains": [<each host>:443], "strictAllowlist": true}`

- [ ] **Step 1: Write the failing test**

```rust
    /// Bash may reach a fixed list of hosts that serve public source and
    /// docs, on 443 alone, and every other host is refused, not asked
    /// about, in a tab as headless. No wildcard, no IP: allowing every
    /// host would reach loopback, Docker and the LAN by IP literal, which
    /// the sandbox's local-address guard does not cover.
    #[test]
    fn bash_reaches_only_the_listed_hosts_on_443() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let net = &v["sandbox"]["network"];
        assert_eq!(net["strictAllowlist"], true);
        assert_eq!(net["allowAllUnixSockets"], true);
        let hosts: Vec<&str> = net["allowedDomains"].as_array().unwrap().iter().map(|h| h.as_str().unwrap()).collect();
        assert_eq!(hosts.len(), NETWORK_HOSTS.len());
        for h in &hosts {
            assert!(h.ends_with(":443"), "{h}");
            assert!(!h.contains('*'), "{h}");
            assert!(!h.trim_end_matches(":443").chars().all(|c| c.is_ascii_digit() || c == '.'), "no IP literal: {h}");
        }
        for h in ["github.com:443", "raw.githubusercontent.com:443", "crates.io:443", "herdr.dev:443"] {
            assert!(hosts.contains(&h), "{h}");
        }
    }
```

- [ ] **Step 2: Run it and check that it fails**

Run: `cargo test --lib refine::tests::bash_reaches_only_the_listed_hosts_on_443`
Expected: FAIL to compile, because `NETWORK_HOSTS` doesn't exist yet.

- [ ] **Step 3: Add the hosts and the network block**

Put this next to `CREDENTIALS`:

```rust
/// The hosts sandboxed Bash may reach, on port 443 alone: public source
/// and docs, for `curl` of a raw file or a shallow `git clone` into the
/// session's temp folder. Every other host is refused outright
/// (`strictAllowlist`), not asked about. Never a wildcard or an IP, and
/// never every host: the sandbox's guard against local addresses skips IP
/// literals, so an open list reaches loopback, Docker and the LAN.
/// WebFetch is separate and reaches any host, as it always has.
pub const NETWORK_HOSTS: &[&str] = &[
    "github.com",
    "api.github.com",
    "raw.githubusercontent.com",
    "codeload.github.com",
    "objects.githubusercontent.com",
    "gitlab.com",
    "crates.io",
    "static.crates.io",
    "index.crates.io",
    "docs.rs",
    "pypi.org",
    "files.pythonhosted.org",
    "registry.npmjs.org",
    "herdr.dev",
];
```

In `session_settings`:

```rust
    let hosts: Vec<String> = NETWORK_HOSTS.iter().map(|h| format!("{h}:443")).collect();
```

```rust
            "network": { "allowAllUnixSockets": true, "allowedDomains": hosts, "strictAllowlist": true },
```

Add this paragraph to the doc comment:

```rust
/// Bash's network is [`NETWORK_HOSTS`] on 443 and nothing else, refused
/// rather than asked about (`strictAllowlist`, which Claude Code honours
/// from `--settings`). That comes after the secrets are hidden and the
/// hooks and shared temp folder are fenced off, since a page the session
/// reads can tell it to fetch and run something.
```

- [ ] **Step 4: Run the tests, and commit**

Run: `cargo test`. Expected: all pass.

```bash
git add src/refine.rs
git commit -m "feat(refine): let sandboxed Bash fetch from source and docs hosts" -m "Every curl or git clone a refine tried failed for want of network. A
strict list of public source and docs hosts on 443 lets it read
upstream code and docs, and refuses every other host rather than
prompting; allowing every host was rejected, as IP literals reach
loopback and the LAN.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Log and announce an unattended run's denials

**Files:**
- Modify: `src/refine.rs`
  - `claude_unattended_argv`
  - `run_unattended`
  - `unattended_steps`
  - `run_claude`
  - New `Denial`, `RunResult`, `parse_run`, `denied_sentence`, `with_denials` and `append_raw`
- Test: `src/refine.rs`
  - `an_unattended_run_is_the_tabs_claude_with_no_one_to_ask`
  - `a_run_logs_its_output_under_its_step_and_fails_on_a_bad_exit`
  - New tests

**Interfaces:**
- Consumes: nothing from Tasks 2–5, apart from the signatures they changed in `unattended_steps`.
- Produces (all private to `refine.rs`):
  - `struct Denial { tool: String, input: String }` (`Debug, Clone, PartialEq, Eq`)
  - `struct RunResult { text: String, denials: Vec<Denial> }` (`Debug, Default, PartialEq, Eq`)
  - `fn parse_run(stdout: &str) -> Option<RunResult>`
  - `fn denied_sentence(denials: &[Denial]) -> Option<String>`
  - `fn with_denials(message: &str, denials: &[Denial]) -> String`
  - `fn append_raw(log: &Path, text: &str)`
  - `fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String], denials: &mut Vec<Denial>) -> Result<()>` (plus `tmp` if Task 2 Step 6's fallback applied)
  - `unattended_steps(…, log: &Path, denials: &mut Vec<Denial>) -> Result<()>`
  - `claude_unattended_argv` ends with `["--permission-prompts", "none", "--output-format", "json"]`

On 2.1.296, `claude -p --output-format json` prints one object. Its `"result"` is the final text (a string), and its `"permission_denials"` is an array of `{"tool_name": "Bash", "tool_use_id": "…", "tool_input": {…}}`. A sandbox network refusal shows up there as the tool `SandboxNetworkAccess`.

- [ ] **Step 1: Write the failing tests**

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

In `an_unattended_run_is_the_tabs_claude_with_no_one_to_ask`, change the doc comment's first sentence and the tail assertion:

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
Expected: compile errors.

- [ ] **Step 3: Add the parsing and wording helpers after `run_claude`**

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

Put this next to `append_line`:

```rust
/// Add `text` to the log as it is, with no line added. Best effort, as
/// [`append_line`].
fn append_raw(log: &Path, text: &str) {
    if let Ok(mut file) = File::options().append(true).create(true).open(log) {
        let _ = file.write_all(text.as_bytes());
    }
}
```

- [ ] **Step 4: Rewrite `run_claude`**

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

`Command::output` keeps a stdio that was set explicitly, so stderr still goes to the log while stdout is captured. If Task 2 Step 6's fallback applied, keep its `tmp` parameter and `.env(…)` line.

- [ ] **Step 5: Add the flag to the argv**

```rust
    argv.extend(["--permission-prompts", "none", "--output-format", "json"].map(String::from));
```

Add this to its doc comment, after "is refused rather than waited on.": `` `--output-format json` makes the run print its result as JSON, refusals included, for [`run_claude`] to log. ``

- [ ] **Step 6: Thread the denials through**

`unattended_steps` gains `denials: &mut Vec<Denial>`, and both `run_claude(…)` calls pass `denials` as their last argument. In `run_unattended`:

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

In `run_unattended`'s doc comment, after "Both runs' output goes to one log ([`log_path`]), which a failure's error names;", add: `` any action Claude was refused is listed there too and counted in the notification or the error, with the log's path. ``

- [ ] **Step 7: Run the tests, and commit**

Run: `cargo test`. Expected: all pass, with no warnings.

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

### Task 7: Skills and docs

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md`: the sandbox paragraph (about line 31) and the `auto` bullet (about lines 98–113).
- Modify: `.claude/skills/report-task/SKILL.md`: the sandbox paragraph (about line 29) and the unattended paragraph (about lines 87–99).
- Modify: `README.md`: the requirements paragraph (about line 207) and the Add & all paragraph (about lines 405–416).
- Modify: `llms.txt`: line 44.

**Interfaces:**
- Consumes: the facts of Tasks 2–6.
- Produces: nothing.

- [ ] **Step 1: Both skills' sandbox paragraph**

In both files, replace:

```markdown
Your Bash commands run in a sandbox that can read anything but write only to
the task database. A command that fails with `Read-only file system` or
`Operation not permitted` hit that fence on purpose: do not look for a way
around it, and do not ask the user to lift it.
```

with:

```markdown
Your Bash commands run in a sandbox that can read anything but secrets and
write only to the task database and `$TMPDIR`, which is yours for scratch
files such as a shallow `git clone` of an upstream repository. They can
reach a fixed list of source and docs hosts (GitHub, GitLab, crates.io,
docs.rs, PyPI, npm, herdr.dev) and no others; use WebFetch for any other
page. A command that fails with `Read-only file system`, `Operation not
permitted`, or a refused connection hit that fence on purpose: do not look
for a way around it, and do not ask the user to lift it.
```

- [ ] **Step 2: Take out the shell-shape rules**

In `refine-task/SKILL.md`, the `auto` bullet ends with `Grilling is never unattended. Nobody can answer a permission prompt either, so … which may read anywhere.` Cut it after `Grilling is never unattended.`

In `report-task/SKILL.md`, replace the passage from `Nobody can answer a permission prompt either,` through `anywhere.` with:

```markdown
The run's own log is under `~/.local/share/niri-tasks/unattended/`, named by
the task's first eight characters.
```

Then check:

```bash
grep -n -i 'permission prompt\|pipeline\|\$( )\|loops\|denied' .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md
```

Expected: nothing still tells the model to avoid a shell shape.

- [ ] **Step 3: README**

Find the sentence in the requirements paragraph about the sandbox `that keeps that Claude to reading the project and writing only the task`, and make it read `that keeps that Claude to reading the project, its own temp folder and a fixed list of source and docs hosts, and writing only the task`.

In the Add & all paragraph, after `` `~/.local/share/niri-tasks/unattended/`). `` add:

```markdown
The log holds each run's final answer and every action Claude was refused,
if any, and the notification counts the refusals and gives the log's path.
```

- [ ] **Step 4: llms.txt**

In line 44, replace ``with a notification `Planned and reported: …`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log`).`` with:

```text
with a notification `Planned and reported: …`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log`); the log lists each action Claude was refused (`== denied: <tool> <input>`, or `== denied: none`), and the notification counts them (`Claude was refused 2 actions (Bash, Read).`) and gives the log.
```

Keep it as one table line.

- [ ] **Step 5: ADR Consequences note**

The dated addendum from Task 1 already says the hooks escape is closed. Check that it does. Leave the original Consequences text unchanged.

- [ ] **Step 6: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md README.md llms.txt docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "docs(refine): describe the widened fence and drop shape rules" -m "The skills now say what the sandbox allows (secrets hidden, \$TMPDIR,
a fixed list of hosts) and no longer steer the model around prompts the
rules now settle; the README and llms.txt say an unattended run logs and
counts its denials.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Probe the new fence with the real settings

This task changes no code, unless a probe fails. It proves the fence holds and the safe work runs.

**Files:** none.

**Interfaces:**
- Consumes: Tasks 2–6, and the shim in `$D/bin/claude` from Task 1.
- Produces: a pass or fail for each probe, recorded in `$D/fence-probes.md`.

- [ ] **Step 1: Record the new argv**

```bash
cargo build
D=~/.local/share/niri-tasks/reviews/af9e53fc-diagnosis
rm -f "$D"/argv-*.bin
./target/debug/niritasks task add 'chore: Fence probe, delete me'
U=$(task rc.json.array=on +niri_tasks status:pending description~'Fence probe' export | jq -r '.[0].uuid')
PATH="$D/bin:$PATH" ./target/debug/niritasks task refine "$U" --unattended
```

The shim now adds `--output-format stream-json --verbose` after the run's own `--output-format json`, and the last one wins. That's fine, because the log is then the stream.

- [ ] **Step 2: Run the probes**

Each probe re-runs `argv-0` with only the prompt changed. Run them from `~/Projects/niri-tasks`, where the refine's project is read-only:

```bash
cd ~/Projects/niri-tasks
probe() {
  mapfile -d '' -t A < "$D/argv-0.bin"
  A[1]="Run exactly this one Bash command, unchanged, then quote its full output and nothing else: $1"
  /home/paul/.local/bin/claude "${A[@]}" --output-format json 2>&1 | jq -c '{result, denials: .permission_denials}'
}
probe 'for f in src/*.rs; do wc -l "$f"; done | tail -3; echo n=$(ls ~/.task | wc -l)'
probe 'echo TMPDIR=$TMPDIR; ls /tmp/claude-1000 2>&1 | head -2'
probe 'touch src/probe.rs; echo rc=$?'
probe 'touch ~/.task/hooks/on-launch-probe; echo rc=$?'
probe 'curl -sS -o /dev/null -w "%{http_code}\n" https://raw.githubusercontent.com/rust-lang/rust/master/README.md'
probe 'curl -sS -o /dev/null -w "%{http_code}\n" https://example.com/; echo rc=$?'
probe 'curl -sS -m5 http://127.0.0.1:5432/; echo rc=$?'
probe 'cd $TMPDIR && git clone -q --depth 1 https://github.com/sharkdp/fd.git && ls fd | head -3'
probe 'cat ~/.codex/auth.json | head -c 20; echo rc=$?'
cd -
```

Pass means each of these:

| # | Probe | Pass |
|---|---|---|
| 1 | loop and `$( )` | line counts and `n=…`, with `denials` empty |
| 2 | temp folder | `TMPDIR=…/.cache/niri-tasks/refine-tmp/<uuid8>`, and `/tmp/claude-1000` not listed (hidden) |
| 3 | project write | `Read-only file system`, rc non-zero |
| 4 | hooks write | `Read-only file system` or `Operation not permitted`, rc non-zero |
| 5 | listed host | `200` |
| 6 | unlisted host | refused (a 403 or a proxy error, rc non-zero), with no prompt; a `SandboxNetworkAccess` denial is fine |
| 7 | loopback | refused |
| 8 | git clone | lists the clone's files |
| 9 | secret | no such file or permission denied, rc non-zero |

Write each result into `$D/fence-probes.md`. If any probe fails, stop and report it to the user before going on: a failure in 3, 4, 6, 7 or 9 is a hole in the fence.

- [ ] **Step 3: Clean up**

```bash
./target/debug/niritasks task status "$U" deleted --yes
```

---

### Task 9: Verify, land, check end to end, and close the tasks

**Files:** none, unless a check fails.

**Interfaces:**
- Consumes: Tasks 1–8.
- Produces: the task's **Done when** checked, and `a67de29b` and `b268d068` closed.

- [ ] **Step 1: Automated checks**

```bash
cargo test
(cd claude/refine-mod && claude plugin test)
```

Expected: both pass. If `claude plugin test` needs other arguments, follow how `tests/all.sh` calls it.

- [ ] **Step 2: Land**

Use the `finish-worktree` skill. Hold `af9e53fc`'s completion until Steps 3–4 pass: if the skill completes it first, reopen it with `niritasks task status af9e53fc stopped`. Make sure the installed `niritasks` is rebuilt from `main`.

- [ ] **Step 3: Attended check, with the user**

```bash
niritasks task add 'docs: Describe how ~/.task and the unattended logs are laid out'
U=$(task rc.json.array=on +niri_tasks status:pending description~'unattended logs are laid out' export | jq -r '.[0].uuid')
niritasks task note "$U" 'Context: list ~/.task and ~/.local/share/niri-tasks/unattended, count the lines of each src/*.rs file with a for loop and $( ), read a log under ~/.worktrees, and fetch https://herdr.dev/llms.txt with curl'
niritasks task refine "$U"
```

Ask the user to watch the tab. Pass: no permission prompt at all, and the only question is the write tool's. Then ask the tab to run `echo $TMPDIR`. Pass: `…/.cache/niri-tasks/refine-tmp/<uuid8>`, not `/tmp/claude-1000`. If it shows `/tmp/claude-1000`, the export did not reach Claude: stop and report this to the user before closing anything.

- [ ] **Step 4: Unattended check**

```bash
niritasks task refine "$U" --unattended
LOG=$(ls -t ~/.local/share/niri-tasks/unattended/"${U:0:8}"-*.log | head -1)
grep -n '^== ' "$LOG"
```

Pass:
- Both steps show `== denied: none`.
- The notification reads `Planned and reported: …` with no refusal sentence.
- The report step read the run log, as seen in its transcript under `~/.claude/projects/-home-paul-Projects-niri-tasks/`. If it had no reason to read the log, say so.

- [ ] **Step 5: Clean up and close**

```bash
niritasks task status "$U" deleted --yes
niritasks task note a67de29b-8ce4-47db-a1a7-5709d66cfc75 'Superseded by af9e53fc: refine stays in default mode with the sandbox in auto-allow, a bare Bash rule and a strict host list; see docs/adr/0002, 2026-10-12.'
niritasks task status a67de29b-8ce4-47db-a1a7-5709d66cfc75 completed
niritasks task note 82ff74c6-f7f3-49d5-95db-b597a16cb6fa 'Superseded by af9e53fc, which replaced the shell-shape skill rules with a bare Bash allow.'
niritasks task note b268d068 'Fixed by af9e53fc: denyWrite on ~/.task/hooks, the folder made before each session.'
niritasks task status b268d068 completed
niritasks task status af9e53fc completed
```
