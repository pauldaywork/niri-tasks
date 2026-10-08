//! The long-running daemon: task panels on every monitor, and a warm GTK
//! process for task boxes.
//!
//! Started by the `niri-tasks` systemd user unit as `niritasks daemon`.
//!
//! ## Refresh strategy
//!
//! Rather than subscribe to niri's event stream and watch the task database
//! with a file-watcher — two background threads and their reconnect logic — the
//! tick does two cheap checks and only does real work when one of them changes:
//!
//!   1. which workspace each monitor is showing, over the niri socket (no
//!      subprocess), and
//!   2. the mtimes of taskwarrior's `pending.data` and `completed.data`, in
//!      the directory taskwarrior itself names (two stats).
//!
//! `task export` is a subprocess and the expensive part, so it runs only when
//! one of those actually moved, and once per workspace tag rather than once
//! per monitor. Idle cost is a socket round-trip and two stats.

use crate::panel::{model, style, Panel};
use crate::{niri, tag, task};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Application, CssProvider};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

pub const APP_ID: &str = "dev.niri-tasks.daemon";

const TICK: Duration = Duration::from_millis(700);

type Panels = Rc<RefCell<HashMap<String, Rc<Panel>>>>;

thread_local! {
    /// The panels, where an IPC request can reach them. Only ever touched on
    /// the GTK main thread, which is the one thread they may be.
    static PANELS: Panels = Panels::default();
}

/// What the last tick saw, so a tick that changes nothing does nothing.
#[derive(Default)]
struct State {
    /// Connector → the name of the workspace it shows. `None` until the first
    /// tick, and again after a hotplug, to force a redraw.
    outputs: Option<BTreeMap<String, Option<String>>>,
    /// [`db_mtimes`] at the last tick.
    task_mtimes: [Option<SystemTime>; 2],
    /// Where taskwarrior keeps its data, asked of taskwarrior itself once,
    /// so a `.taskrc` that moves it is honoured. None when `task` could not
    /// say, in which case nothing is watched: the cards then follow only
    /// workspace changes, and `task export` is most likely failing too.
    data_dir: Option<std::path::PathBuf>,
}

/// The mtimes of `pending.data` and `completed.data` in `dir`. Both: editing,
/// noting or removing a finished task writes `completed.data` alone, and its
/// card on the Finished tab has to follow.
fn db_mtimes(dir: &std::path::Path) -> [Option<SystemTime>; 2] {
    ["pending.data", "completed.data"]
        .map(|file| std::fs::metadata(dir.join(file)).ok()?.modified().ok())
}

/// Every task card for a workspace, or none when it is unnamed or empty. The
/// panel does the capping.
fn cards_for_workspace(name: Option<&str>) -> Vec<model::Card> {
    let t = tag::workspace_tag(name.unwrap_or_default());
    if t.is_empty() {
        return Vec::new();
    }
    let Ok(mut tasks) = task::pending_for_tag(&t) else {
        return Vec::new();
    };
    // Waiting ones too, for the keyboard's Waiting tab; the panel keeps them
    // off the hover and the peek.
    tasks.extend(task::waiting_for_tag(&t).unwrap_or_default());
    // The last few finished, for the Finished tab; the panel keeps them off
    // every other tab, the hover and the peek, as it does waiting ones.
    tasks.extend(task::completed_for_tag(&t).unwrap_or_default());
    if tasks.is_empty() {
        return Vec::new();
    }
    let blocked = task::blocked_uuids_for_tag(&t).unwrap_or_default();
    model::cards(&tasks, &blocked)
}

pub fn run() -> anyhow::Result<()> {
    // Startup housekeeping that used to be its own spawn-at-startup script:
    // give workspace 1 a name so it has a tag from the first moment. Failure is
    // not fatal — the panel is still worth running.
    if let Err(e) = crate::workspace_default() {
        eprintln!("could not set default workspace name: {e}");
    }

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(build);
    app.run_with_args::<&str>(&[]);
    Ok(())
}

fn build(app: &Application) {
    let Some(display) = gdk::Display::default() else {
        eprintln!("no display");
        return;
    };

    let provider = CssProvider::new();
    provider.load_from_data(&style::css());
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // One panel per monitor, keyed by connector name — which is also niri's
    // output name, so it is what ties a panel to the workspace it shows.
    let panels: Panels = PANELS.with(Rc::clone);
    let data_dir = task::data_location()
        .map_err(|e| eprintln!("could not find the task database: {e}"))
        .ok();
    let state = Rc::new(RefCell::new(State {
        data_dir,
        ..State::default()
    }));

    sync_monitors(app, &display, &panels, &state);

    // Hotplug: plugging in the external monitor, or Super+Alt+Comma blanking
    // the built-in one, changes this list. A new panel starts empty;
    // sync_monitors forgets what was drawn so the next tick fills it.
    {
        let app = app.clone();
        let display2 = display.clone();
        let panels = panels.clone();
        let state = state.clone();
        display.monitors().connect_items_changed(move |_, _, _, _| {
            sync_monitors(&app, &display2, &panels, &state);
        });
    }

    // Hold the application open: layer-shell surfaces are not "windows" in the
    // sense GTK counts for lifetime, and an empty panel has none mapped at
    // all, so without this the app would quit as soon as the cards went away.
    let _hold = app.hold();

    let panels_tick = panels.clone();
    let state_tick = state.clone();
    gtk4::glib::timeout_add_local(TICK, move || {
        tick(&panels_tick, &state_tick);
        gtk4::glib::ControlFlow::Continue
    });

    // Paint once immediately rather than waiting out the first tick.
    tick(&panels, &state);

    // Serve the CLI's requests: the box, the panel's keyboard, the project
    // list. Failing to listen is not fatal for the panels, but every GUI
    // command will report no daemon until it is fixed.
    if let Err(e) = crate::ipc::listen(move |req| {
        // Back onto the main thread: GTK may only be touched from there, and
        // the listener runs on its own. The Application cannot be captured
        // across that boundary — it is not Send — so look it up once we are
        // already on the main thread, where holding it is legal.
        gtk4::glib::idle_add_once(move || {
            let Some(app) = gtk4::gio::Application::default() else {
                return;
            };
            if let Ok(app) = app.downcast::<Application>() {
                serve_box_request(&app, req);
            }
        });
    }) {
        eprintln!(
            "IPC unavailable ({e}); task add, edit, note, panel and project open will report no daemon"
        );
    }
}

/// `NIRITASKS_DEBUG=1` traces each tick to stderr — which, under the systemd unit,
/// means `journalctl --user -u niri-tasks`.
fn debug(msg: &str) {
    if std::env::var_os("NIRITASKS_DEBUG").is_some() {
        eprintln!("[daemon] {msg}");
    }
}

/// What Mod+Alt+Ctrl+T says on a workspace with no task to show.
pub const NO_TASKS: &str = "No tasks here — Mod+Alt+T adds one.";

/// What Mod+Alt+W says when the focused monitor has no panel to show the
/// project list on: a monitor GTK has not named yet (see `sync_monitors`).
pub const NO_PANEL: &str = "No task panel on this monitor yet to show the project list on.";

/// What asking for a task box says while a different one is open. That box is
/// brought forward instead and the request dropped, so this says why the box
/// asked for did not appear.
pub const BOX_ALREADY_OPEN: &str = "A task box is already open — save or discard it first.";

/// The notice for what `taskbox::open_in` did, if it needs one.
fn already_open_notice(opened: crate::taskbox::Opened) -> Option<&'static str> {
    (opened == crate::taskbox::Opened::Other).then_some(BOX_ALREADY_OPEN)
}

/// What Mod+Alt+Ctrl+T says when there is no card to hand the keyboard to:
/// the tag's own refusal on an unnamed workspace, which says how to name it,
/// else that the workspace has no tasks. `tag` is
/// `crate::require_workspace_tag()`'s answer.
fn no_cards_text(tag: anyhow::Result<String>) -> String {
    match tag {
        Err(e) => e.to_string(),
        Ok(_) => NO_TASKS.to_string(),
    }
}

/// Open the box for a request from the CLI, and do the taskwarrior work when it
/// is submitted — the CLI has already exited by then, so this side owns it. Or
/// hand the focused monitor's panel the keyboard, on its cards or on the project list.
fn serve_box_request(app: &Application, req: crate::ipc::Request) {
    use crate::{ipc::Request, notify, task, taskbox, text};

    match req {
        Request::Projects => {
            let output = niri::focused_workspace().ok().flatten().and_then(|w| w.output);
            let Some(panel) = output.and_then(|o| PANELS.with(|p| p.borrow().get(&o).cloned())) else {
                notify::tasks(NO_PANEL);
                return;
            };
            match crate::project::Projects::load() {
                Ok(projects) => panel.open_projects(projects),
                Err(e) => notify::tasks(&e.to_string()),
            }
        }
        Request::Panel => {
            let focused = niri::focused_workspace().ok().flatten();
            let output = focused.as_ref().and_then(|w| w.output.clone());
            let panel = output
                .as_ref()
                .and_then(|o| PANELS.with(|p| p.borrow().get(o).cloned()));
            if !panel.is_some_and(|p| {
                // One `herdr agent list` per slide-out, for every card at
                // once; a session that is not running answers at once with
                // none.
                let agents = focused
                    .as_ref()
                    .and_then(|w| w.name.as_deref())
                    .map(crate::link::live_agent_names)
                    .unwrap_or_default();
                p.take_keyboard(agents)
            }) {
                notify::tasks(&no_cards_text(crate::require_workspace_tag()));
            }
        }

        Request::Add { refine } => {
            let tag = match crate::require_workspace_tag() {
                Ok(t) => t,
                Err(e) => {
                    notify::tasks(&e.to_string());
                    return;
                }
            };
            let tag_for_submit = tag.clone();
            let opened = taskbox::open_in(
                app,
                taskbox::BoxConfig::add(&tag, refine),
                move |sub: taskbox::Submission| {
                    match task::add_with_notes(
                        &tag_for_submit,
                        &text::add_args(&sub.description),
                        &sub.note_texts(),
                    ) {
                        Err(e) => notify::tasks(&e.to_string()),
                        Ok(uuid) => {
                            notify::tasks(&format!("Added to +{tag_for_submit}: {}", sub.description));
                            // Spawned, not run: herdr must not hold up the
                            // panels' main loop.
                            if let (true, Some(uuid)) = (sub.refine, uuid) {
                                crate::refine::spawn_quick(&uuid);
                            }
                        }
                    }
                },
            );
            if let Some(notice) = already_open_notice(opened) {
                notify::tasks(notice);
            }
        }

        Request::Edit(uuid) => open_task_box(app, uuid, taskbox::Mode::Edit),
        Request::Note(uuid) => open_task_box(app, uuid, taskbox::Mode::Note),
    }
}

/// Open the task box on an existing task, and save what comes back. Edit and
/// Note are one window; the mode only says where the cursor starts.
fn open_task_box(app: &Application, uuid: String, mode: crate::taskbox::Mode) {
    use crate::{notify, task, taskbox};

    let Ok(Some(t)) = task::get(&uuid) else {
        notify::tasks("Task not found");
        return;
    };
    // The full uuid, not the one asked for, which may be a prefix.
    let uuid = t.uuid.clone();
    let opened = taskbox::open_in(
        app,
        taskbox::BoxConfig::for_task(mode, t),
        move |sub: taskbox::Submission| {
            if let Err(e) = task::replace_text(&uuid, &sub.description, &sub.notes) {
                notify::tasks(&e.to_string());
            }
        },
    );
    if let Some(notice) = already_open_notice(opened) {
        notify::tasks(notice);
    }
}

fn tick(panels: &Panels, state: &Rc<RefCell<State>>) {
    // niri unreachable reads as "no monitor shows a named workspace", which
    // hides every panel rather than leaving stale cards up.
    let outputs = niri::workspaces()
        .map(|ws| niri::active_workspace_by_output(&ws))
        .unwrap_or_default();
    let mtimes = state
        .borrow()
        .data_dir
        .as_deref()
        .map(db_mtimes)
        .unwrap_or_default();

    {
        let s = state.borrow();
        if s.outputs.as_ref() == Some(&outputs) && s.task_mtimes == mtimes {
            return; // nothing moved; skip the subprocess
        }
    }
    debug(&format!("changed: outputs={outputs:?} panels={}", panels.borrow().len()));

    // Two monitors on the same workspace name share one query.
    let mut by_name: HashMap<Option<String>, Vec<model::Card>> = HashMap::new();
    for (connector, panel) in panels.borrow().iter() {
        let name = outputs.get(connector).cloned().flatten();
        // The panel's tag, for the notepad its Ideas tab opens.
        let t = tag::workspace_tag(name.as_deref().unwrap_or_default());
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(&t, cards);
    }

    let mut s = state.borrow_mut();
    s.outputs = Some(outputs);
    s.task_mtimes = mtimes;
}

/// Give every monitor a panel, keyed by its connector, and drop panels whose
/// monitor went away. Forgets what was drawn afterwards, so the next tick
/// fills any new panel even though niri's outputs have not changed.
///
/// A monitor GTK lists before it knows its connector gets no panel yet: on
/// Wayland a reconnected display is added a moment before its name arrives,
/// and a panel keyed any other way matches no niri output — so it stayed
/// empty, which is how the external monitor's panel vanished after a
/// hotplug until the daemon restarted. Its `connector` arriving runs this
/// again instead.
fn sync_monitors(app: &Application, display: &gdk::Display, panels: &Panels, state: &Rc<RefCell<State>>) {
    let monitors = display.monitors();
    let mut live: Vec<String> = Vec::new();

    for i in 0..monitors.n_items() {
        let Some(obj) = monitors.item(i) else { continue };
        let Ok(monitor) = obj.downcast::<gdk::Monitor>() else {
            continue;
        };
        let Some(key) = monitor.connector().map(|c| c.to_string()) else {
            let (app, display, panels, state) = (app.clone(), display.clone(), panels.clone(), state.clone());
            monitor.connect_connector_notify(move |_| sync_monitors(&app, &display, &panels, &state));
            continue;
        };
        live.push(key.clone());

        if panels.borrow().contains_key(&key) {
            continue;
        }
        let panel = Panel::new(app, &monitor);
        panels.borrow_mut().insert(key, panel);
    }

    // Drop panels for monitors that went away, so an unplugged display does
    // not leave an orphan.
    panels.borrow_mut().retain(|key, panel| {
        let keep = live.contains(key);
        if !keep {
            panel.close();
        }
        keep
    });
    debug(&format!("panels: {:?}", panels.borrow().keys().collect::<Vec<_>>()));
    state.borrow_mut().outputs = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mod+Alt+Ctrl+T with no card to take the keyboard for: an unnamed
    /// workspace says what the tag refusal says, a named one that it is empty.
    #[test]
    fn no_cards_says_why() {
        assert_eq!(no_cards_text(Ok("web".into())), NO_TASKS);
        assert_eq!(
            no_cards_text(Err(anyhow::anyhow!("This workspace has no name — Mod+Alt+W opens a project on a named one."))),
            "This workspace has no name — Mod+Alt+W opens a project on a named one."
        );
    }

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

    #[test]
    fn an_unnamed_workspace_has_no_cards() {
        assert!(cards_for_workspace(None).is_empty());
        assert!(cards_for_workspace(Some("日本")).is_empty(), "no usable tag characters");
    }

    /// Only a request for a different box hears that one is already open: the
    /// same box asked for twice is simply brought forward, which says enough.
    #[test]
    fn only_another_box_says_one_is_open() {
        use crate::taskbox::Opened;
        assert_eq!(already_open_notice(Opened::New), None);
        assert_eq!(already_open_notice(Opened::Same), None);
        assert_eq!(already_open_notice(Opened::Other), Some(BOX_ALREADY_OPEN));
    }
}
