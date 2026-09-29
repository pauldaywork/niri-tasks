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
    github, ipc, niri, notify, picker::Picker, project, refine, require_workspace_tag, session,
    task, taskbox, text, work,
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
    /// Print the workspace's task tag, or exit 1 if it has none
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

    /// Run the task panels and the task-box server (long-running; started by a systemd user unit)
    Daemon,
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Print the active task's description, or nothing
    Active,
    /// Pick a task from this workspace and act on it
    List {
        /// Print the rows that would be shown, instead of opening the picker.
        /// Exists so the list can be diffed against the shell original without
        /// a GUI in the way.
        #[arg(long)]
        dry_run: bool,
    },
    /// Slide out the task panel and pick a task with the keyboard (Mod+Alt+Ctrl+T)
    Panel,
    /// Open the action menu for one task (clicking its task card)
    Menu { uuid: String },
    /// Add a task
    Add { text: Vec<String> },
    /// Print a task's description by uuid
    GetText { uuid: String },
    /// Replace a task's description
    Edit { uuid: String, text: Vec<String> },
    /// Print a task's notes, one per line
    GetNotes { uuid: String },
    /// Attach a note to a task
    Note { uuid: String, text: Vec<String> },
    /// Work a task up into a plan with Claude, in a new tab of the workspace's herdr session
    Refine {
        uuid: String,
        /// Interview first, via the `grilling` skill, rather than drafting straight away
        #[arg(long)]
        grill: bool,
    },
    /// Start working on a task in its own git worktree, with Claude planning it
    Start {
        uuid: String,
        /// The setup step, run inside the tab `task start` opens: make the
        /// worktree, open it, start Claude, close the tab
        #[arg(long, requires = "workspace")]
        here: bool,
        /// The workspace the task belongs to (only with --here, which runs
        /// inside herdr where niri's focus says nothing about it)
        #[arg(long)]
        workspace: Option<String>,
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
    if let Some(s) = taskbox::show(taskbox::BoxConfig::for_task(mode, t)) {
        task::replace_text(uuid, &s.description, &s.notes)?;
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
            if let Some(active) = task::active_for_tag(&t)? {
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
            let width = niri_tasks::picker::clamp_task_width(t.description.chars().count());
            return task_menu(&tag, uuid, &t.description, width);
        }

        // With no text, open the box. With text, add straight away — which is
        // what makes `niritasks task add ship it due:friday` work from a shell.
        TaskCommand::Add { text: words } => {
            let tag = require_workspace_tag()?;
            // The box can return notes with the description; the shell form has
            // nowhere to type them, so it never does.
            let (description, notes) = if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Add) {
                    return Ok(());
                }
                match taskbox::show(taskbox::BoxConfig::add(&tag)) {
                    Some(s) => {
                        let notes = s.note_texts();
                        (s.description, notes)
                    }
                    None => return Ok(()),
                }
            } else {
                (text::collapse_whitespace(&words.join(" ")), Vec::new())
            };

            if description.is_empty() {
                return Ok(());
            }
            task::add_with_notes(&tag, &text::add_args(&description), &notes)?;
            notify::tasks(&format!("Added to +{tag}: {description}"));
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
            work::launch(&workspace, &uuid, &t.description)?;
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
        return task_command(TaskCommand::Add { text: vec![] });
    }

    let description = tasks
        .iter()
        .find(|t| t.uuid == selected)
        .map(|t| t.description.clone())
        .unwrap_or_default();

    task_menu(&tag, selected, &description, niri_tasks::picker::clamp_task_width(longest))
}

/// The actions for one task, and doing the one picked. `width` is the delete
/// confirmation's, matched to the list it was reached from.
fn task_menu(tag: &str, selected: String, description: &str, width: usize) -> Result<()> {
    let action = Picker::new()
        .lines(7)
        .width(20)
        .prompt("")
        .run(&[
            "Edit".into(),
            "Note".into(),
            "Refine".into(),
            "Grill me".into(),
            "Start working".into(),
            "Update status".into(),
            "Move to workspace".into(),
        ])?;

    match action.as_deref() {
        // Edit and Note open the box, not a one-line picker: the descriptions
        // you reach for the edit box to fix are the long ones, and a note has
        // nowhere to show existing notes in a single row.
        Some("Edit") => return task_command(TaskCommand::Edit { uuid: selected, text: vec![] }),
        Some("Note") => return task_command(TaskCommand::Note { uuid: selected, text: vec![] }),
        // Both open Claude in the workspace's herdr session; they differ only
        // in whether it interviews you before drafting.
        Some("Refine") => return task_command(TaskCommand::Refine { uuid: selected, grill: false }),
        Some("Grill me") => return task_command(TaskCommand::Refine { uuid: selected, grill: true }),
        // Its own worktree and a Claude to plan it; picked again, back to both.
        Some("Start working") => {
            return task_command(TaskCommand::Start { uuid: selected, here: false, workspace: None })
        }
        Some("Update status") => return task_status(tag, &selected, description, width),
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
fn task_status(tag: &str, uuid: &str, description: &str, width: usize) -> Result<()> {
    let state = Picker::new()
        .arg("--no-sort")
        .lines(5)
        .width(20)
        .prompt("status ")
        .run(&[
            "Active".into(),
            "Stopped".into(),
            "Waiting".into(),
            "Completed".into(),
            "Deleted".into(),
        ])?;

    match state.as_deref() {
        Some("Active") => {
            task::set_active(tag, uuid)?;
            notify::tasks(&format!("Active: {description}"));
        }
        Some("Stopped") => {
            task::stop(uuid)?;
            notify::tasks(&format!("Stopped: {description}"));
        }
        Some("Waiting") => {
            task::wait(uuid)?;
            notify::tasks(&format!("Waiting: {description}"));
        }
        Some("Completed") => {
            task::complete(uuid)?;
            notify::tasks(&format!("Completed: {description}"));
        }
        Some("Deleted") => {
            let confirm = Picker::new()
                .lines(2)
                .width(width)
                .prompt("delete? ")
                .run(&["No".into(), "Yes, delete".into()])?;
            if confirm.as_deref() == Some("Yes, delete") {
                task::delete(uuid)?;
                notify::tasks(&format!("Deleted: {description}"));
            }
        }
        _ => {}
    }
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
