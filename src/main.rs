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
    actions::Action,
    caller_workspace, caller_workspace_tag, github, ipc, link, niri, notify, project, refine,
    require_workspace_tag, session,
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
    /// Slide out the task panel and work its tasks with the keyboard (Mod+Alt+Ctrl+T)
    Panel,
    /// Move a task to a state, as a card's Stop, Waiting, Complete and Remove do
    ///
    /// The same states, code and notification as the card's buttons, for
    /// scripts and for finishing a task's worktree. Use it rather than `task
    /// <uuid> done`, `start` or `stop`: `active` also links the herdr pane it
    /// runs in to the task, and every change sends the buttons' notification.
    Status {
        /// The task's uuid, or its first 8 characters, as in a
        /// `task/<slug>-<uuid8>` branch
        uuid: String,
        /// Where to move it; `stopped` also brings back a waiting task
        state: task::Status,
        /// Confirm `deleted`, which a card's Remove asks about and a script cannot be asked
        #[arg(long)]
        yes: bool,
    },
    /// Move a task to another project's workspace, as its card's Move to workspace does
    ///
    /// The folder is one of ~/Projects' folders, other than this workspace's;
    /// the task trades this workspace's tag for the folder's. In a pane of a
    /// named herdr session this workspace is the session's, else the focused
    /// one.
    Move {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// The ~/Projects folder to move it to, as named there
        folder: String,
    },
    /// Add a task to this workspace, or open the task box with no text
    ///
    /// In a pane of a named herdr session the task goes to the session's
    /// workspace, so a task an agent files stays put when you switch
    /// workspace; anywhere else, as from a keybind, to the focused one.
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
    /// lifts it in `task next`. Nothing else about the task changes.
    UpNext {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },

    /// Work a task up into a plan with Claude, in a new tab of the workspace's herdr session
    ///
    /// The workspace is this herdr session's in a herdr pane, else the
    /// focused one, as for task add, task start and task session.
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
    /// task, it goes back to both. The workspace is this herdr session's in a
    /// herdr pane, else the focused one.
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
    ///
    /// The workspace is this herdr session's in a herdr pane, else the
    /// focused one.
    Session {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Show the task panel's project list (Mod+Alt+W); with a name, put that project on its own named workspace
    Open {
        /// A ~/Projects folder, a GitHub repo as the list shows it, or a new name to make a folder of
        name: Option<String>,
    },
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
    dispatch(Cli::parse())
}

/// Run one parsed command line.
fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Tag { session } => {
            let tag = if session {
                niri_tasks::session_workspace_tag()?
            } else {
                require_workspace_tag()?
            };
            print!("{tag}");
        }
        Command::Task(c) => return task_command(c),
        Command::Project(ProjectCommand::Open { name }) => return project_open(name),
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
        // The daemon draws the panel. Without one there is no panel to
        // hand the keyboard to, and saying so beats doing nothing.
        TaskCommand::Panel => {
            anyhow::ensure!(
                delegate_to_daemon(ipc::Request::Panel),
                "The niri-tasks daemon is not running, so there is no task panel. Start it with `systemctl --user start niri-tasks`."
            );
        }

        TaskCommand::Status { uuid, state, yes } => {
            anyhow::ensure!(
                yes || state != task::Status::Deleted,
                "Deleting a task needs --yes, Remove's confirmation."
            );
            let t = task::get(&uuid)?.context("task not found")?;
            apply_status(&t.uuid, &t.description, state)?;
        }

        TaskCommand::Move { uuid, folder } => {
            // From a herdr pane, the session's workspace, as for task add;
            // the panel's spawn has focused its own monitor first.
            let tag = caller_workspace_tag()?;
            let t = task::get(&uuid)?.context("task not found")?;
            let (_, names) = project::list()?;
            let to = project::move_task(&t, &tag, &folder, &names)?;
            notify::tasks(&format!("Moved to +{to}: {}", t.description));
        }

        // With no text, open the box. With text, add straight away — which is
        // what makes `niritasks task add ship it due:friday` work from a shell.
        // The binding is `and_refine` because `refine` is the module, imported
        // at the top of this file.
        TaskCommand::Add { text: words, refine: and_refine } => {
            // From a herdr pane, the session's workspace: an agent filing a
            // task must not follow the user's focus to another workspace.
            let tag = caller_workspace_tag()?;
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
            notify::tasks(&format!("{}: {}", Action::UpNext.label(t.is_up_next()), t.description));
        }

        TaskCommand::Start { uuid, here, workspace } => {
            if here {
                // clap's `requires` guarantees it; the context is for the type.
                let workspace = workspace.context("--here needs --workspace")?;
                return work::set_up_here(&workspace, &uuid);
            }
            // From a herdr pane, the session's workspace, as for task add: an
            // agent's Start must not open in whatever workspace has the focus.
            let workspace = caller_workspace()?;
            let t = task::get(&uuid)?.context("task not found")?;
            anyhow::ensure!(t.status == "pending", "Only a pending task can be started.");
            work::launch(&workspace, &t)?;
        }

        TaskCommand::Refine { uuid, grill } => {
            // The same refusal every entry point makes: a task refined on an
            // unnamed workspace would have no session to open in.
            let workspace = caller_workspace()?;
            let t = task::get(&uuid)?.context("task not found")?;
            // task::get is unfiltered by status; a completed or deleted task
            // has nothing left to work up into a plan.
            anyhow::ensure!(t.status == "pending", "Only a pending task can be refined.");
            let mode = if grill { refine::Mode::Grill } else { refine::Mode::Quick };
            refine::launch(&workspace, &t, mode)?;
        }

        TaskCommand::Session { uuid } => {
            let workspace = caller_workspace()?;
            // The uuid as found, not as typed: the agent's name is made from
            // its first eight characters, and a typed task number has none of
            // them.
            let t = task::get(&uuid)?.context("task not found")?;
            link::go_to(&workspace, &t.uuid)?;
        }
    }
    Ok(())
}

/// Move a task and say so. The one place it is done, so a task completed from
/// a script looks exactly like one completed from its card.
///
/// Marking a task active from inside a herdr pane also names that pane's
/// agent after it (see `link::link_current_pane`): that is how a Claude
/// started by hand becomes findable from the task's card. Best effort — the
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

/// `project open`: with no name, the daemon's project list, which runs this
/// again with the row picked; with one, open that project.
///
/// The GitHub rows come from a cache, refreshed here for the *next* open so
/// the list never waits on the network. Here and not in the daemon: the
/// refresh is a child never waited on, which init reaps once this process
/// exits, and which the daemon would keep as a zombie.
fn project_open(name: Option<String>) -> Result<()> {
    let Some(name) = name else {
        if let Some(cache) = github::cache_path() {
            github::spawn_refresh(&cache);
        }
        anyhow::ensure!(
            delegate_to_daemon(ipc::Request::Projects),
            "The niri-tasks daemon is not running, so there is no project list. Start it with `systemctl --user start niri-tasks`."
        );
        return Ok(());
    };
    open_project(&name)
}

/// Open `selected`, a row of the project list or a name typed there: a
/// folder as it is, a GitHub row cloned first, a new name made a folder
/// first. Then back to the project's workspace if it has one, else onto the
/// last workspace on this output, named for it, with its programs started.
fn open_project(selected: &str) -> Result<()> {
    let (projects_dir, names) = project::list()?;
    let remote = github::cache_path()
        .map(|cache| github::remote_only(&github::read_cache(&cache), &names))
        .unwrap_or_default();

    let name = match github::choose(selected, &names, &remote) {
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

    /// The old picker and menu are gone: the task panel does what they did.
    #[test]
    fn the_picker_and_the_menu_are_no_commands() {
        assert!(Cli::try_parse_from(["niritasks", "task", "list"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "list", "--dry-run"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "menu", "c53b6e3d"]).is_err());
    }

    /// A card's Move to workspace runs this, with the folder picked on the
    /// project list; a folder is required.
    #[test]
    fn moving_a_task_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "move", "c53b6e3d", "alpha"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "move", "c53b6e3d"]).is_err());
    }

    /// Mod+Alt+W runs `project open` bare, for the panel's project list,
    /// and picking a row runs it again with the row, GitHub marker and all.
    #[test]
    fn opening_a_project_takes_an_optional_name() {
        assert!(Cli::try_parse_from(["niritasks", "project", "open"]).is_ok());
        let mut argv = vec!["niritasks".to_string()];
        argv.extend(project::open_args(&github::mark("convo")));
        assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
    }

    /// The panel offers to make `~/Projects/-foo`, so a name that starts
    /// with a dash has to reach `project open` as a name, not as a flag.
    #[test]
    fn opening_a_dashed_name_is_not_a_flag() {
        let mut argv = vec!["niritasks".to_string()];
        argv.extend(project::open_args("-foo"));
        assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
    }

    /// The action row runs a task action as its own `niritasks` words, so each
    /// has to be a command the real CLI accepts. Otherwise a typo shows up as
    /// a click that does nothing.
    #[test]
    fn every_task_action_is_a_command_the_cli_accepts() {
        for action in Action::ALL {
            let mut argv = vec!["niritasks".to_string()];
            argv.extend(action.args("c53b6e3d"));
            // Move's words stop short of the folder the project list picks.
            if action == Action::Move {
                argv.push("alpha".to_string());
            }
            if let Err(e) = Cli::try_parse_from(&argv) {
                panic!("{} runs {argv:?}, which the CLI rejects: {e}", action.label(false));
            }
        }
    }

    /// A card's Go to session runs this; a script can too.
    #[test]
    fn going_to_a_tasks_session_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "session", "c53b6e3d"]).is_ok());
    }

    /// A card's Speak runs this.
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
