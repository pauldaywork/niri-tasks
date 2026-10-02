//! niritasks — workspace-scoped Taskwarrior for niri.
//!
//! Every task is tagged with the name of the workspace it was created on, so
//! "my tasks" always means "the tasks for the project I'm looking at". Which
//! tasks you see is decided purely by the niri workspace name, which
//! `niritasks project open` sets to the folder name; ~/Projects is read only to offer
//! folders to pick from — opening one, or moving a task to another.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use niri_ipc::WorkspaceReferenceArg;
use niri_tasks::{
    github, ipc, link, niri, notify, picker::Picker, project, refine, require_workspace_tag, session,
    speak, task, taskbox, text, work,
};

/// Hand the box to the daemon if one is listening.
///
/// The daemon is already a warm GTK process, so its box appears immediately
/// rather than paying ~0.6s (2.6s cold) to start another one. Returning false
/// means no daemon, and the caller builds the box itself — the fallback is the
/// point, since a daemon you cannot do without is a dependency rather than a
/// cache, and needing one was the thing that made the DMS plugin worth removing.
fn delegate_to_daemon(req: ipc::Request) -> bool {
    ipc::send(&req).is_ok()
}

#[derive(Parser)]
#[command(name = "niritasks", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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

    /// Taskwarrior operations scoped to the focused workspace
    #[command(subcommand)]
    Task(TaskCommand),

    /// Workspace naming
    #[command(subcommand)]
    Workspace(WorkspaceCommand),

    /// Project folder operations
    #[command(subcommand)]
    Project(ProjectCommand),

    /// Open a terminal in the focused workspace's ~/Projects folder (Mod+Return)
    Terminal,

    /// Internal: run the task panels and the task-box server (long-running; the niri-tasks systemd user unit starts it)
    Daemon,
}

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
        /// Then refine it straight away with Claude, as the box's Add & refine
        /// does; with no text, the box opens with that as its default (Mod+Alt+Shift+T)
        #[arg(long)]
        refine: bool,
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

    /// Read a task's description and notes aloud; run again to stop
    ///
    /// Claude (Haiku) rewrites the task for listening and a local Kokoro
    /// server speaks it, started in docker if it is not running. It plays in
    /// the background, so this returns at once; run while anything is being
    /// spoken, it stops that instead.
    Speak {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Internal: speak in this process — the background process `task
        /// speak` starts
        #[arg(long)]
        here: bool,
    },

    /// Mark a task up next, or clear the mark from one already up next
    ///
    /// Up next is Taskwarrior's own `+next` tag: the task's card turns yellow
    /// and sits under the active tasks, and its urgency rises by 15, which
    /// lifts it in the picker. Nothing else about the task changes.
    UpNext {
        /// The task's uuid, or its first 8 characters
        uuid: String,
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

#[derive(Subcommand)]
enum WorkspaceCommand {
    /// Create a new workspace and name it
    New,
    /// Rename the focused workspace
    Rename,
    /// Name workspace 1 "general" if it is unnamed (run at startup)
    Default,
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Pick a folder from ~/Projects and put it on its own named workspace
    Open,
}

fn main() {
    if let Err(e) = run() {
        // Errors here are things the caller needs to act on ("name this
        // workspace first"), and who the caller is decides where they have to
        // land: a keybind has no terminal to read stderr, while a script or an
        // agent never sees a desktop notification. So both — see notify::error.
        notify::error(&e.to_string());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Tag { session } => {
            let tag = if session {
                niri_tasks::session_workspace_tag()?
            } else {
                require_workspace_tag()?
            };
            print!("{tag}");
        }
        Command::Task(c) => return task_command(c),
        Command::Workspace(c) => return workspace_command(c),
        Command::Project(ProjectCommand::Open) => return project_open(),
        Command::Terminal => return terminal(),
        Command::Daemon => return niri_tasks::daemon::run(),
    }
    Ok(())
}

/// Open the task box on an existing task and save what comes back. Edit and
/// Note are one window; the mode only says where the cursor starts.
fn edit_in_box(uuid: &str, mode: taskbox::Mode) -> Result<()> {
    let t = task::get(uuid)?.context("task not found")?;
    // The uuid the task was found under, not the one typed: `task edit 4a3f`
    // is a prefix, and replace_text insists on the exact task it opened.
    let uuid = t.uuid.clone();
    if let Some(s) = taskbox::show(taskbox::BoxConfig::for_task(mode, t)) {
        task::replace_text(&uuid, &s.description, &s.notes)?;
    }
    Ok(())
}

fn task_command(cmd: TaskCommand) -> Result<()> {
    match cmd {
        // "Nothing" covers every uninteresting case identically (unnamed
        // workspace, no tasks, none started) because the caller's job is to
        // disappear in all of them rather than explain which one it hit.
        TaskCommand::Active => {
            let Some(name) = niri::focused_workspace_name()? else {
                return Ok(());
            };
            let t = niri_tasks::tag::workspace_tag(&name);
            if t.is_empty() {
                return Ok(());
            }
            for active in task::active_for_tag(&t)? {
                println!("{}", active.description);
            }
        }

        TaskCommand::List { dry_run } => return task_list(dry_run),

        // The daemon draws the panel; without one, the fuzzel list does the job.
        TaskCommand::Panel => {
            if !delegate_to_daemon(ipc::Request::Panel) {
                return task_list(false);
            }
        }

        TaskCommand::Menu { uuid } => {
            let tag = require_workspace_tag()?;
            let t = task::get(&uuid)?.context("task not found")?;
            // The uuid the task was found under, not the one typed: `task menu 4a3f`
            // is a prefix, and task_menu's link derivation needs the exact full uuid.
            let width = niri_tasks::picker::clamp_task_width(t.description.chars().count());
            return task_menu(&tag, t.uuid.clone(), &t.description, t.is_up_next(), width);
        }

        TaskCommand::Status { uuid, state, yes } => {
            anyhow::ensure!(
                yes || state != task::Status::Deleted,
                "Deleting a task needs --yes, the menu's confirmation."
            );
            let t = task::get(&uuid)?.context("task not found")?;
            apply_status(&t.uuid, &t.description, state)?;
        }

        // With no text, open the box. With text, add straight away — which is
        // what makes `niritasks task add ship it due:friday` work from a shell.
        // The binding is `and_refine` because `refine` is the module, imported
        // at the top of this file.
        TaskCommand::Add { text: words, refine: and_refine } => {
            let tag = require_workspace_tag()?;
            // The box can return notes with the description; the shell form has
            // nowhere to type them, so it never does. Which button was pressed
            // decides whether to refine, not how the box was opened.
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

            if description.is_empty() {
                return Ok(());
            }
            let uuid = task::add_with_notes(&tag, &text::add_args(&description), &notes)?;
            notify::tasks(&format!("Added to +{tag}: {description}"));
            if let (true, Some(uuid)) = (and_refine, uuid) {
                refine::spawn_quick(&uuid);
            }
        }

        TaskCommand::GetText { uuid } => {
            let t = task::get(&uuid)?.context("task not found")?;
            println!("{}", t.description);
        }

        TaskCommand::Edit { uuid, text: words } => {
            if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Edit(uuid.clone())) {
                    return Ok(());
                }
                return edit_in_box(&uuid, taskbox::Mode::Edit);
            }
            let description = text::collapse_whitespace(&words.join(" "));
            if description.is_empty() {
                return Ok(());
            }
            task::modify_description(&uuid, &description)?;
        }

        TaskCommand::GetNotes { uuid } => {
            let t = task::get(&uuid)?.context("task not found")?;
            for a in &t.annotations {
                println!("{}", a.line());
            }
        }

        TaskCommand::Note { uuid, text: words } => {
            if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Note(uuid.clone())) {
                    return Ok(());
                }
                return edit_in_box(&uuid, taskbox::Mode::Note);
            }
            let note = text::collapse_whitespace(&words.join(" "));
            if note.is_empty() {
                return Ok(());
            }
            task::annotate(&uuid, &note)?;
        }

        TaskCommand::Speak { uuid, here } => {
            if here {
                return speak::run(&uuid);
            }
            // The uuid as found, not as typed, so the background process
            // looks up the same task the press was on.
            let t = task::get(&uuid)?.with_context(|| format!("No task {uuid}."))?;
            speak::toggle(&t.uuid, &t.description)?;
        }

        TaskCommand::UpNext { uuid } => {
            // The uuid as found, not as typed, and its tags, which say which
            // way to toggle.
            let t = task::get(&uuid)?.context("task not found")?;
            task::set_up_next(&t.uuid, !t.is_up_next())?;
            // The words that were picked, as Update status notifies its state.
            notify::tasks(&format!("{}: {}", task::up_next_label(t.is_up_next()), t.description));
        }

        TaskCommand::Start { uuid, here, workspace } => {
            if here {
                // clap's `requires` guarantees it; the context is for the type.
                let workspace = workspace.context("--here needs --workspace")?;
                return work::set_up_here(&workspace, &uuid);
            }
            require_workspace_tag()?;
            let workspace = niri::focused_workspace_name()?.unwrap_or_default();
            let t = task::get(&uuid)?.context("task not found")?;
            anyhow::ensure!(t.status == "pending", "Only a pending task can be started.");
            work::launch(&workspace, &t)?;
        }

        TaskCommand::Refine { uuid, grill } => {
            // The same refusal every entry point makes: a task refined on an
            // unnamed workspace would have no session to open in.
            require_workspace_tag()?;
            let workspace = niri::focused_workspace_name()?.unwrap_or_default();
            let t = task::get(&uuid)?.context("task not found")?;
            // task::get is unfiltered by status; a completed or deleted task
            // has nothing left to work up into a plan.
            anyhow::ensure!(t.status == "pending", "Only a pending task can be refined.");
            let mode = if grill { refine::Mode::Grill } else { refine::Mode::Quick };
            refine::launch(&workspace, &uuid, &t.description, mode)?;
        }

        TaskCommand::Session { uuid } => {
            require_workspace_tag()?;
            let workspace = niri::focused_workspace_name()?.unwrap_or_default();
            // The uuid as found, not as typed: the agent's name is made from
            // its first eight characters, and a typed prefix may be shorter.
            let t = task::get(&uuid)?.context("task not found")?;
            link::go_to(&workspace, &t.uuid)?;
        }
    }
    Ok(())
}

/// The list is fed to fuzzel as "<uuid>\t<description>" and displayed with
/// --with-nth=2: you read and filter the description, we get back the uuid. Ids
/// would be shorter but they are renumbered as tasks complete, so a stale id
/// can point at the wrong task.
fn task_list(dry_run: bool) -> Result<()> {
    let tag = require_workspace_tag()?;
    let tasks = task::pending_for_tag(&tag)?;

    let rows = niri_tasks::rows::build(&tasks);
    let longest = niri_tasks::rows::longest(&rows);
    let entries: Vec<String> = rows.iter().map(|(u, d)| format!("{u}\t{d}")).collect();

    if dry_run {
        for e in &entries {
            println!("{e}");
        }
        eprintln!(
            "lines={} width={}",
            niri_tasks::picker::clamp_lines(rows.len()),
            niri_tasks::picker::clamp_task_width(longest)
        );
        return Ok(());
    }

    let selected = Picker::new()
        .arg("--with-nth=2")
        .arg("--accept-nth=1")
        .lines(niri_tasks::picker::clamp_lines(rows.len()))
        .width(niri_tasks::picker::clamp_task_width(longest))
        .prompt(&format!("+{tag} "))
        .run(&entries)?;

    let Some(selected) = selected else { return Ok(()) };

    // Drop anything that is not one of our uuids — fuzzel echoes typed text
    // when it matches no entry, which here would mean acting on a uuid that
    // does not exist.
    if !rows.iter().any(|(u, _)| *u == selected) {
        return Ok(());
    }

    // Hand straight over to the add path rather than reimplementing it, so
    // there is one definition of what "adding a task" means.
    if selected == niri_tasks::rows::ADD_SENTINEL {
        return task_command(TaskCommand::Add { text: vec![], refine: false });
    }

    let picked = tasks.iter().find(|t| t.uuid == selected);
    let description = picked.map(|t| t.description.clone()).unwrap_or_default();
    let up_next = picked.is_some_and(|t| t.is_up_next());

    task_menu(&tag, selected, &description, up_next, niri_tasks::picker::clamp_task_width(longest))
}

/// The menu entry that goes back to the Claude working on a task.
const GO_TO_SESSION: &str = "Go to session";

/// The task menu's entries. Go to session leads, and only when a Claude is
/// working on the task — a task with none shows the menu as it always has.
/// The up next toggle reads as the step it takes: Not up next on a task
/// already up next.
fn menu_entries(has_session: bool, up_next: bool) -> Vec<String> {
    let mut entries = Vec::new();
    if has_session {
        entries.push(GO_TO_SESSION.to_string());
    }
    entries.extend(
        [
            "Edit",
            "Note",
            "Speak",
            task::up_next_label(up_next),
            "Refine",
            "Grill me",
            "Start working",
            "Update status",
            "Move to workspace",
        ]
        .map(String::from),
    );
    entries
}

/// The actions for one task, and doing the one picked. `width` is the delete
/// confirmation's, matched to the list it was reached from. `up_next` says
/// which way the up next entry reads.
fn task_menu(tag: &str, selected: String, description: &str, up_next: bool, width: usize) -> Result<()> {
    // Asked of herdr on every open; a session that is not running answers
    // at once, and no answer just means no Go to session.
    let workspace = niri::focused_workspace_name()?.unwrap_or_default();
    let entries = menu_entries(link::live_agent(&workspace, &selected).is_some(), up_next);
    let action = Picker::new()
        .lines(entries.len())
        .width(20)
        .prompt("")
        .run(&entries)?;

    match action.as_deref() {
        // Back to the Claude working on it — never starts one.
        Some(GO_TO_SESSION) => return task_command(TaskCommand::Session { uuid: selected }),
        // Edit and Note open the box, not a one-line picker: the descriptions
        // you reach for the edit box to fix are the long ones, and a note has
        // nowhere to show existing notes in a single row.
        Some("Edit") => return task_command(TaskCommand::Edit { uuid: selected, text: vec![] }),
        Some("Note") => return task_command(TaskCommand::Note { uuid: selected, text: vec![] }),
        // In the background: the menu closes at once, and picking Speak
        // again stops it.
        Some("Speak") => return task_command(TaskCommand::Speak { uuid: selected, here: false }),
        // Either word: the entry reads as the step it takes.
        Some(picked) if picked == task::up_next_label(up_next) => {
            return task_command(TaskCommand::UpNext { uuid: selected })
        }
        // Both open Claude in the workspace's herdr session; they differ only
        // in whether it interviews you before drafting.
        Some("Refine") => return task_command(TaskCommand::Refine { uuid: selected, grill: false }),
        Some("Grill me") => return task_command(TaskCommand::Refine { uuid: selected, grill: true }),
        // Its own worktree and a Claude to plan it; picked again, back to both.
        Some("Start working") => {
            return task_command(TaskCommand::Start { uuid: selected, here: false, workspace: None })
        }
        Some("Update status") => return task_status(&selected, description, width),
        Some("Move to workspace") => return task_move(tag, &selected, description),
        _ => {}
    }
    Ok(())
}

/// Move one task to a picked state.
///
/// Active and Stopped drive taskwarrior's start/stop flag; Waiting, Completed
/// and Deleted are its real statuses. One list rather than that distinction,
/// because from the menu they are all just "where is this task now". Deleted
/// is the one destructive pick, so it alone keeps a confirmation.
fn task_status(uuid: &str, description: &str, width: usize) -> Result<()> {
    let rows: Vec<String> = task::Status::ALL
        .iter()
        .map(|s| s.label().to_string())
        .collect();
    let picked = Picker::new()
        .arg("--no-sort")
        .lines(5)
        .width(20)
        .prompt("status ")
        .run(&rows)?;
    let Some(status) = picked.as_deref().and_then(task::Status::from_label) else {
        return Ok(());
    };

    if status == task::Status::Deleted {
        let confirm = Picker::new()
            .lines(2)
            .width(width)
            .prompt("delete? ")
            .run(&["No".into(), "Yes, delete".into()])?;
        if confirm.as_deref() != Some("Yes, delete") {
            return Ok(());
        }
    }
    apply_status(uuid, description, status)
}

/// Move a task and say so. The one place both the menu and `task status` do
/// it, so a task marked done from a script looks exactly like one marked done
/// from its card.
///
/// Marking a task active from inside a herdr pane also names that pane's
/// agent after it (see `link::link_current_pane`): that is how a Claude
/// started by hand becomes findable from the task's menu. Best effort — the
/// task is active either way, so a refused rename is only reported.
fn apply_status(uuid: &str, description: &str, status: task::Status) -> Result<()> {
    task::set_status(uuid, status)?;
    if status == task::Status::Active {
        if let Err(e) = link::link_current_pane(uuid) {
            eprintln!("{e:#}");
        }
    }
    notify::tasks(&format!("{}: {description}", status.label()));
    Ok(())
}

/// Move a task to another workspace by retagging it.
///
/// The destinations are the `~/Projects` folders, listed by folder name and
/// folded to a tag only once one is picked — so the list reads the way the
/// project picker does, and picking `niri-tasks` here puts the task on the same
/// `+niri_tasks` that opening that project would give it.
fn task_move(tag: &str, uuid: &str, description: &str) -> Result<()> {
    let (_, names) = projects()?;
    let destinations = project::move_destinations(&names, tag);
    if destinations.is_empty() {
        notify::tasks("No other project to move this to.");
        return Ok(());
    }

    let longest = destinations.iter().map(|n| n.chars().count()).max().unwrap_or(0);
    let selected = Picker::new()
        .arg("--no-sort")
        .lines(niri_tasks::picker::clamp_lines(destinations.len()))
        .width(niri_tasks::picker::clamp_project_width(longest))
        .prompt("move to ")
        .run(&destinations)?;

    let Some(selected) = selected else { return Ok(()) };

    // fuzzel echoes typed text verbatim when it matches no entry. For the
    // project picker that is a feature — it is how a folder gets created — but
    // here it would invent a tag for a project that does not exist, and a task
    // on a tag no workspace ever produces is invisible to every list. Only an
    // entry off the list counts.
    if !destinations.contains(&selected) {
        return Ok(());
    }

    let destination = niri_tasks::tag::workspace_tag(&selected);
    anyhow::ensure!(
        !destination.is_empty(),
        "'{selected}' has no usable tag characters."
    );

    task::move_to_tag(uuid, tag, &destination)?;
    notify::tasks(&format!("Moved to +{destination}: {description}"));
    Ok(())
}

/// `~/Projects` and the folders in it, sorted, dotfiles left out.
///
/// Shared by the project picker and the move-a-task picker so the two always
/// offer the same set — a project you can open is a project you can move a task
/// to.
fn projects() -> Result<(std::path::PathBuf, Vec<String>)> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let projects_dir = std::path::Path::new(&home).join("Projects");
    anyhow::ensure!(
        projects_dir.is_dir(),
        "No {} folder found.",
        projects_dir.display()
    );

    let mut names: Vec<String> = std::fs::read_dir(&projects_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();

    Ok((projects_dir, names))
}

fn workspace_command(cmd: WorkspaceCommand) -> Result<()> {
    match cmd {
        WorkspaceCommand::New => {
            let Some(name) = prompt_for_name("Name the new workspace", "")? else {
                return Ok(());
            };
            // focus-workspace-down only creates a workspace when you are
            // already on the last one; jump straight to the last workspace on
            // this output — niri always keeps an empty one there — then name it.
            let all = niri::workspaces()?;
            let focused = all.iter().find(|w| w.is_focused).context("no focused workspace")?;
            let output = focused.output.clone().unwrap_or_default();
            let last = niri::last_workspace_idx(&all, &output).context("no workspaces on output")?;

            niri::focus_workspace(WorkspaceReferenceArg::Index(last))?;
            niri::set_workspace_name(&name, None)?;
        }
        WorkspaceCommand::Rename => {
            let current = niri::focused_workspace_name()?.unwrap_or_default();
            let Some(name) = prompt_for_name("Rename this workspace", &current)? else {
                return Ok(());
            };
            niri::set_workspace_name(&name, None)?;
        }
        WorkspaceCommand::Default => niri_tasks::workspace_default()?,
    }
    Ok(())
}

/// Ask for a workspace name.
///
/// Stage 2 replaces this with the GTK box; until then it is fuzzel rather than
/// zenity, so there is one prompt style rather than two.
fn prompt_for_name(prompt: &str, prefill: &str) -> Result<Option<String>> {
    let mut p = Picker::new().lines(0).width(40).prompt(&format!("{prompt}: "));
    if !prefill.is_empty() {
        p = p.arg(format!("--search={prefill}"));
    }
    let typed = p.run(&[])?.unwrap_or_default();
    let name = text::collapse_whitespace(&typed);
    Ok((!name.is_empty()).then_some(name))
}

fn project_open() -> Result<()> {
    let (projects_dir, names) = projects()?;

    // Under the local folders, the account's GitHub repos that are not cloned
    // yet. The rows come from the cache; the refresh fired here feeds the
    // *next* open, so the popup never waits on the network.
    let remote = match github::cache_path() {
        Some(cache) => {
            github::spawn_refresh(&cache);
            github::remote_only(&github::read_cache(&cache), &names)
        }
        None => Vec::new(),
    };

    let mut entries = names.clone();
    entries.extend(remote.iter().map(|n| github::mark(n)));

    let longest = entries.iter().map(|n| n.chars().count()).max().unwrap_or(0);

    // An empty ~/Projects is not an error: fuzzel shows a bare input box and
    // whatever you type becomes the first project.
    let selected = Picker::new()
        .arg("--no-sort")
        .lines(niri_tasks::picker::clamp_lines(entries.len()))
        .width(niri_tasks::picker::clamp_project_width(longest))
        .run(&entries)?;

    let Some(selected) = selected else { return Ok(()) };

    let name = match github::choose(&selected, &names, &remote) {
        github::Choice::Nothing => return Ok(()),
        github::Choice::Rejected(msg) => anyhow::bail!(msg),
        github::Choice::Local(n) => n,
        github::Choice::Create(n) => {
            std::fs::create_dir(projects_dir.join(&n))
                .with_context(|| format!("Could not create {}/{n}", projects_dir.display()))?;
            notify::project(&format!("Created {}/{n}", projects_dir.display()));
            n
        }
        github::Choice::Clone(n) => {
            // The clone blocks this keybind, not the compositor; the
            // notifications are what says it started and finished.
            notify::project(&format!("Cloning {n}…"));
            github::clone(&n, &projects_dir.join(&n))?;
            notify::project(&format!("Cloned {}/{n}", projects_dir.display()));
            n
        }
    };

    let dir = projects_dir.join(&name);

    let all = niri::workspaces()?;
    if let Some(ws) = niri::find_workspace_by_name(&all, &name) {
        // Already opened this project once — go back to its workspace rather
        // than ending up with two workspaces sharing a name. Re-picking means
        // "take me back", not "give me another terminal", so only start things
        // up if the workspace is empty.
        let id = ws.id;
        niri::focus_workspace(WorkspaceReferenceArg::Name(name.clone()))?;
        if niri::window_count(id)? == 0 {
            spawn_startup(&dir, &name)?;
        }
    } else {
        let focused = all.iter().find(|w| w.is_focused).context("no focused workspace")?;
        let output = focused.output.clone().unwrap_or_default();
        let last = niri::last_workspace_idx(&all, &output).context("no workspaces on output")?;

        niri::focus_workspace(WorkspaceReferenceArg::Index(last))?;
        niri::set_workspace_name(&name, None)?;
        spawn_startup(&dir, &name)?;
    }
    Ok(())
}

/// Start a project workspace's programs — its herdr session's terminal, and
/// the editor if installed.
///
/// The name is set before this runs, which is what puts the windows on the
/// right workspace: niri spawns onto whatever is focused.
fn spawn_startup(dir: &std::path::Path, workspace: &str) -> Result<()> {
    for command in project::startup_commands(dir, workspace) {
        niri::spawn(command)?;
    }
    Ok(())
}

/// A terminal in the focused workspace's project folder.
///
/// Run directly rather than through niri's spawn, so a failure — no ghostty,
/// D-Bus refusing the window — comes back here and is reported, instead of
/// the key doing nothing. The window still lands on the focused workspace:
/// the running ghostty makes it, and niri places new windows by focus.
fn terminal() -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let workspace = niri::focused_workspace_name()?.unwrap_or_default();
    let dir = session::start_dir(std::path::Path::new(&home), &workspace);

    let cmd = project::terminal_command(&dir);
    let status = std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .with_context(|| format!("could not run {}", cmd[0]))?;
    anyhow::ensure!(status.success(), "{} failed to open a window ({status})", cmd.join(" "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use niri_tasks::panel::actions::Action;

    /// Every button on a task card spawns `niritasks` with these arguments, so
    /// each has to be a command the real CLI accepts. Otherwise a typo shows
    /// up as a click that does nothing.
    #[test]
    fn every_card_button_is_a_command_the_cli_accepts() {
        for action in Action::ALL {
            let mut argv = vec!["niritasks".to_string()];
            argv.extend(action.args("c53b6e3d"));
            if let Err(e) = Cli::try_parse_from(&argv) {
                panic!("{} runs {argv:?}, which the CLI rejects: {e}", action.label());
            }
        }
    }

    /// The menu's Go to session runs this; a script can too.
    #[test]
    fn going_to_a_tasks_session_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "session", "c53b6e3d"]).is_ok());
    }

    /// The menu's Speak runs this, and so does the panel's button.
    #[test]
    fn speaking_a_task_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "speak", "c53b6e3d"]).is_ok());
    }

    /// `task speak` starts the speech in the background with these
    /// arguments, so they have to be ones the CLI accepts, or a press would
    /// start a process that only prints usage.
    #[test]
    fn the_background_speech_is_a_command_the_cli_accepts() {
        let mut argv = vec!["niritasks".to_string()];
        argv.extend(niri_tasks::speak::worker_args("c53b6e3d"));
        if let Err(e) = Cli::try_parse_from(&argv) {
            panic!("speak starts {argv:?}, which the CLI rejects: {e}");
        }
    }

    /// Go to session leads the menu when there is a session to go to, and a
    /// task with none gets exactly the menu it always had.
    #[test]
    fn go_to_session_leads_the_menu_only_when_there_is_one() {
        let without = menu_entries(false, false);
        assert_eq!(
            without,
            vec!["Edit", "Note", "Speak", "Up next", "Refine", "Grill me", "Start working", "Update status", "Move to workspace"]
        );
        let with = menu_entries(true, false);
        assert_eq!(with[0], GO_TO_SESSION);
        assert_eq!(with[1..], without[..]);
    }

    /// The menu offers the step up next would take: Not up next, in the same
    /// place, on a task already up next.
    #[test]
    fn the_menu_offers_to_clear_up_next_on_a_task_up_next() {
        let marked = menu_entries(false, true);
        assert_eq!(marked[3], "Not up next");
        assert!(!marked.contains(&"Up next".to_string()));
    }

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
                .is_none_or(|c| !(c.is_alphanumeric() || c == '-'))
        })
    }

    /// Every subcommand, and every flag, that no line in `lines` shows being
    /// run. A subcommand counts once some line runs it. A flag counts once
    /// some line runs its subcommand with that flag, so it is documented in
    /// use and not only named in passing.
    /// Long flags only: a short-only flag or a positional is not checked
    /// here (positionals are covered by the `--help` test).
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
}
