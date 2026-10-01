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
//!   2. the mtime of taskwarrior's `pending.data` (a stat).
//!
//! `task export` is a subprocess and the expensive part, so it runs only when
//! one of those actually moved, and once per workspace tag rather than once
//! per monitor. Idle cost is a socket round-trip and a stat.

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
    task_mtime: Option<SystemTime>,
}

fn pending_data_path() -> Option<std::path::PathBuf> {
    // Honour TASKDATA so a sandboxed run watches the right file.
    if let Some(d) = std::env::var_os("TASKDATA") {
        return Some(std::path::PathBuf::from(d).join("pending.data"));
    }
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".task/pending.data"))
}

fn task_db_mtime() -> Option<SystemTime> {
    std::fs::metadata(pending_data_path()?).ok()?.modified().ok()
}

/// Every task card for a workspace, or none when it is unnamed or empty. The
/// panel does the capping.
fn cards_for_workspace(name: Option<&str>) -> Vec<model::Card> {
    let t = tag::workspace_tag(name.unwrap_or_default());
    if t.is_empty() {
        return Vec::new();
    }
    let Ok(tasks) = task::pending_for_tag(&t) else {
        return Vec::new();
    };
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
    let state = Rc::new(RefCell::new(State::default()));

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

    // Serve task-box requests. The daemon is already a warm GTK process, so a
    // box it opens appears immediately instead of paying ~0.6s (2.6s cold) to
    // start another one. Failing to listen is not fatal: the CLI falls back to
    // building the box itself, which is the whole reason this is a cache and
    // not a dependency.
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
        eprintln!("task box over IPC unavailable ({e}); the CLI will open its own");
    }
}

/// `NIRITASKS_DEBUG=1` traces each tick to stderr — which, under the systemd unit,
/// means `journalctl --user -u niri-tasks`.
fn debug(msg: &str) {
    if std::env::var_os("NIRITASKS_DEBUG").is_some() {
        eprintln!("[daemon] {msg}");
    }
}

/// Open the box for a request from the CLI, and do the taskwarrior work when it
/// is submitted — the CLI has already exited by then, so this side owns it. Or
/// hand the focused monitor's panel the keyboard.
fn serve_box_request(app: &Application, req: crate::ipc::Request) {
    use crate::{ipc::Request, notify, task, taskbox, text};

    match req {
        Request::Panel => {
            let focused = niri::focused_workspace().ok().flatten();
            let output = focused.as_ref().and_then(|w| w.output.clone());
            let panel = output
                .as_ref()
                .and_then(|o| PANELS.with(|p| p.borrow().get(o).cloned()));
            if !panel.is_some_and(|p| {
                // One `herdr agent list` per slide-out, for every card at
                // once, and none when the fuzzel list takes over; a session
                // that is not running answers at once with none.
                let agents = focused
                    .as_ref()
                    .and_then(|w| w.name.as_deref())
                    .map(crate::link::live_agent_names)
                    .unwrap_or_default();
                p.take_keyboard(agents)
            }) {
                // No cards to pick from: the fuzzel list instead, with its
                // "Add task" row, or its "name this workspace" error.
                crate::panel::surface::open_menu(
                    output.as_deref().unwrap_or_default(),
                    &["task".into(), "list".into()],
                );
            }
        }

        Request::Add => {
            let tag = match crate::require_workspace_tag() {
                Ok(t) => t,
                Err(e) => {
                    notify::tasks(&e.to_string());
                    return;
                }
            };
            let tag_for_submit = tag.clone();
            taskbox::open_in(
                app,
                taskbox::BoxConfig::add(&tag),
                move |sub: taskbox::Submission| {
                    if let Err(e) = task::add_with_notes(
                        &tag_for_submit,
                        &text::add_args(&sub.description),
                        &sub.note_texts(),
                    ) {
                        notify::tasks(&e.to_string());
                    } else {
                        notify::tasks(&format!("Added to +{tag_for_submit}: {}", sub.description));
                    }
                },
            );
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
    taskbox::open_in(
        app,
        taskbox::BoxConfig::for_task(mode, t),
        move |sub: taskbox::Submission| {
            if let Err(e) = task::replace_text(&uuid, &sub.description, &sub.notes) {
                notify::tasks(&e.to_string());
            }
        },
    );
}

fn tick(panels: &Panels, state: &Rc<RefCell<State>>) {
    // niri unreachable reads as "no monitor shows a named workspace", which
    // hides every panel rather than leaving stale cards up.
    let outputs = niri::workspaces()
        .map(|ws| niri::active_workspace_by_output(&ws))
        .unwrap_or_default();
    let mtime = task_db_mtime();

    {
        let s = state.borrow();
        if s.outputs.as_ref() == Some(&outputs) && s.task_mtime == mtime {
            return; // nothing moved; skip the subprocess
        }
    }
    debug(&format!("changed: outputs={outputs:?} panels={}", panels.borrow().len()));

    // Two monitors on the same workspace name share one query.
    let mut by_name: HashMap<Option<String>, Vec<model::Card>> = HashMap::new();
    for (connector, panel) in panels.borrow().iter() {
        let name = outputs.get(connector).cloned().flatten();
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(cards);
    }

    let mut s = state.borrow_mut();
    s.outputs = Some(outputs);
    s.task_mtime = mtime;
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

    #[test]
    fn pending_data_path_follows_taskdata() {
        std::env::set_var("TASKDATA", "/tmp/somewhere");
        assert_eq!(
            pending_data_path().unwrap(),
            std::path::PathBuf::from("/tmp/somewhere/pending.data")
        );
        std::env::remove_var("TASKDATA");
    }

    #[test]
    fn an_unnamed_workspace_has_no_cards() {
        assert!(cards_for_workspace(None).is_empty());
        assert!(cards_for_workspace(Some("日本")).is_empty(), "no usable tag characters");
    }
}
