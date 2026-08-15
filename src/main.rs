//! wt — workspace-scoped Taskwarrior for niri.
//!
//! Every task is tagged with the name of the workspace it was created on, so
//! "my tasks" always means "the tasks for the project I'm looking at". Nothing
//! here knows about ~/Projects — it goes purely off the niri workspace name,
//! which `wt project open` sets to the folder name.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use niri_ipc::WorkspaceReferenceArg;
use niri_tasks::{
    niri, notify, picker::Picker, project, require_workspace_tag, session, task, taskbox, text,
};

#[derive(Parser)]
#[command(name = "wt", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the focused workspace's task tag, or exit 1 if it has none
    Tag,

    /// Taskwarrior operations scoped to the focused workspace
    #[command(subcommand)]
    Task(TaskCommand),

    /// Workspace naming
    #[command(subcommand)]
    Workspace(WorkspaceCommand),

    /// Project folder operations
    #[command(subcommand)]
    Project(ProjectCommand),

    /// Open a tmux session named after the focused workspace (ghostty's `command =`)
    TmuxSession,
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
        // Errors here are things the user needs to act on ("name this
        // workspace first"), so they go to the desktop, not just to a stderr
        // nobody is watching — these run from keybinds with no terminal.
        notify::tasks(&e.to_string());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Tag => {
            print!("{}", require_workspace_tag()?);
        }
        Command::Task(c) => return task_command(c),
        Command::Workspace(c) => return workspace_command(c),
        Command::Project(ProjectCommand::Open) => return project_open(),
        Command::TmuxSession => return tmux_session(),
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

        // With no text, open the box. With text, add straight away — which is
        // what makes `wt task add ship it due:friday` work from a shell.
        TaskCommand::Add { text: words } => {
            let tag = require_workspace_tag()?;
            let description = if words.is_empty() {
                match taskbox::show(taskbox::BoxConfig {
                    mode: taskbox::Mode::Add,
                    subtitle: format!("+{tag}"),
                    initial: String::new(),
                    notes: String::new(),
                }) {
                    Some(t) => t,
                    None => return Ok(()),
                }
            } else {
                text::collapse_whitespace(&words.join(" "))
            };

            if description.is_empty() {
                return Ok(());
            }
            task::add(&tag, &text::add_args(&description))?;
            notify::tasks(&format!("Added to +{tag}: {description}"));
        }

        TaskCommand::GetText { uuid } => {
            let t = task::get(&uuid)?.context("task not found")?;
            println!("{}", t.description);
        }

        TaskCommand::Edit { uuid, text: words } => {
            let description = if words.is_empty() {
                // The box fetches the description itself rather than taking it
                // as an argument — the descriptions you reach for the edit box
                // to fix are the long ones, and those are exactly the ones a
                // single picker row showed you a fraction of.
                let current = task::get(&uuid)?.context("task not found")?.description;
                match taskbox::show(taskbox::BoxConfig {
                    mode: taskbox::Mode::Edit,
                    subtitle: String::new(),
                    initial: current,
                    notes: String::new(),
                }) {
                    Some(t) => t,
                    None => return Ok(()),
                }
            } else {
                text::collapse_whitespace(&words.join(" "))
            };

            if description.is_empty() {
                return Ok(());
            }
            task::modify_description(&uuid, &description)?;
        }

        TaskCommand::GetNotes { uuid } => {
            let t = task::get(&uuid)?.context("task not found")?;
            for a in &t.annotations {
                // Date first, then the text, whitespace collapsed — the shape
                // the box lists them in.
                let date = a.entry.get(..8).unwrap_or(&a.entry);
                println!("{}  {}", date, text::collapse_whitespace(&a.description));
            }
        }

        TaskCommand::Note { uuid, text: words } => {
            let note = if words.is_empty() {
                // Existing notes are listed above the input: they are otherwise
                // invisible from the picker, which shows a description, and an
                // annotation is not one.
                let t = task::get(&uuid)?.context("task not found")?;
                let notes = t
                    .annotations
                    .iter()
                    .map(|a| {
                        let date = a.entry.get(..8).unwrap_or(&a.entry);
                        format!("{date}  {}", text::collapse_whitespace(&a.description))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                match taskbox::show(taskbox::BoxConfig {
                    mode: taskbox::Mode::Annotate,
                    subtitle: t.description,
                    initial: String::new(),
                    notes,
                }) {
                    Some(t) => t,
                    None => return Ok(()),
                }
            } else {
                text::collapse_whitespace(&words.join(" "))
            };

            if note.is_empty() {
                return Ok(());
            }
            task::annotate(&uuid, &note)?;
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

    let width = niri_tasks::picker::clamp_task_width(longest);
    let action = Picker::new()
        .lines(5)
        .width(20)
        .prompt("")
        .run(&[
            "Edit".into(),
            "Note".into(),
            "Delete".into(),
            "Complete".into(),
            "Set active".into(),
        ])?;

    match action.as_deref() {
        // Edit and Note open the box, not a one-line picker: the descriptions
        // you reach for the edit box to fix are the long ones, and a note has
        // nowhere to show existing notes in a single row.
        Some("Edit") => return task_command(TaskCommand::Edit { uuid: selected, text: vec![] }),
        Some("Note") => return task_command(TaskCommand::Note { uuid: selected, text: vec![] }),
        Some("Delete") => {
            let confirm = Picker::new()
                .lines(2)
                .width(width)
                .prompt("delete? ")
                .run(&["No".into(), "Yes, delete".into()])?;
            if confirm.as_deref() == Some("Yes, delete") {
                task::delete(&selected)?;
                notify::tasks(&format!("Deleted: {description}"));
            }
        }
        Some("Complete") => {
            task::complete(&selected)?;
            notify::tasks(&format!("Completed: {description}"));
        }
        Some("Set active") => {
            task::set_active(&tag, &selected)?;
            notify::tasks(&format!("Active: {description}"));
        }
        _ => {}
    }
    Ok(())
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
        WorkspaceCommand::Default => {
            // Startup only: give workspace 1 a name so it has a tag from the
            // first moment, rather than silently dropping tasks until named.
            let all = niri::workspaces()?;
            if let Some(ws) = all.iter().find(|w| w.idx == 1) {
                if ws.name.is_none() {
                    niri::set_workspace_name("general", Some(WorkspaceReferenceArg::Index(1)))?;
                }
            }
        }
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

    let longest = names.iter().map(|n| n.chars().count()).max().unwrap_or(0);

    // An empty ~/Projects is not an error: fuzzel shows a bare input box and
    // whatever you type becomes the first project.
    let selected = Picker::new()
        .arg("--no-sort")
        .lines(niri_tasks::picker::clamp_lines(names.len()))
        .width(niri_tasks::picker::clamp_project_width(longest))
        .run(&names)?;

    let Some(selected) = selected else { return Ok(()) };

    let name = match project::resolve(&selected, &names) {
        project::Resolved::Nothing => return Ok(()),
        project::Resolved::Rejected(msg) => anyhow::bail!(msg),
        project::Resolved::Existing(n) => n,
        project::Resolved::Create(n) => {
            std::fs::create_dir(projects_dir.join(&n))
                .with_context(|| format!("Could not create {}/{n}", projects_dir.display()))?;
            notify::project(&format!("Created {}/{n}", projects_dir.display()));
            n
        }
    };

    let all = niri::workspaces()?;
    if let Some(ws) = niri::find_workspace_by_name(&all, &name) {
        // Already opened this project once — go back to its workspace rather
        // than ending up with two workspaces sharing a name. Re-picking means
        // "take me back", not "give me another terminal", so only spawn one if
        // the workspace is empty.
        let id = ws.id;
        niri::focus_workspace(WorkspaceReferenceArg::Name(name.clone()))?;
        if niri::window_count(id)? == 0 {
            niri::spawn(vec!["ghostty".into()])?;
        }
    } else {
        let focused = all.iter().find(|w| w.is_focused).context("no focused workspace")?;
        let output = focused.output.clone().unwrap_or_default();
        let last = niri::last_workspace_idx(&all, &output).context("no workspaces on output")?;

        niri::focus_workspace(WorkspaceReferenceArg::Index(last))?;
        niri::set_workspace_name(&name, None)?;
        niri::spawn(vec!["ghostty".into()])?;
    }
    Ok(())
}

fn tmux_session() -> Result<()> {
    use std::os::unix::process::CommandExt;

    let home = std::env::var("HOME").context("HOME is unset")?;

    // Fall back to the workspace index when unnamed, then to "unknown".
    let raw = match niri::focused_workspace()? {
        Some(ws) => ws.name.unwrap_or_else(|| ws.idx.to_string()),
        None => "unknown".to_string(),
    };

    let base = session::sanitize_session_name(&raw);
    let name = session::next_session_name(&base, |candidate| {
        std::process::Command::new("tmux")
            .args(["has-session", "-t", candidate])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    let dir = session::start_dir(std::path::Path::new(&home), &raw);

    // exec, so ghostty's child is tmux itself rather than a shell wrapping it.
    Err(std::process::Command::new("tmux")
        .args(["new-session", "-A", "-s", &name, "-c"])
        .arg(&dir)
        .exec())
    .context("could not exec tmux")
}
