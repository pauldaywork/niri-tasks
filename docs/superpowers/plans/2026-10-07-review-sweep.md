# Review Sweep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close out the first section of the 2026-10-07 architecture review (v2): drop the dead fuzzel dependency, the two subcommands nobody calls, the shell-script differential tests, make the daemon the owner of every GUI window, and read the task database path from Taskwarrior.

**Architecture:** Pure removals plus two small consolidations. After it, the `niritasks` CLI is the scriptable surface and the only process that draws anything is `niritasks daemon`; the CLI asks it over the existing IPC socket and reports one error when it is not running. Nothing new is designed here.

**Tech Stack:** Rust (clap, gtk4), bash e2e scripts, Taskwarrior.

**Spec:** `/tmp/architecture-review-20261007-205517.html`, sections 1 (N1, R4, R5+R6, D4). The decisions behind it are in the Q&A recorded in `/home/paul/.claude/plans/robust-roaming-phoenix.md`.

## Global Constraints

- Commit messages are Conventional Commits, `<type>(<scope>): <summary>`, imperative, lowercase, no full stop, subject ≤ 72 chars, body wrapped at 72 saying what and why. End every commit message with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- `cargo test` must pass after every task. Two of its tests read README.md's `## Commands` block and llms.txt and fail if any clap subcommand or long flag is not shown being run (`src/main.rs`, `readme_commands_block_runs_every_subcommand_and_flag`, `llms_txt_runs_every_subcommand_and_flag`). Removing a subcommand therefore also means removing its lines from both files in the same task.
- Work in the `review-sweep` worktree branch and commit per task; `finish-worktree` lands it on `main` at the end.
- Do not touch `docs/superpowers/plans/*` other than this file, and do not touch `.ua/`.
- Keep comments in the house style: full sentences, say why, no bullet lists inside code comments.

---

### Task 1: Delete the fuzzel dependency (N1)

**Files:**
- Delete: `src/picker.rs`, `fuzzel/picker.ini`
- Modify: `src/lib.rs:17`, `install.sh:64-66, 134`, `README.md:157, 168, 368-370`, `llms.txt:5`
- Modify (comments only): `src/project.rs:1-16, 35, 58-59, 137`, `src/github.rs:105`, `src/panel/surface.rs:135`, `src/panel/state.rs:126-130, 2046-2048, 2090`, `src/taskbox.rs:1-21`, `src/refine.rs:312-313`

**Interfaces:**
- Consumes: nothing.
- Produces: no `picker` module; `fuzzel` is no longer a runtime dependency.

- [ ] **Step 1: Prove `picker` has no callers**

Run: `grep -rn "picker::\|use crate::picker\|Picker" src tests --include=*.rs --include=*.sh | grep -v "^src/picker.rs"`
Expected: no output.

- [ ] **Step 2: Delete the module and the theme**

```bash
git rm src/picker.rs fuzzel/picker.ini
```

In `src/lib.rs` delete the line `pub mod picker;`.

- [ ] **Step 3: Installer**

In `install.sh` delete the whole section 3 (the three lines from `# ─── 3. the picker theme` through `link "$REPO/fuzzel/picker.ini" "$CONFIG/fuzzel/picker.ini"`, and the blank line after it). Renumber the remaining section headers 4→3, 5→4, 6→5, 7→6. Change the dependency loop to:

```bash
for dep in niri task ghostty; do
```

- [ ] **Step 4: README and llms.txt**

`README.md` line 157: change `symlinks the niri include, the fuzzel picker theme and` to `symlinks the niri include and`.

`README.md` line 168: change `` `niri`, `taskwarrior`, `fuzzel`, `ghostty` 1.2 or later `` to `` `niri`, `taskwarrior`, `ghostty` 1.2 or later ``.

`README.md` lines 368-370 (the e2e-panel paragraph): replace
```
Run it with `NIRITASKS=$PWD/target/debug/niritasks`: without that it runs the
installed `niritasks`, which may be an older build whose `project open` still
runs fuzzel.
```
with
```
Run it with `NIRITASKS=$PWD/target/debug/niritasks`: without that it runs the
installed `niritasks`, which may be an older build.
```

`llms.txt` line 5: change `` `niritasks` wraps Taskwarrior (`task`), niri, fuzzel and ghostty, and optionally herdr, Claude Code and worktrunk (`wt`). `` to `` `niritasks` wraps Taskwarrior (`task`), niri and ghostty, and optionally fzf, herdr, Claude Code and worktrunk (`wt`). ``

- [ ] **Step 5: Comments that explain fuzzel to nobody**

`src/project.rs` lines 1-12, replace the module doc's first three paragraphs with:

```rust
//! Project folders: the `~/Projects` list, its name normalisation, the list a
//! task can move to, and moving it.
//!
//! A name typed on the project list that matches no folder becomes a
//! directory name, so it is normalised and then guarded. The re-check after
//! normalisation matters: typing "my project" may well have just become an
//! existing "my-project", in which case the list should open that rather than
//! fail to create it.
```
(keep the `//! Also here: which programs …` line that follows.)

`src/project.rs:16` `/// What the picker decided to do with the text the user accepted.` → `/// What to do with a name typed on the project list.`
`src/project.rs:35` `/// Resolve accepted picker text against the list of existing project folders.` → `/// Resolve a typed name against the list of existing project folders.`
`src/project.rs:58-59`: `// Guard the cases that would write outside ~/Projects, or make a folder the` / `// picker can never show again (it filters dotfiles from its list).` → `// Guard the cases that would write outside ~/Projects, or make a folder the` / `// list can never show again (it leaves dotfiles out).`
`src/project.rs:137` `/// Shared by the project picker and a card's Move to workspace, so the two` → `/// Shared by the project list and a card's Move to workspace, so the two`

`src/github.rs:105` `/// Fire-and-forget refresh of the cache for the *next* picker open.` → `/// Fire-and-forget refresh of the cache for the *next* project list.`

`src/panel/surface.rs:135` `//! It is held with exclusive keyboard mode throughout, as fuzzel does. niri` → `//! It is held with exclusive keyboard mode throughout. niri`

`src/panel/state.rs:126-130`: in that doc comment replace `A` / `/// new name makes a folder, as fuzzel echoed text that matched no row,` with `/// new name makes a folder,` (keep the rest of the sentence grammatical: "…once nothing shows. A new name makes a folder, and a name already a folder opens it, …").
`src/panel/state.rs:2047` `/// for the folder "my-project". Enter still opens it, as fuzzel did, and` → `/// for the folder "my-project". Enter still opens it, and`
`src/panel/state.rs:2090` `/// While anything matches, Enter takes the match, as fuzzel did.` → `/// While anything matches, Enter takes the match.`

`src/taskbox.rs` lines 1-21, replace the module doc with:

```rust
//! The task box: one window for adding a task and for editing one.
//!
//! It is a plain fixed-size window, not layer-shell. niri auto-floats windows
//! whose minimum and maximum sizes are equal, so setting both is all it takes
//! to get a floating box — no window rule needed.
//!
//! Its description is a wrapping text area and its notes are a list of rows,
//! one per note, each editable in place with its date at its right end and an
//! × to delete it. Add, Edit and `task note` all open this same window;
//! they differ only in what is filled in and where the cursor starts.
//!
//! The daemon serves the box when it is running, and the CLI builds its own
//! when it is not, so the box works either way.
```
(Task 5 rewrites the last paragraph again; write it as above here.)

`src/refine.rs:312-313` `// Through niri, so the window lands on the focused workspace —` / `// the one the task belongs to — as the project picker's does.` → `// Through niri, so the window lands on the focused workspace —` / `// the one the task belongs to — as the project list's does.`

- [ ] **Step 6: Verify nothing mentions fuzzel outside history**

Run: `grep -rni fuzzel src tests niri install.sh llms.txt README.md CONTEXT.md Cargo.toml; cargo test 2>&1 | tail -3`
Expected: no grep output; tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A src install.sh README.md llms.txt fuzzel
git commit -m "chore: drop fuzzel, which nothing runs any more

The task picker, the action menu and the project picker all moved into
the task panel, so src/picker.rs had no callers and fuzzel/picker.ini
was a theme no process read. Stop linking and requiring it, and reword
the comments that explained fuzzel's echo to a reader who will never
meet it.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: Retire the shell-pipeline differential tests (R6)

**Files:**
- Delete: `tests/differential.rs`
- Modify: `src/tag.rs` (tests + lines 9-14), `src/text.rs` (delete `trim`, its test, lines 9-12 and 26), `src/notify.rs:3`, `src/niri.rs:3`, `src/lib.rs:3-5`, `README.md:333, 352, 472-475`

**Interfaces:**
- Produces: `text::trim` no longer exists. `tag::workspace_tag` is unchanged.

- [ ] **Step 1: Write the corpus test in tag.rs**

Add to the `tests` module of `src/tag.rs`, after `non_ascii_folds_to_underscore`:

```rust
    /// The awkward names the shell version was checked against, kept as a
    /// table so the folding rule stays pinned to the cases that once broke it.
    #[test]
    fn the_awkward_name_corpus_folds_the_same() {
        let cases = [
            ("general", "general"),
            ("ubuntu-setup", "ubuntu_setup"),
            ("Ubuntu-Setup", "ubuntu_setup"),
            ("UBUNTU SETUP", "ubuntu_setup"),
            ("with.dots", "with_dots"),
            ("with:colons", "with_colons"),
            ("trailing-", "trailing"),
            ("-leading", "leading"),
            ("MiXeD CaSe-Thing", "mixed_case_thing"),
            ("tabs\tand spaces", "tabs_and_spaces"),
            ("under_score-dash mix", "under_score_dash_mix"),
        ];
        for (name, tag) in cases {
            assert_eq!(workspace_tag(name), tag, "{name:?}");
        }
    }
```

- [ ] **Step 2: Run it**

Run: `cargo test --lib tag::`
Expected: all tag tests pass, including the new one (it pins current behaviour; if any case fails, the expectation is wrong, not the code — check the rule in `workspace_tag` and fix the table).

- [ ] **Step 3: Add the tab case to project::normalize's test**

In `src/project.rs`, the test containing `assert_eq!(normalize("my project"), "my-project");` (around line 374): add one line after `assert_eq!(normalize("  padded  "), "padded");`:

```rust
        assert_eq!(normalize("tabs\tand spaces"), "tabs-and-spaces");
```

- [ ] **Step 4: Delete the differential file and text::trim**

```bash
git rm tests/differential.rs
```

In `src/text.rs` delete the `trim` function with its doc comment (lines 18-22) and the test `trims_surrounding_whitespace`. Rewrite the module doc lines 6-12 to:

```rust
//! * **Adding** a task word-splits the description, so taskwarrior's own
//!   attribute syntax works — "ship the release due:friday priority:H" sets a
//!   due date and a priority rather than becoming part of the description.
//! * **Editing** and **annotating** do *not* split. The text is passed as one
//!   argument after `--`, so a typed `due:` stays literal text.
```
and lines 14-16 (`In Rust the split is explicit …`) to:
```rust
//! The split is explicit rather than a shell side effect, so arguments never
//! hit a glob expander.
```
and the `collapse_whitespace` doc (lines 24-28) to:
```rust
/// Collapse any run of whitespace to a single space, then trim.
///
/// Taskwarrior descriptions are single-line, and the box is multi-line, so a
/// pasted newline has to go somewhere.
```
Also in `text.rs`'s tests: the comment above `add_splits_so_taskwarrior_attributes_parse` (`/// The behaviour `set -f` + unquoted expansion bought in the shell version:`) → `/// Attributes reach `task` as their own arguments.`; the comment above `add_does_not_glob` → `/// A description containing `*` is never expanded against the current` / `/// directory: argv is passed directly, so there is no glob stage at all.`

- [ ] **Step 5: The other stale citations**

`src/tag.rs:9-14`: replace
```rust
//! Ported from `task-lib.sh:workspace_tag`, which was:
//!     tr '[:upper:]' '[:lower:]'
//!     | sed -e 's/[^a-z0-9_]\+/_/g' -e 's/^_\+//' -e 's/_\+$//'
//!
//! Note the order: lowercasing happens *first*, so an uppercase letter becomes
//! its lowercase self rather than an underscore.
```
with
```rust
//! The rule: lowercase first, then every run of characters outside
//! `[a-z0-9_]` becomes one underscore, then leading and trailing underscores
//! go. The order matters: lowercasing happens *first*, so an uppercase letter
//! becomes its lowercase self rather than an underscore.
```
`src/tag.rs` test `non_ascii_folds_to_underscore` comment: `// The shell version's [^a-z0-9_] is ASCII-only; keep that behaviour so` → `// The rule is ASCII-only, so`.

`src/notify.rs:3-4`: `//! Ported from `task_notify` / `notify` in the shell scripts. Failure to notify` → `//! Failure to notify`.

`src/niri.rs:3-4`: replace `//! The shell scripts shelled out to `niri msg -j …` and piped the result` / `//! through `jq`. This replaces both: one socket connection, typed responses.` with `//! One socket connection, typed responses, instead of `niri msg -j …` and `jq`.`

`src/lib.rs:3-5`: replace with
```rust
//! The logic lives here rather than in `main.rs` so it can be exercised by
//! integration tests.
```

`README.md:333`: `cargo test                  # unit, differential, write-path        ~2s` → `cargo test                  # unit and write-path                   ~2s`
`README.md:352`: `` | `cargo test` | `taskwarrior` on `$PATH`, and `bash` for the differential suite | `` → `` | `cargo test` | `taskwarrior` on `$PATH` | ``
`README.md:472-475`: delete the paragraph beginning `` `tests/differential.rs` runs the original shell pipelines `` and the blank line before it.

- [ ] **Step 6: Verify**

Run: `cargo test 2>&1 | tail -5; grep -rn "differential\|task-lib.sh\|open_project_workspace.sh\|task-add-text.sh\|TaskBoxModal" src tests README.md llms.txt`
Expected: tests pass; grep prints nothing.

- [ ] **Step 7: Commit**

```bash
git add -A src tests README.md
git commit -m "test: pin the tag corpus in tag.rs and drop the shell differential

tests/differential.rs compared the Rust rules against bash pipelines
copied from scripts that left the repo long ago, so it no longer tested
a specification, only GNU sed. Its awkward-name corpus now lives beside
the rule it pins, text::trim loses its only caller, and the comments
stop pointing readers at files they cannot open.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: Drop `task active` and the `workspace default` subcommand (R5)

**Files:**
- Modify: `src/main.rs` (TaskCommand::Active 74-75 and its arm 288-299; `Command::Workspace`, `WorkspaceCommand` 221-224, `workspace_command`, the dispatch arm; the test `workspaces_are_named_only_by_opening_a_project`), `niri/niri-tasks.kdl:14-17`, `src/lib.rs:148-153`, `README.md:230, 260`, `llms.txt:9, 26, 48`

**Interfaces:**
- Produces: `niritasks task active` and `niritasks workspace default` no longer parse. `niri_tasks::workspace_default()` stays, called only by `daemon::run`. `task::active_for_tag` stays (write_path.rs uses it to test `set_active`).

- [ ] **Step 1: Make the test say what the CLI should refuse**

In `src/main.rs` tests, change `workspaces_are_named_only_by_opening_a_project` to:

```rust
    /// A workspace gets its name by opening a project on it, so there is no
    /// naming one freely; workspace 1's startup name is the daemon's job.
    #[test]
    fn workspaces_are_named_only_by_opening_a_project() {
        assert!(Cli::try_parse_from(["niritasks", "workspace", "rename"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "workspace", "new"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "workspace", "default"]).is_err());
    }

    /// The panel shows active tasks; nothing reads them off the CLI.
    #[test]
    fn task_active_is_no_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "active"]).is_err());
    }
```

Run: `cargo test --bin niritasks workspaces_are_named task_active_is`
Expected: FAIL (both subcommands still parse).

- [ ] **Step 2: Remove the subcommands**

In `src/main.rs`:
- Delete the `Active` variant (`/// Print each active task's description, one per line, or nothing` + `Active,`).
- Delete the `TaskCommand::Active => { … }` arm and the comment block above it (the three lines starting `// "Nothing" covers every uninteresting case identically`).
- Delete the `Workspace(WorkspaceCommand)` variant with its doc, the whole `enum WorkspaceCommand`, the `Command::Workspace(c) => return workspace_command(c),` arm, and `fn workspace_command`.
- `niri` may now be unused in the `use niri_tasks::{…}` list: remove it only if `cargo build` warns.

- [ ] **Step 3: kdl, lib doc, README, llms**

`niri/niri-tasks.kdl`: delete lines 14-17 (the comment `// Name workspace 1 on startup …` through `spawn-at-startup "niritasks" "workspace" "default"`) and the blank line after.

`src/lib.rs` `workspace_default` doc: replace `/// Run once at startup. Naming matters …` paragraph's last line `/// Lives here rather than in the binary because the daemon runs it too.` with `/// The daemon runs it once as it starts.`

`README.md`: delete line 230 (`niritasks task active …`) and line 260 (`niritasks workspace default …`).
`llms.txt`: in rule 1 (line 9) delete the sentence `` `task active` follows focus too. ``; delete the table rows for `niritasks task active` and `niritasks workspace default`.

- [ ] **Step 4: Verify**

Run: `cargo test 2>&1 | tail -5; grep -n "task active\|workspace default" README.md llms.txt CONTEXT.md .claude/skills/*/SKILL.md src/main.rs`
Expected: tests pass (including the README/llms coverage tests); grep prints nothing.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/lib.rs niri/niri-tasks.kdl README.md llms.txt
git commit -m "feat!: drop task active and the workspace default subcommand

task active was a status-bar readout with no bar left to read it. The
daemon already names workspace 1 as it starts, so niri's spawn-at-startup
line and the CLI subcommand did the same job a second time. Workspaces
are now named only by opening a project, and the daemon names the first.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: Watch the task database where Taskwarrior says it is (D4)

**Files:**
- Modify: `src/daemon.rs:44-68` (State, data_path, task_db_mtimes), `:128` (State construction), `:306-340` (tick), tests `data_paths_follow_taskdata`

**Interfaces:**
- Consumes: `task::data_location() -> Result<PathBuf>` (`src/task.rs:312`), which runs `task _get rc.data.location` and so honours `TASKDATA` and `.taskrc`.
- Produces: `fn db_mtimes(dir: &Path) -> [Option<SystemTime>; 2]`, pure given a directory.

- [ ] **Step 1: Write the failing test**

Replace `data_paths_follow_taskdata` in `src/daemon.rs` tests with:

```rust
    /// The tick watches pending.data and completed.data inside the directory
    /// taskwarrior names; a file not there yet reads as no mtime.
    #[test]
    fn db_mtimes_read_both_files_in_the_data_dir() {
        let dir = std::env::temp_dir().join(format!("niri-tasks-mtimes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pending.data"), b"").unwrap();
        let [pending, completed] = db_mtimes(&dir);
        assert!(pending.is_some(), "pending.data exists, so it has an mtime");
        assert!(completed.is_none(), "completed.data is not there yet");
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

Run: `cargo test --lib daemon::`
Expected: FAIL to compile, `db_mtimes` not found.

- [ ] **Step 2: Implement**

In `src/daemon.rs`, replace `data_path` and `task_db_mtimes` (lines 53-68) with:

```rust
/// The mtimes of `pending.data` and `completed.data` in `dir`. Both: editing,
/// noting or removing a finished task writes `completed.data` alone, and its
/// card on the Finished tab has to follow.
fn db_mtimes(dir: &std::path::Path) -> [Option<SystemTime>; 2] {
    ["pending.data", "completed.data"]
        .map(|file| std::fs::metadata(dir.join(file)).ok()?.modified().ok())
}
```

Add a field to `State`:

```rust
    /// Where taskwarrior keeps its data, asked of taskwarrior itself once,
    /// so a `.taskrc` that moves it is honoured. None when `task` could not
    /// say, in which case every tick redraws.
    data_dir: Option<std::path::PathBuf>,
```

At `let state = Rc::new(RefCell::new(State::default()));` (in `build`) change to:

```rust
    let data_dir = task::data_location()
        .map_err(|e| eprintln!("could not find the task database: {e}"))
        .ok();
    let state = Rc::new(RefCell::new(State { data_dir, ..State::default() }));
```

In `tick`, replace `let mtimes = task_db_mtimes();` with:

```rust
    let mtimes = state
        .borrow()
        .data_dir
        .as_deref()
        .map(db_mtimes)
        .unwrap_or_default();
```

Update the module doc at lines 13-15 so it reads `2. the mtimes of taskwarrior's `pending.data` and `completed.data`, in the directory taskwarrior itself names (two stats).` Update the `State` doc on `task_mtimes` to `/// [`db_mtimes`] at the last tick.`

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib daemon::`
Expected: PASS.

- [ ] **Step 4: Check the sandboxed e2e still finds its database**

`tests/e2e-panel.sh` starts the nested daemon with `TASKDATA` set; `task _get rc.data.location` returns that directory, so the watch still follows it. Run, if a Wayland session and python3-pil are available: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -5`. Expected: `failed: 0`. If the machine cannot run it, note the skip in the task report rather than claiming it ran.

- [ ] **Step 5: Commit**

```bash
git add src/daemon.rs
git commit -m "fix(daemon): watch the task database where taskwarrior keeps it

The tick hand-built ~/.task/pending.data and completed.data, so a
.taskrc that moved data.location left the panel watching files nothing
wrote and the cards went stale until a workspace change forced a tick.
Ask taskwarrior for the directory once at start, as the refine sandbox
already does through task::data_location.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: The daemon owns every window (R4)

**Files:**
- Modify: `src/ipc.rs:1-13, 78-86`, `src/main.rs` (delegate_to_daemon 19-28, edit_in_box, the Panel/Add/Edit/Note arms, project_open 493-503, the `taskbox` import), `src/taskbox.rs` (delete lines ~140-251: the fast-startup comment block, `prefer_fast_startup`, `fast_startup_overrides`, `screen_reader_enabled`, `A11Y_SCHEMA`, `show`; the three `fast_startup_*` tests; the `open_in` doc at 133-139; the module doc's last paragraph), `src/daemon.rs:160-180` (the comment on `listen` and its error text), `tests/e2e-box.sh:10, 30-38, 426-438`, `README.md:336, 389-395, 417-424`

**Interfaces:**
- Consumes: `ipc::send(&Request) -> Result<()>` and `ipc::Request::{Add{refine}, Edit(uuid), Note(uuid), Panel, Projects}`.
- Produces: `pub const NO_DAEMON: &str` in `ipc.rs`; `ipc::send`'s error Display is that text. `taskbox::show` no longer exists; `taskbox::open_in` is the only way to show the box.

- [ ] **Step 1: Give `ipc::send` the user-facing error**

In `src/ipc.rs`, above `pub fn send`, add:

```rust
/// What every GUI command says when the daemon is not there to draw for it.
pub const NO_DAEMON: &str =
    "The niri-tasks daemon is not running, so there is nothing to draw the window. Start it with `systemctl --user start niri-tasks`.";
```

and change `send` to:

```rust
/// Ask the daemon to draw something. `Err` means no daemon is listening; the
/// daemon is the only process that draws, so the caller reports it rather
/// than drawing itself.
pub fn send(req: &Request) -> Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).context(NO_DAEMON)?;
    writeln!(stream, "{}", req.encode())?;
    stream.flush()?;
    Ok(())
}
```

Rewrite the module doc lines 9-13 (`This is the same shape as the dms ipc call …` through `… one you cannot is a dependency.`) to:

```rust
//! The daemon is the one process that draws: the panels, the box and the
//! project list. The CLI asks it over this socket and, when nothing answers,
//! says so with [`NO_DAEMON`] rather than building a window of its own, which
//! would mean a second copy of what the box does on submit.
```

Add a test in `ipc.rs`'s tests module:

```rust
    /// With no daemon the error is the one sentence every GUI command shows.
    #[test]
    fn no_daemon_is_one_sentence() {
        std::env::set_var("XDG_RUNTIME_DIR", "/nonexistent-niri-tasks-test");
        let err = send(&Request::Panel).unwrap_err();
        assert_eq!(err.to_string(), NO_DAEMON);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
```
(Check the existing ipc tests do not depend on `XDG_RUNTIME_DIR`; if one does, set and restore it the same way there.)

Run: `cargo test --lib ipc::`
Expected: PASS.

- [ ] **Step 2: The CLI asks, and only asks**

In `src/main.rs`:
- Delete `delegate_to_daemon` and its doc (lines 19-28), and `edit_in_box` with its doc.
- `TaskCommand::Panel` arm becomes:
```rust
        // The daemon draws the panel; with none, say so.
        TaskCommand::Panel => ipc::send(&ipc::Request::Panel)?,
```
- In the `Add` arm replace
```rust
            let (description, notes, and_refine) = if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Add { refine: and_refine }) {
                    return Ok(());
                }
                match taskbox::show(taskbox::BoxConfig::add(&tag, and_refine)) {
                    Some(s) => {
                        let notes = s.note_texts();
                        (s.description, notes, s.refine)
                    }
                    None => return Ok(()),
                }
            } else {
                (text::collapse_whitespace(&words.join(" ")), Vec::new(), and_refine)
            };
```
with
```rust
            // With no text, the daemon's box does the adding when it is
            // submitted; this process only asks for it.
            if words.is_empty() {
                return ipc::send(&ipc::Request::Add { refine: and_refine });
            }
            let description = text::collapse_whitespace(&words.join(" "));
            let notes: Vec<String> = Vec::new();
```
and update the comment above the arm (`// With no text, open the box. With text, add straight away …`) to `// With no text, ask the daemon for the box. With text, add straight away —`. Keep the rest of the arm (`if description.is_empty()`, `add_with_notes`, notify, `spawn_quick`) as it is; the `tag` is still needed for the add.
- `Edit` arm: replace the `if words.is_empty() { if delegate_to_daemon(…) { return Ok(()); } return edit_in_box(&uuid, taskbox::Mode::Edit); }` block with `if words.is_empty() { return ipc::send(&ipc::Request::Edit(uuid)); }`. Same for `Note` with `ipc::Request::Note(uuid)`.
- `project_open`: replace
```rust
        anyhow::ensure!(
            delegate_to_daemon(ipc::Request::Projects),
            "The niri-tasks daemon is not running, so there is no project list. Start it with `systemctl --user start niri-tasks`."
        );
        return Ok(());
```
with `return ipc::send(&ipc::Request::Projects);`.
- Remove `taskbox` from the `use niri_tasks::{…}` list (and anything else `cargo build` reports unused).

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test --bin niritasks 2>&1 | tail -3`
Expected: no warnings or errors; the CLI tests pass.

- [ ] **Step 3: The box has one way in**

In `src/taskbox.rs`:
- Delete `show` and its doc, `prefer_fast_startup` and the long comment block above it (from `/// Two environment overrides …` or wherever that block starts, down to the end of `screen_reader_enabled`), `fast_startup_overrides`, `A11Y_SCHEMA`.
- Delete the tests `fast_startup_sets_both_when_nothing_is_set_or_listening`, `fast_startup_leaves_an_explicit_choice_alone`, `a_running_screen_reader_keeps_the_accessibility_bridge`.
- `open_in`'s doc (lines 133-139) mentions `show`: rewrite to `/// Open the box inside the daemon's running Application, calling `on_submit` with what was saved. The window is built once, in `build_window`.` (keep whatever else in that doc is still true).
- Module doc: replace the last paragraph (`The daemon serves the box when it is running, and the CLI builds its own …`) with `//! Only the daemon opens it, over IPC, so the submit logic exists once.`
- Check `Rc`, `RefCell`, `OnceCell` imports are still used (the STYLE cell uses OnceCell; `build_window` uses Rc); remove any `cargo build` reports unused.

In `src/daemon.rs` around line 160, change the comment `// Serve task-box requests. … Failing to listen is not fatal: the CLI falls back to building the box itself, which is the whole reason this is a cache and not a dependency.` to `// Serve the CLI's requests: the box, the panel's keyboard, the project list. Failing to listen is not fatal for the panels, but every GUI command will report no daemon until it is fixed.` and the `eprintln!` text to `"IPC unavailable ({e}); task add, edit, note, panel and project open will report no daemon"`.

Run: `cargo test 2>&1 | tail -3; grep -rn "taskbox::show\|prefer_fast_startup\|edit_in_box\|delegate_to_daemon" src tests`
Expected: pass; grep prints nothing.

- [ ] **Step 4: e2e-box runs once, and checks the refusal**

In `tests/e2e-box.sh` replace the tail (from `# Both paths matter:` to the end) with:

```bash
nested_daemon_start "$SB/daemon.err"
run_suite "served by the daemon"
run_refine_suite "served by the daemon"
nested_daemon_stop
sleep 1

# With no daemon in the nested niri there is nothing to draw the box, and the
# CLI says so instead of building one: the daemon is the one process that
# draws, so a window with no daemon would be a bug, not a fallback.
echo
echo "=== no daemon ==="
if out=$("${NENV[@]}" "$NIRITASKS" task add 2>&1); then
    bad "task add with no daemon exited 0 (output: $out)"
else
    case "$out" in
        *"daemon is not running"*) ok "task add with no daemon says the daemon is not running" ;;
        *) bad "task add with no daemon said: $out" ;;
    esac
fi
[ -z "$(box_id)" ] && ok "and opened no box" || bad "a box opened with no daemon"

nested_service_untouched
summary
[ "$fail" -eq 0 ]
```

Header comments: line 10 area says nothing about the fallback, leave it. Lines 35-38: replace `Your` / `# own niri-tasks daemon keeps running: the fallback half runs with no daemon` / `# inside the nested niri, which is where the box looks for one. The task` / `# database is a sandbox's.` with `# own niri-tasks daemon keeps running: the box looks for a daemon inside the` / `# nested niri, where one is started for the run. The task database is a` / `# sandbox's.` (keep `Your` at the end of the preceding line).

- [ ] **Step 5: README**

`README.md:336`: `bash tests/e2e-box.sh       # the task box, in a nested niri         ~65s` → `~35s`.

`README.md:389-395` paragraph: replace `It` / `proves both ways the box opens: served by a daemon running inside the nested` / `niri, then with none running there, when the CLI builds the box itself. Your` / `own `niri-tasks` daemon is never stopped.` with `The box is served by a daemon started inside the nested niri; once that is stopped, it checks the CLI refuses with "daemon is not running" rather than drawing a box of its own. Your own `niri-tasks` daemon is never stopped.` (rewrap to ~80 columns).

`README.md:417-424`: in the paragraph under `### Why the three scripts are not cargo tests`, replace `It runs the whole thing twice,` / `inside a nested niri of its own, once served by a daemon there and once with` / `none, because the fallback is the reason this tool does not depend on a daemon.` with `It runs inside a nested niri of its own, served by a daemon started there, because the daemon is the one process that draws.` (rewrap).

- [ ] **Step 6: Run the box e2e if the machine can**

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-box.sh 2>&1 | tail -8`
Expected: `failed: 0`, with the two new `no daemon` PASS lines. If wtype or a Wayland session is missing, say so in the report; do not claim it ran.

- [ ] **Step 7: Commit**

```bash
git add src/ipc.rs src/main.rs src/taskbox.rs src/daemon.rs tests/e2e-box.sh README.md
git commit -m "feat!: let the daemon own the task box as it owns the panel

The project list already refused to open without the daemon; the task
box still built a GTK window inside the CLI when none answered, with a
second copy of the add-and-refine and edit logic and a 0.6s to 2.6s
start. Every GUI command now asks the daemon over IPC and reports one
sentence when it is not running. e2e-box runs once and checks that
refusal instead of the fallback.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: Record the decisions (ADR 0003)

**Files:**
- Create: `docs/adr/0003-the-daemon-draws-and-the-cli-asks.md`
- Modify: `llms.txt` (the `## Optional` list gains the ADR), `CONTEXT.md` (no term changes needed; verify "picker" appears only as an _Avoid_)

- [ ] **Step 1: Write the ADR**

```markdown
# The daemon draws; the CLI asks

`niritasks daemon` is the one process that puts anything on screen: the task
panel on each monitor, the task box, and the project list. Every `niritasks`
command that needs a window sends one line over `$XDG_RUNTIME_DIR/niri-tasks.sock`
and, when nothing answers, exits 1 with one sentence saying the daemon is not
running. The CLI never builds a window itself.

Until 2026-10-07 the box was the exception: the CLI built its own GTK window
when no daemon answered, on the argument that "a daemon you can do without is
a cache; one you cannot is a dependency". That argument stopped holding once
the panel, which cannot exist without the daemon, became the only way to act
on a task, and the project list moved into it. Keeping the fallback meant two
copies of the submit logic and a slow path nobody used.

The same day fuzzel left: the task picker and the action menu had already
folded into the panel's action row, and the project picker became the panel's
project list, ranked by fzf when it is installed and by substring match when
it is not. Workspaces are named only by opening a project
(`niritasks project open`), never renamed; the daemon names workspace 1
`general` at start.

## Considered options

- **Keep the CLI-built box as a fallback.** Rejected: the daemon is a
  dependency anyway, and the fallback duplicated add-and-refine and edit.
- **Rebuild the project picker as a separate daemon popup window.** Not
  taken: the panel already had the list machinery (cards, focus, keys), so
  the list became a mode of the panel instead.
- **Keep fuzzel for the project list.** Rejected once the panel list worked:
  one GUI toolkit, and typed rows instead of a string echoed back.

## Consequences

The panel and the box are only ever drawn by one GTK process, so styles,
fonts and the blur are set once. A machine without the daemon running gets
the same sentence from every GUI command. e2e-box runs once, served by a
daemon inside the nested niri, and checks the refusal rather than a fallback.
```

- [ ] **Step 2: Link it from llms.txt**

In `llms.txt`'s `## Optional` list add, after the ADR 0001 line:

```
- [docs/adr/0003-the-daemon-draws-and-the-cli-asks.md](docs/adr/0003-the-daemon-draws-and-the-cli-asks.md): why only the daemon draws windows, and why fuzzel left
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test 2>&1 | tail -3` (the llms shape test still passes: `## Optional` stays last).

```bash
git add docs/adr/0003-the-daemon-draws-and-the-cli-asks.md llms.txt
git commit -m "docs(adr): record that the daemon draws and the cli asks

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## Self-review notes

- Spec coverage: N1 → Task 1; R6 → Task 2; R5 → Task 3; D4 → Task 4; R4 → Task 5; the report's ADR note → Task 6. The report's order was N1 → R6 → R5 → D4 → R4; the tasks follow it.
- Type consistency: `ipc::NO_DAEMON` and `ipc::send` are defined in Task 5 step 1 and used in steps 2 and 4 (the e2e matches on the substring "daemon is not running", which is in `NO_DAEMON`). `db_mtimes(dir: &Path)` is defined and used only in Task 4.
- Nothing here depends on the panel deepening that follows in a separate plan.
