//! wt — workspace-scoped Taskwarrior for niri.
//!
//! Every task is tagged with the name of the workspace it was created on, so
//! "my tasks" always means "the tasks for the project I'm looking at". Nothing
//! here knows about ~/Projects — it goes purely off the niri workspace name,
//! which `wt project open` sets to the folder name.

use clap::{Parser, Subcommand};

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
    List,
    /// Add a task; opens the box when no text is given
    Add { text: Option<String> },
    /// Edit a task's description; opens the box when no text is given
    Edit { uuid: String, text: Option<String> },
    /// Attach a note to a task; opens the box when no text is given
    Note { uuid: String, text: Option<String> },
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Tag => todo!("wired up in stage 1"),
        Command::Task(_) => todo!("wired up in stage 1"),
        Command::Workspace(_) => todo!("wired up in stage 1"),
        Command::Project(_) => todo!("wired up in stage 1"),
        Command::TmuxSession => todo!("wired up in stage 1"),
    }
}
