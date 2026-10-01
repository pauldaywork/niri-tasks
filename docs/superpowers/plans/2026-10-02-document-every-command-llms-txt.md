# Document Every niritasks Command, and llms.txt — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent or a person can learn to drive `niritasks` correctly from the repo alone: every subcommand and flag is documented in `--help`, in README's Commands block and in a new `llms.txt`, and a cargo test fails if any of the three falls behind the CLI.

**Architecture:** The CLI is the clap derive enums in `src/main.rs` (`Command`, `TaskCommand`, `WorkspaceCommand`, `ProjectCommand`). `src/main.rs` already has a `#[cfg(test)] mod tests` that uses the private `Cli` type. The new tests go there too. They walk `Cli::command()` (`clap::CommandFactory`) for every leaf subcommand and its arguments, then check three things: each has `--help` text, README's Commands block runs each subcommand and flag, and `llms.txt` does the same and has the llmstxt.org shape.

**Tech Stack:** Rust, clap 4 derive (`CommandFactory`, `Command::get_subcommands`, `Arg::get_long`, `Arg::get_help`), Markdown.

**Spec:** Taskwarrior task `6a970973-2475-4ec4-8fcf-246d2bdb75b7`. Read it with `task rc.json.array=on 6a970973-2475-4ec4-8fcf-246d2bdb75b7 export`. Its description and notes are the spec.

## Global Constraints

- Goal: an agent or person can learn to drive `niritasks` correctly from the repo alone, with every subcommand and flag documented and `llms.txt` as the entry point.
- Name the file `llms.txt` (not `llm.txt`) and put it at the repo root.
- `llms.txt` is self-contained: the full command reference is inline, with links to README.md, CONTEXT.md and the skills for depth.
- `llms.txt` shape (llmstxt.org): an H1 name, a blockquote summary, free paragraphs, then H2 sections of `[name](url): note` link lists, ending with an `## Optional` section agents may skip.
- Spell out the rules agents get wrong: `tag --session` not bare `tag` in a terminal; `add` word-splits but `edit`/`note` don't; address tasks by uuid or uuid8; change status via `niritasks task status`, never `task done`; `deleted` needs `--yes`; which commands open a GUI and which are scriptable.
- Mark internal commands as internal rather than hiding them: `daemon` (systemd unit) and `task start --here --workspace` (run inside the tab `task start` opens).
- A cargo test fails when a clap subcommand is missing from README's Commands block or from `llms.txt`.
- Done when: README and `niritasks <cmd> --help` cover every subcommand and flag, `llms.txt` is at the repo root in the llmstxt.org shape, and `cargo test` passes, drift test included.
- Out of scope: serving `llms.txt` on a website, an `llms-full.txt`, and installing it anywhere via `install.sh`.
- House style: comments say *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Findings and decisions made while planning (flag any you disagree with)

1. **The tests live in `src/main.rs`'s existing `mod tests`.** `Cli` is private to the binary crate, so an integration test under `tests/` cannot see it. The module already parses argv with `Cli::try_parse_from`. Run the tests with `cargo test --bin niritasks`.
2. **A "mention" means a line that *runs* the command.** A subcommand counts as documented once a line contains `niritasks <path>` as a whole command (`niritasks task get` does not count for `get-text`). A flag counts once a line runs its subcommand *with* that flag. This is why README's `niritasks workspace new|rename|default` becomes three lines, and why `--yes` gets its own `niritasks task status <uuid> deleted --yes` line. At the moment it is only mentioned on a continuation comment.
3. **`--help` gets a test of its own: no subcommand or argument may have blank help.** The positional `<UUID>`, `<TEXT>` and `<STATE>` arguments are blank today (`niritasks task start --help` shows `<UUID>` with nothing beside it). That is the place where `--help` "says less than the README".
4. **The files are read at test time with `std::fs`, not `include_str!`.** With `include_str!`, a missing `llms.txt` would be a compile error, which would take down every test in the binary. With `fs`, it is one clean failure.
5. **The `llms.txt` command reference sits in the free section, as a table and paragraphs, not under headings.** The llmstxt.org format allows anything *except headings* between the blockquote and the first H2. The H2 sections are reserved for link lists. So the reference uses bold lead-ins and a table, and the H2s are `## Docs`, `## Skills` and `## Optional`.
6. **The links are relative paths** (`README.md`, `.claude/skills/finish-worktree/SKILL.md`). There is no website (see Out of scope), and an agent reading `llms.txt` is reading it in the repo, where relative paths resolve. GitHub URLs would point at `main`, not at the checkout being read.
7. **One rule beyond the spec's list, because it sits next to rule 1:** `task add`, `task active`, `task list`, `task menu`, `refine`, `start` and `session` all take the workspace from focus, the same as bare `tag`. An agent filing a task from a terminal should use `task add "+$(niritasks tag --session)" -- "<text>"`, which is what the `workspace-tasks` skill already does. The commands that take only a uuid (`status`, `get-text`, `edit`, `get-notes`, `note`) work from anywhere.
8. **`niritasks` has no JSON export, and `llms.txt` says so.** It points at `task rc.json.array=on <uuid8> export`, the command the skills already use.

---

### Task 1: `--help` text for every subcommand and argument

**Files:**
- Modify: `src/main.rs` — the `Command` enum (lines 35–64), the `TaskCommand` enum (lines 66–125), and `mod tests` (from line 717)

**Interfaces:**
- Produces (in `src/main.rs` `mod tests`, used by Tasks 2 and 3):
  - `fn leaf_commands() -> Vec<(String, clap::Command)>`: every runnable subcommand as its words (`"task get-text"`), paired with its `clap::Command`. Groups (`task`, `workspace`, `project`) are walked into and never listed. clap's own `help` subcommand is skipped.
  - `fn own_args(cmd: &clap::Command) -> impl Iterator<Item = &clap::Arg>`: the command's arguments without clap's `help`/`version`.

- [ ] **Step 1: Write the failing test**

Add `use clap::CommandFactory;` under `use super::*;` at the top of `mod tests` in `src/main.rs`. Then add these at the end of the module:

```rust
    /// Every subcommand a person can run, as the words that run it ("task
    /// get-text"), with the command itself. Groups like `task` are walked
    /// into rather than listed: on their own they only print help.
    fn leaf_commands() -> Vec<(String, clap::Command)> {
        fn walk(prefix: &str, cmd: &clap::Command, out: &mut Vec<(String, clap::Command)>) {
            for sub in cmd.get_subcommands().filter(|s| s.get_name() != "help") {
                let path = if prefix.is_empty() {
                    sub.get_name().to_string()
                } else {
                    format!("{prefix} {}", sub.get_name())
                };
                if sub.has_subcommands() {
                    walk(&path, sub, out);
                } else {
                    out.push((path, sub.clone()));
                }
            }
        }
        let mut out = Vec::new();
        walk("", &Cli::command(), &mut out);
        out
    }

    /// The arguments a person passes, not the --help and --version clap adds.
    fn own_args(cmd: &clap::Command) -> impl Iterator<Item = &clap::Arg> {
        cmd.get_arguments()
            .filter(|a| !matches!(a.get_id().as_str(), "help" | "version"))
    }

    /// `niritasks <cmd> --help` is the first reference a script or an agent
    /// reaches for, so nothing in it may come out blank: not a subcommand,
    /// and not a bare `<UUID>` with nothing beside it.
    #[test]
    fn every_subcommand_and_argument_has_help() {
        let mut blank = Vec::new();
        for (path, cmd) in leaf_commands() {
            if cmd.get_about().is_none() {
                blank.push(path.clone());
            }
            for arg in own_args(&cmd) {
                if arg.get_help().is_none() {
                    blank.push(format!("{path} <{}>", arg.get_id()));
                }
            }
        }
        assert!(blank.is_empty(), "no --help text for: {blank:?}");
    }
```

- [ ] **Step 2: Run the test to make sure it fails**

Run: `cargo test --bin niritasks every_subcommand_and_argument_has_help`
Expected: FAIL. The message lists the blank positionals: `task menu <uuid>`, `task status <uuid>`, `task status <state>`, `task add <text>`, `task get-text <uuid>`, `task edit <uuid>`, `task edit <text>`, `task get-notes <uuid>`, `task note <uuid>`, `task note <text>`, `task refine <uuid>`, `task start <uuid>`, `task session <uuid>`.

- [ ] **Step 3: Rewrite the `Command` enum's doc comments**

Replace the `Tag` variant's doc and the `Daemon` variant's doc in `enum Command` (leave `Task`, `Workspace`, `Project` and `Terminal` as they are):

```rust
    /// Print the focused workspace's task tag, or exit 1 if it has none
    ///
    /// From a terminal, a script or anything that runs for more than a
    /// moment, pass --session: the user switches workspace while it runs,
    /// and the focused answer moves with them.
    Tag {
        /// Take the workspace from this terminal's herdr session, or failing
        /// that its ~/Projects folder, rather than from whatever is focused.
        /// For anything long-running: focus moves, the terminal's own workspace
        /// does not.
        #[arg(long)]
        session: bool,
    },
```

```rust
    /// Internal: run the task panels and the task-box server (long-running; the niri-tasks systemd user unit starts it)
    Daemon,
```

- [ ] **Step 4: Rewrite the `TaskCommand` enum**

Replace the whole `enum TaskCommand { … }` (lines 66–125) with the version below. Variant names, field names and `#[arg]` attributes stay the same. Only the `///` text changes, and the inline fields are spread over lines so they can carry docs.

```rust
#[derive(Subcommand)]
enum TaskCommand {
    /// Print each active task's description, one per line, or nothing
    Active,
    /// Pick a task from this workspace and act on it (opens fuzzel)
    List {
        /// Print the rows that would be shown, instead of opening the picker.
        /// Exists so the list can be diffed against the shell original without
        /// a GUI in the way.
        #[arg(long)]
        dry_run: bool,
    },
    /// Slide out the task panel and pick a task with the keyboard (Mod+Alt+Ctrl+T)
    Panel,
    /// Open the action menu for one task, as clicking its task card does (opens fuzzel)
    Menu {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
    /// Move a task to a state, as the menu's "Update status" does
    ///
    /// The same states, code and notification as the menu, for scripts and for
    /// finishing a task's worktree. Use it rather than `task <uuid> done`,
    /// `start` or `stop`: `active` also links the herdr pane it runs in to the
    /// task, and every change sends the menu's notification.
    Status {
        /// The task's uuid, or its first 8 characters, as in a
        /// `task/<slug>-<uuid8>` branch
        uuid: String,
        /// Where to move it; `stopped` also brings back a waiting task
        state: task::Status,
        /// Confirm `deleted`, which the menu asks about and a script cannot be asked
        #[arg(long)]
        yes: bool,
    },
    /// Add a task to the focused workspace, or open the task box with no text
    ///
    /// The text is word-split, so taskwarrior reads its own attributes in it:
    /// `niritasks task add ship it due:friday` sets a due date. With no text
    /// it opens the task box, a window, instead.
    Add {
        /// The description, taskwarrior attributes and all; leave it out for the task box
        text: Vec<String>,
    },
    /// Print a task's description
    GetText {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
    /// Replace a task's description, or open the task box on it with no text
    ///
    /// The text is not word-split: it becomes the description as typed, so a
    /// `due:friday` in it stays literal. With no text it opens the task box,
    /// a window, on the task's description and notes.
    Edit {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// The new description, kept literal; leave it out for the task box
        text: Vec<String>,
    },
    /// Print a task's notes, one per line: its date, two spaces, its text
    GetNotes {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
    /// Attach a note to a task, or open the task box on a new note with no text
    ///
    /// The text is not word-split, so a `due:friday` in it stays literal.
    /// With no text it opens the task box, a window, with the cursor in a new
    /// empty note.
    Note {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// The note, kept literal; leave it out for the task box
        text: Vec<String>,
    },
    /// Work a task up into a plan with Claude, in a new tab of the workspace's herdr session
    Refine {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Interview first, via the `grilling` skill, rather than drafting straight away
        #[arg(long)]
        grill: bool,
    },
    /// Start working on a task in its own git worktree, with Claude planning it
    ///
    /// Opens a herdr tab that makes the worktree (branch
    /// `task/<slug>-<uuid8>`) and starts Claude in it. Run again on the same
    /// task, it goes back to both.
    Start {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Internal: the setup step, run inside the tab `task start` opens:
        /// make the worktree, open it, start Claude, close the tab
        #[arg(long, requires = "workspace")]
        here: bool,
        /// Internal: the workspace the task belongs to (only with --here,
        /// which runs inside herdr where niri's focus says nothing about it)
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Go to the Claude working on a task, in the workspace's herdr session
    Session {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
}
```

- [ ] **Step 5: Run the test to make sure it passes, and look at the help it produced**

Run: `cargo test --bin niritasks every_subcommand_and_argument_has_help`
Expected: PASS.

Run: `cargo run -q -- task edit --help && cargo run -q -- task status --help && cargo run -q -- daemon --help`
Expected: `<UUID>` and `<TEXT>` each have text beside them. `edit --help` includes the "not word-split" paragraph. `status --help` lists `[possible values: active, stopped, waiting, completed, deleted]` under `<STATE>`'s new text. `daemon --help` starts with `Internal:`.

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`
Expected: all pass. This includes the existing `every_card_button_is_a_command_the_cli_accepts`, which shows that the argv shapes did not change.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "Give every niritasks subcommand and argument its --help text, and test that none is blank

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: README's Commands block covers every subcommand and flag

**Files:**
- Modify: `src/main.rs` — `mod tests` (append)
- Modify: `README.md` — the `## Commands` section (lines 147–174)

**Interfaces:**
- Consumes: `leaf_commands()` and `own_args()` from Task 1.
- Produces (in `src/main.rs` `mod tests`, used by Task 3):
  - `fn repo_file(name: &str) -> String`: a file at the repo root, read at test time. It panics with the file's name if the file is missing.
  - `fn undocumented(lines: &[&str]) -> Vec<String>`: every subcommand path (`"task get-text"`) and every `"<path> --<flag>"` that no line in `lines` runs. An empty vec means everything is covered.

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `src/main.rs`:

```rust
    /// A file at the repo root, read when the test runs. A missing file
    /// is one failing test, where include_str! would stop every test from
    /// compiling.
    fn repo_file(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {name}: {e}"))
    }

    /// Whether `line` runs `niritasks <path>` as a whole command, so that
    /// `niritasks task get` would not count for `task get-text`.
    fn mentions(line: &str, path: &str) -> bool {
        let needle = format!("niritasks {path}");
        line.match_indices(&needle).any(|(i, _)| {
            line[i + needle.len()..]
                .chars()
                .next()
                .map_or(true, |c| !(c.is_alphanumeric() || c == '-'))
        })
    }

    /// Every subcommand, and every flag, that no line in `lines` shows being
    /// run. A subcommand counts once some line runs it. A flag counts once
    /// some line runs its subcommand with that flag, so it is documented in
    /// use and not only named in passing.
    fn undocumented(lines: &[&str]) -> Vec<String> {
        let mut missing = Vec::new();
        for (path, cmd) in leaf_commands() {
            let runs: Vec<&str> = lines.iter().copied().filter(|l| mentions(l, &path)).collect();
            if runs.is_empty() {
                missing.push(path);
                continue;
            }
            for long in own_args(&cmd).filter_map(|a| a.get_long()) {
                let flag = format!("--{long}");
                if !runs.iter().any(|l| l.contains(&flag)) {
                    missing.push(format!("{path} {flag}"));
                }
            }
        }
        missing
    }

    /// The README's Commands block is where a person looks to find out what
    /// the CLI can do. It had already fallen behind the CLI once (get-text,
    /// get-notes and daemon were all missing), so it is checked against clap.
    #[test]
    fn readme_commands_block_runs_every_subcommand_and_flag() {
        let readme = repo_file("README.md");
        let after = readme
            .split_once("\n## Commands\n")
            .expect("README has a ## Commands section")
            .1;
        let block = after
            .split_once("```\n")
            .expect("a fenced block under ## Commands")
            .1
            .split_once("```")
            .expect("the fenced block is closed")
            .0;
        let missing = undocumented(&block.lines().collect::<Vec<_>>());
        assert!(missing.is_empty(), "README's Commands block never runs: {missing:?}");
    }
```

- [ ] **Step 2: Run the test to make sure it fails**

Run: `cargo test --bin niritasks readme_commands_block_runs_every_subcommand_and_flag`
Expected: FAIL with `README's Commands block never runs: ["task status --yes", "task get-text", "task get-notes", "task start --here", "task start --workspace", "workspace new", "workspace rename", "workspace default", "daemon"]`. The order follows clap's.

- [ ] **Step 3: Rewrite the Commands section in `README.md`**

Replace everything from the `## Commands` heading down to (not including) `### Two rules that look alike and are not` with the text below. Use a four-backtick outer fence when you copy it, so the inner triple fence stays intact.

````markdown
## Commands

`niritasks` is usable directly, not just from keybinds. [`llms.txt`](llms.txt)
is the same reference written for agents, with the rules they most often get
wrong.

```
niritasks tag                      # the focused workspace's tag
niritasks tag --session            # the tag of the workspace this terminal was opened on
                                   #   (its herdr session, else its ~/Projects folder)
niritasks task active              # each active task's description, one per line, or nothing
niritasks task list                # the picker
niritasks task list --dry-run      # the rows it would show, for scripting and testing
niritasks task panel               # hand the task panel the keyboard (Mod+Alt+Ctrl+T)
niritasks task menu <uuid>         # one task's actions, as a task card click opens them
niritasks task status <uuid> <state>  # the menu's Update status: active|stopped|waiting|completed|deleted
                                   #   (stopped also brings back a waiting task)
niritasks task status <uuid> deleted --yes  # deleted needs --yes, the menu's confirmation
niritasks task add <text>          # honours taskwarrior attributes: due:friday, priority:H
niritasks task add                 #   with no text, the task box (Mod+Alt+T)
niritasks task get-text <uuid>     # print its description
niritasks task edit <uuid> <text>  # replaces the description; attributes stay literal
niritasks task edit <uuid>         #   with no text, the task box on its description and notes
niritasks task get-notes <uuid>    # print its notes, one per line: date, two spaces, text
niritasks task note <uuid> <text>  # attaches an annotation, literal too
niritasks task note <uuid>         #   with no text, the task box with the cursor in a new note
niritasks task refine <uuid>       # work it up into a plan with Claude, in the workspace's herdr session
niritasks task refine <uuid> --grill  #   the same, interviewing you first
niritasks task start <uuid>        # its own worktree (task/<slug>-<uuid8>), opened in the herdr session,
                                   #   with Claude planning it; picked again, back to both
niritasks task start <uuid> --here --workspace <name>
                                   # internal: the setup step, run inside the tab `task start` opens
niritasks task session <uuid>      # back to the Claude working on it (work-/task-<uuid8>) in the herdr session
niritasks workspace new            # create a workspace and name it (Mod+Alt+W)
niritasks workspace rename         # rename the focused workspace, and with it its tag (Mod+Alt+Ctrl+W)
niritasks workspace default        # name workspace 1 "general" if it is unnamed (niri runs it at startup)
niritasks project open             # pick a ~/Projects folder onto its own named workspace (Mod+Alt+P)
niritasks terminal                 # a terminal in the focused workspace's ~/Projects folder (Mod+Return)
niritasks daemon                   # internal: the task panels and task-box server, run by the
                                   #   niri-tasks systemd user unit
```

Every `<uuid>` is the full uuid or its first 8 characters, never a task's
number: numbers are renumbered as tasks complete, so a stale one points at
another task. `niritasks <command> --help` says the same as each line here, at
more length.

````

- [ ] **Step 4: Run the test to make sure it passes**

Run: `cargo test --bin niritasks readme_commands_block_runs_every_subcommand_and_flag`
Expected: PASS.

- [ ] **Step 5: Prove the test catches drift**

Temporarily delete the `niritasks task get-notes <uuid>` line from README and run the test again.
Expected: FAIL naming `"task get-notes"`. Then put the line back by hand (do not use `git checkout`, which would also throw away the uncommitted rewrite) and re-run to PASS. Do not commit the deletion.

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs README.md
git commit -m "List every niritasks subcommand and flag in README's Commands block, and test it against clap

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: `llms.txt` at the repo root

**Files:**
- Create: `llms.txt`
- Modify: `src/main.rs` — `mod tests` (append)

**Interfaces:**
- Consumes: `repo_file()` and `undocumented()` from Task 2.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `src/main.rs`:

```rust
    /// llms.txt is where an agent learns the CLI, so it is held to the same
    /// check as README: every subcommand and flag shown being run.
    #[test]
    fn llms_txt_runs_every_subcommand_and_flag() {
        let llms = repo_file("llms.txt");
        let missing = undocumented(&llms.lines().collect::<Vec<_>>());
        assert!(missing.is_empty(), "llms.txt never runs: {missing:?}");
    }

    /// The llmstxt.org shape: an H1 name first, a blockquote summary next, and
    /// the link-list sections ending with `## Optional`, the one an agent
    /// short of context may skip.
    #[test]
    fn llms_txt_has_the_llmstxt_shape() {
        let llms = repo_file("llms.txt");
        let mut lines = llms.lines().filter(|l| !l.trim().is_empty());
        assert!(lines.next().is_some_and(|l| l.starts_with("# ")), "llms.txt starts with an H1");
        assert!(lines.next().is_some_and(|l| l.starts_with("> ")), "a blockquote follows the H1");
        let h2s: Vec<&str> = llms.lines().filter(|l| l.starts_with("## ")).collect();
        assert_eq!(h2s.last(), Some(&"## Optional"), "the last section is ## Optional");
        assert!(
            !llms.lines().any(|l| l.starts_with("### ")),
            "llms.txt has no H3s: the free section allows no headings, the H2s are link lists"
        );
    }
```

- [ ] **Step 2: Run the tests to make sure they fail**

Run: `cargo test --bin niritasks llms_txt`
Expected: both FAIL with a `reading llms.txt:` panic (the file does not exist).

- [ ] **Step 3: Write `llms.txt`**

Create `llms.txt` at the repo root with exactly this content. Use a four-backtick outer fence when you copy it.

````markdown
# niri-tasks

> Workspace-scoped Taskwarrior for the niri Wayland compositor. The name of a niri workspace is the task tag: a task added on workspace `website` is tagged `+website`, and the list, the task panel and every `niritasks` command show that workspace's tasks and nothing else. This file is the whole command reference for the `niritasks` binary, plus the rules agents most often get wrong.

`niritasks` wraps Taskwarrior (`task`), niri, fuzzel and ghostty, and optionally herdr, Claude Code and worktrunk (`wt`). A workspace's tag is its name lowercased, with every run of other characters folded to `_`, so workspace `niri-tasks` has tag `niri_tasks`. An unnamed workspace has no tag, and every command that needs one refuses on it. A failing command prints its error to stderr, shows it as a desktop notification and exits 1.

**Rules agents get wrong**

1. **In a terminal, use `niritasks tag --session`, not bare `niritasks tag`.** Bare `tag` answers "which workspace is focused right now", and the user switches workspace while you work. `--session` takes the workspace from this terminal's herdr session, or failing that its `~/Projects/<folder>`, neither of which moves. The same goes for every command that acts on "this workspace" (`task add`, `task active`, `task list`, `task menu`, `task refine`, `task start`, `task session`): they follow focus. To file a task from a terminal, tag it yourself: `task add "+$(niritasks tag --session)" -- "<description>"`.
2. **`niritasks task add` word-splits its text; `niritasks task edit` and `niritasks task note` do not.** `niritasks task add ship it due:friday priority:H` sets a due date and a priority. `niritasks task edit <uuid> ship it due:friday` makes `due:friday` part of the description. Getting this backwards fails silently. Prefer `niritasks task note` to raw `task <uuid> annotate`: niritasks passes `--`, and raw `annotate` with text that starts like `file.rs:52` silently sets an attribute and files no note.
3. **Address a task by its uuid or the uuid's first 8 characters**, the `<uuid8>` at the end of a `task/<slug>-<uuid8>` branch. Never use its number: numbers are renumbered as other tasks complete, so a stale one points at another task.
4. **Change a task's status with `niritasks task status <uuid> <state>`, never `task <uuid> done`, `start` or `stop`.** It is the same path the task menu uses. `active` also links the herdr pane you run it in to the task, so its menu can find you, and every change sends the menu's notification. The states are `active`, `stopped` (which also brings back a waiting task), `waiting`, `completed` and `deleted`.
5. **`deleted` needs `--yes`.** `niritasks task status <uuid> deleted --yes` is the menu's confirmation, given up front. Pass it only when the user asked for the task to be deleted.
6. **Never run a GUI command unattended.** The Runs column below says which commands open a window and wait for a person, which open a herdr tab, and which are scriptable. A scriptable command prints, or changes the database, and exits.
7. **`niritasks daemon` and `niritasks task start <uuid> --here --workspace <name>` are internal.** The niri-tasks systemd user unit runs the daemon. `task start` runs `--here` inside the herdr tab it opens. Do not run either yourself.

**Reading tasks.** `niritasks` has no JSON output of its own; read tasks with Taskwarrior: `task rc.json.array=on <uuid8> export` for one task, or `task "+$(niritasks tag --session)" status:pending export` for this workspace's pending tasks. `niritasks task get-text <uuid>` and `niritasks task get-notes <uuid>` print one task's description and notes as plain text.

**Command reference.** Every `<uuid>` is the full uuid or its first 8 characters. `niritasks <command> --help` says the same at more length.

| Command | Does | Runs |
|---|---|---|
| `niritasks tag` | Print the focused workspace's tag; exit 1 if it has none | scriptable, follows focus |
| `niritasks tag --session` | Print the tag of the workspace this terminal belongs to | scriptable |
| `niritasks task active` | Print each active task's description on the focused workspace, one per line, or nothing | scriptable, follows focus |
| `niritasks task list` | The picker: pick one of the focused workspace's tasks, then an action | GUI (fuzzel) |
| `niritasks task list --dry-run` | Print the picker's rows, `<uuid>\t<description>`, and its size on stderr | scriptable, follows focus |
| `niritasks task panel` | Hand the task panel the keyboard (Mod+Alt+Ctrl+T); with no daemon, the picker | GUI |
| `niritasks task menu <uuid>` | One task's action menu, as clicking its task card opens it | GUI (fuzzel) |
| `niritasks task status <uuid> <state>` | Move it to `active`, `stopped`, `waiting` or `completed` | scriptable |
| `niritasks task status <uuid> deleted --yes` | Delete it | scriptable |
| `niritasks task add <text>` | Add a task to the focused workspace; the text is word-split, so `due:friday` is an attribute | scriptable, follows focus |
| `niritasks task add` | With no text, the task box, to type a task and its notes | GUI |
| `niritasks task get-text <uuid>` | Print its description | scriptable |
| `niritasks task edit <uuid> <text>` | Replace its description, as typed | scriptable |
| `niritasks task edit <uuid>` | With no text, the task box on its description and notes | GUI |
| `niritasks task get-notes <uuid>` | Print its notes, one per line: date, two spaces, text | scriptable |
| `niritasks task note <uuid> <text>` | Add a note, as typed | scriptable |
| `niritasks task note <uuid>` | With no text, the task box with the cursor in a new note | GUI |
| `niritasks task refine <uuid>` | Open a tab in the workspace's herdr session with Claude working the task into a plan (the `refine-task` skill) | herdr tab |
| `niritasks task refine <uuid> --grill` | The same, interviewing the user first | herdr tab |
| `niritasks task start <uuid>` | Make the task's own git worktree, branch `task/<slug>-<uuid8>`, and open it in a herdr tab with Claude planning it; run again, go back to both | herdr tab |
| `niritasks task start <uuid> --here --workspace <name>` | Internal: the setup step `task start` runs inside the tab it opens | internal |
| `niritasks task session <uuid>` | Go to the herdr tab of the Claude working on it | herdr tab |
| `niritasks workspace new` | Ask for a name, then make a new workspace with it (Mod+Alt+W) | GUI (fuzzel) |
| `niritasks workspace rename` | Ask for a new name for the focused workspace, which changes its tasks' tag too (Mod+Alt+Ctrl+W) | GUI (fuzzel) |
| `niritasks workspace default` | Name workspace 1 `general` if it is unnamed; niri runs it at startup | scriptable |
| `niritasks project open` | Pick a `~/Projects` folder, put it on its own named workspace, and open a terminal and an editor there (Mod+Alt+P) | GUI (fuzzel, windows) |
| `niritasks terminal` | Open a terminal in the focused workspace's `~/Projects` folder (Mod+Return) | GUI (window) |
| `niritasks daemon` | Internal: the task panels and the task-box server, run by the niri-tasks systemd user unit | internal |

**Finishing work.** A task started with `niritasks task start` gets its own worktree. When the work is done, the `finish-worktree` skill lands it: it rebases, tests, fast-forwards `main`, pushes, runs `niritasks task status <uuid8> completed` and removes the worktree.

## Docs

- [README.md](README.md): install, keybinds, the task panel and task box, requirements, and how to run the tests
- [CONTEXT.md](CONTEXT.md): the vocabulary — workspace tag, focused vs active workspace, active and planned tasks, task panel, task card, picker, task box

## Skills

- [workspace-tasks](.claude/skills/workspace-tasks/SKILL.md): work through every pending task on this terminal's workspace, one at a time, keeping their status honest
- [refine-task](.claude/skills/refine-task/SKILL.md): work one task up into a plan and tag it `+planned`; what `niritasks task refine` starts
- [finish-worktree](.claude/skills/finish-worktree/SKILL.md): land a finished task worktree on `main` and mark the task completed

## Optional

- [docs/adr/0001-task-panel-in-gtk-not-quickshell.md](docs/adr/0001-task-panel-in-gtk-not-quickshell.md): why the task panel is drawn in GTK
- [tests/write_path.rs](tests/write_path.rs): the tests that pin `add` word-splitting and `edit`/`note` staying literal
- [src/main.rs](src/main.rs): the clap definitions every command above comes from
````

- [ ] **Step 4: Run the tests to make sure they pass**

Run: `cargo test --bin niritasks llms_txt`
Expected: both PASS.

- [ ] **Step 5: Check the facts in `llms.txt` against the code**

Each of these must hold. If one does not, fix `llms.txt` to match the code, not the other way round:
- `src/main.rs` `TaskCommand::Status`: `task::get(&uuid)`, then `apply_status`, which calls `link::link_current_pane` on `Active` and `notify::tasks` every time.
- `src/main.rs` `TaskCommand::GetText`/`Edit`/`GetNotes`/`Note`/`Status` never call `require_workspace_tag` (so they work from anywhere). `Add`, `Menu`, `Refine`, `Start` and `Session` do.
- `src/task.rs` `annotate` and `modify_description` pass `--`.
- `niri/niri-tasks.kdl` has `spawn-at-startup "niritasks" "workspace" "default"` and the keybinds named in the table.
- `systemd/niri-tasks.service` runs `niritasks daemon`.

Run: `grep -n 'require_workspace_tag\|apply_status' src/main.rs; grep -n '"--"' src/task.rs; grep -n niritasks niri/niri-tasks.kdl; grep -n ExecStart systemd/niri-tasks.service`

- [ ] **Step 6: Prove the drift test fails on a new subcommand**

Temporarily add this variant to `enum ProjectCommand` in `src/main.rs`:

```rust
    /// Scratch
    Scratch,
```

and this arm to the `match` in `run()`, beside `Command::Project(ProjectCommand::Open)`:

```rust
        Command::Project(ProjectCommand::Scratch) => {}
```

Then run `cargo test --bin niritasks`.
Expected: `readme_commands_block_runs_every_subcommand_and_flag` and `llms_txt_runs_every_subcommand_and_flag` both FAIL naming `"project scratch"`. Remove both lines by hand (not with `git checkout`, which would also throw away this task's uncommitted tests). Check that `git diff src/main.rs` now shows only the two new tests, then re-run to PASS. This is the property the spec asked for.

- [ ] **Step 7: Run the whole suite**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add llms.txt src/main.rs
git commit -m "Add llms.txt, the niritasks command reference and rules for agents, and test it against clap

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
