//! The active-task overlay.
//!
//! A block of text anchored to the bottom of every monitor, showing the active
//! task for whatever workspace is focused, and hidden entirely when there is
//! none.
//!
//! This replaces `ActiveTaskWidget.qml`, which rendered inside the DMS bar.
//! DMS has no generic "run a command and show its output" widget, so a pill in
//! that bar necessarily meant shipping a plugin; a layer-shell surface owned by
//! this binary means DMS is optional, which is the point.
//!
//! **Margin is a plain config value, not a probe of DMS's bar height.** That is
//! what keeps this simple: nothing here has to know whether a bar exists, how
//! tall it is, or whether it was just toggled off.
//!
//! ## Refresh strategy
//!
//! Rather than subscribe to niri's event stream and watch the task database
//! with a file-watcher — two background threads and their reconnect logic — the
//! tick does two cheap checks and only does real work when one of them changes:
//!
//!   1. the focused workspace name, over the niri socket (no subprocess), and
//!   2. the mtime of taskwarrior's `pending.data` (a stat).
//!
//! `task export` is a subprocess and the expensive part, so it runs only when
//! one of those actually moved. Idle cost is a socket round-trip and a stat.

use crate::{niri, tag, task, theme::Theme};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

pub const APP_ID: &str = "dev.niri-tasks.overlay";

/// Layer-shell namespace, so `layer-rule { match namespace="niri-tasks" }` works.
pub const NAMESPACE: &str = "niri-tasks";

/// How far above the bottom edge the text sits. Tuned once for a bar of the
/// usual height; it is a preference, not a measurement.
///
/// Sized to sit just clear of a bottom bar: anything much above this reads as a
/// stray label floating over the wallpaper rather than something belonging to
/// the bar's row. Nudge it with `WT_OVERLAY_MARGIN` — a taller bar wants more.
const DEFAULT_BOTTOM_MARGIN: i32 = 10;

/// The widest the pill is allowed to get, in characters.
///
/// A ceiling, not a width. The pill hugs its text, so a two-word task gets a
/// two-word pill; this is only the point at which it stops growing and starts
/// ellipsising, so that one long description cannot stretch a readout the width
/// of the monitor.
const DEFAULT_MAX_WIDTH_CHARS: i32 = 80;

fn max_width_chars() -> i32 {
    std::env::var("WT_OVERLAY_MAX_WIDTH_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_WIDTH_CHARS)
}

const TICK: Duration = Duration::from_millis(700);

fn bottom_margin() -> i32 {
    std::env::var("WT_OVERLAY_MARGIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_BOTTOM_MARGIN)
}

/// What the overlay last displayed, so a tick that changes nothing does nothing.
#[derive(Default)]
struct State {
    workspace: Option<String>,
    task_mtime: Option<SystemTime>,
    shown: String,
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

/// The active task's description for the focused workspace, or empty.
fn current_active() -> String {
    let Ok(Some(name)) = niri::focused_workspace_name() else {
        return String::new();
    };
    let t = tag::workspace_tag(&name);
    if t.is_empty() {
        return String::new();
    }
    match task::active_for_tag(&t) {
        Ok(Some(task)) => task.description,
        _ => String::new(),
    }
}

pub fn run() -> anyhow::Result<()> {
    // Startup housekeeping that used to be its own spawn-at-startup script:
    // give workspace 1 a name so it has a tag from the first moment. Failure is
    // not fatal — the overlay is still worth running.
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
    let theme = Theme::load();
    let provider = CssProvider::new();
    provider.load_from_data(&overlay_css(&theme));
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // One surface per monitor, keyed by connector name so hotplug can add and
    // remove them individually.
    let surfaces: Rc<RefCell<HashMap<String, (ApplicationWindow, gtk4::Label)>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let state = Rc::new(RefCell::new(State::default()));

    let Some(display) = gdk::Display::default() else {
        eprintln!("no display");
        return;
    };

    sync_monitors(app, &display, &surfaces);

    // Hotplug: plugging in the external monitor, or Super+Alt+Comma blanking
    // the built-in one, changes this list.
    {
        let app = app.clone();
        let display2 = display.clone();
        let surfaces = surfaces.clone();
        display.monitors().connect_items_changed(move |_, _, _, _| {
            sync_monitors(&app, &display2, &surfaces);
        });
    }

    // Hold the application open: layer-shell surfaces are not "windows" in the
    // sense GTK counts for lifetime, and an empty overlay has none mapped at
    // all, so without this the app would quit as soon as the text went away.
    let _hold = app.hold();

    let surfaces_tick = surfaces.clone();
    let state_tick = state.clone();
    gtk4::glib::timeout_add_local(TICK, move || {
        tick(&surfaces_tick, &state_tick);
        gtk4::glib::ControlFlow::Continue
    });

    // Paint once immediately rather than waiting out the first tick.
    tick(&surfaces, &state);

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

/// `WT_DEBUG=1` traces each tick to stderr — which, under the systemd unit,
/// means `journalctl --user -u niri-tasks`.
fn debug(msg: &str) {
    if std::env::var_os("WT_DEBUG").is_some() {
        eprintln!("[overlay] {msg}");
    }
}

/// Open the box for a request from the CLI, and do the taskwarrior work when it
/// is submitted — the CLI has already exited by then, so this side owns it.
fn serve_box_request(app: &Application, req: crate::ipc::Request) {
    use crate::{ipc::Request, notify, task, taskbox, text};

    match req {
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
                taskbox::BoxConfig {
                    mode: taskbox::Mode::Add,
                    subtitle: format!("+{tag}"),
                    initial: String::new(),
                    notes: String::new(),
                },
                move |sub: taskbox::Submission| {
                    if let Err(e) = task::add_with_notes(
                        &tag_for_submit,
                        &text::add_args(&sub.text),
                        &sub.notes,
                    ) {
                        notify::tasks(&e.to_string());
                    } else {
                        notify::tasks(&format!("Added to +{tag_for_submit}: {}", sub.text));
                    }
                },
            );
        }

        Request::Edit(uuid) => {
            let Ok(Some(t)) = task::get(&uuid) else {
                notify::tasks("Task not found");
                return;
            };
            let uuid_for_submit = uuid.clone();
            taskbox::open_in(
                app,
                taskbox::BoxConfig {
                    mode: taskbox::Mode::Edit,
                    subtitle: String::new(),
                    initial: t.description,
                    notes: String::new(),
                },
                move |sub: taskbox::Submission| {
                    if let Err(e) = task::modify_description(&uuid_for_submit, &sub.text) {
                        notify::tasks(&e.to_string());
                    }
                },
            );
        }

        Request::Note(uuid) => {
            let Ok(Some(t)) = task::get(&uuid) else {
                notify::tasks("Task not found");
                return;
            };
            let notes = t
                .annotations
                .iter()
                .map(|a| {
                    let date = a.entry.get(..8).unwrap_or(&a.entry);
                    format!("{date}  {}", text::collapse_whitespace(&a.description))
                })
                .collect::<Vec<_>>()
                .join("\n");
            let uuid_for_submit = uuid.clone();
            taskbox::open_in(
                app,
                taskbox::BoxConfig {
                    mode: taskbox::Mode::Annotate,
                    subtitle: t.description,
                    initial: String::new(),
                    notes,
                },
                move |sub: taskbox::Submission| {
                    if let Err(e) = task::annotate(&uuid_for_submit, &sub.text) {
                        notify::tasks(&e.to_string());
                    }
                },
            );
        }
    }
}

fn tick(
    surfaces: &Rc<RefCell<HashMap<String, (ApplicationWindow, gtk4::Label)>>>,
    state: &Rc<RefCell<State>>,
) {
    let workspace = niri::focused_workspace_name().ok().flatten();
    let mtime = task_db_mtime();

    {
        let s = state.borrow();
        if s.workspace == workspace && s.task_mtime == mtime {
            return; // nothing moved; skip the subprocess
        }
    }

    let text = current_active();
    debug(&format!(
        "changed: workspace={workspace:?} surfaces={} active={text:?}",
        surfaces.borrow().len()
    ));

    let mut s = state.borrow_mut();
    s.workspace = workspace;
    s.task_mtime = mtime;
    if s.shown == text {
        return;
    }
    s.shown = text.clone();
    drop(s);

    // Hide the surface entirely when there is no active task, rather than
    // showing an empty strip.
    for (window, label) in surfaces.borrow().values() {
        if text.is_empty() {
            // Only hide something that is actually up. Hiding a layer-shell
            // window that has never been mapped leaves it in a state where a
            // later present() silently does nothing — so a daemon that started
            // with no active task would stay invisible for the whole session,
            // however many tasks you started afterwards.
            if window.is_visible() {
                window.set_visible(false);
            }
        } else {
            label.set_text(&text);
            // Unmap before mapping again, so the surface is sized for the text
            // it is about to show. The width is negotiated once, when the
            // surface maps, and changing the label afterwards does not make it
            // ask again — so without this the pill keeps the previous task's
            // width, and a short description sits marooned in the middle of a
            // pill cut for a long one (or a long one ellipsises down to a
            // couple of letters in a pill cut for a short one).
            if window.is_visible() {
                window.set_visible(false);
            }
            // present(), not set_visible(true): a layer surface that has never
            // been presented is not mapped by set_visible alone, so the overlay
            // would stay invisible for the whole session whenever it started
            // with no active task.
            window.present();
        }
    }
}

fn sync_monitors(
    app: &Application,
    display: &gdk::Display,
    surfaces: &Rc<RefCell<HashMap<String, (ApplicationWindow, gtk4::Label)>>>,
) {
    let monitors = display.monitors();
    let mut live: Vec<String> = Vec::new();

    for i in 0..monitors.n_items() {
        let Some(obj) = monitors.item(i) else { continue };
        let Ok(monitor) = obj.downcast::<gdk::Monitor>() else {
            continue;
        };
        let key = monitor
            .connector()
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("monitor-{i}"));
        live.push(key.clone());

        if surfaces.borrow().contains_key(&key) {
            continue;
        }
        let created = make_surface(app, &monitor);
        surfaces.borrow_mut().insert(key, created);
    }

    // Drop surfaces for monitors that went away, so an unplugged display does
    // not leave an orphan.
    surfaces.borrow_mut().retain(|key, (window, _)| {
        let keep = live.contains(key);
        if !keep {
            window.close();
        }
        keep
    });
}

fn make_surface(app: &Application, monitor: &gdk::Monitor) -> (ApplicationWindow, gtk4::Label) {
    let window = ApplicationWindow::builder().application(app).build();

    window.init_layer_shell();
    // Name the surface, so niri layer-rules can target it — the crate's default
    // namespace is just "gtk4-layer-shell", which any GTK layer-shell app on the
    // system would also answer to.
    window.set_namespace(Some(NAMESPACE));
    window.set_layer(Layer::Top);
    window.set_monitor(Some(monitor));
    window.set_anchor(Edge::Bottom, true);
    window.set_margin(Edge::Bottom, bottom_margin());
    // Never take focus or reserve space: this is a readout, not a control, and
    // an exclusive zone would push every window up by its height.
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
    window.set_exclusive_zone(0);

    let label = gtk4::Label::new(None);
    label.add_css_class("active-task");
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    // max-width-chars only. width-chars would be a *minimum* as well as a
    // maximum, and a pill held open to 80 characters around the words "ship it"
    // is a band across the wallpaper rather than a label on a task. Setting
    // neither is not the answer either: an ellipsizing label with no ceiling
    // would let one long description run the width of the monitor.
    label.set_max_width_chars(max_width_chars());

    window.set_child(Some(&label));

    // Map the layer surface once, then hide it again in the same main-loop
    // iteration so nothing is painted.
    //
    // This looks redundant and is not: a layer-shell window that has never been
    // mapped does not respond to a later present(). Without this, a daemon that
    // started with no active task stayed invisible for the entire session no
    // matter how many tasks were started afterwards — the tick ran, found the
    // task, called present(), and nothing appeared.
    window.present();
    window.set_visible(false);

    (window, label)
}

/// How far from opaque the pill sits.
///
/// Low enough that the pill reads as part of the desktop rather than a chip
/// sitting on top of it — more see-through than the picker's 0.5.
///
/// Kept as a constant rather than inlined because it is the one dial that
/// decides whether the `layer-rule` in `niri/niri-tasks.kdl` does anything: that
/// rule blurs what is behind this surface, and the blur is only visible *through*
/// the background. At this alpha the blur is doing most of the work of keeping
/// the text legible, so the two belong together.
const BACKGROUND_ALPHA: f32 = 0.3;

/// The pill's corner radius, matching `fuzzel/picker.ini`'s `[border] radius`
/// so the two surfaces this tool puts on screen are cut the same way.
const CORNER_RADIUS: i32 = 14;

/// The overlay's stylesheet.
///
/// A rounded, translucent pill — the same treatment as the task box and the
/// picker, rather than bare text on the wallpaper.
fn overlay_css(theme: &Theme) -> String {
    format!(
        "
window {{ background-color: transparent; }}
label.active-task {{
    color: {fg};
    background-color: {bg};
    padding: 6px 16px;
    border-radius: {radius}px;
    font-size: 105%;
}}
",
        fg = theme.surface_text,
        bg = crate::theme::with_alpha(&theme.surface_container, BACKGROUND_ALPHA),
        radius = CORNER_RADIUS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margin_defaults_and_is_overridable() {
        std::env::remove_var("WT_OVERLAY_MARGIN");
        assert_eq!(bottom_margin(), DEFAULT_BOTTOM_MARGIN);

        std::env::set_var("WT_OVERLAY_MARGIN", "120");
        assert_eq!(bottom_margin(), 120);

        // Garbage falls back rather than panicking a long-running daemon.
        std::env::set_var("WT_OVERLAY_MARGIN", "not-a-number");
        assert_eq!(bottom_margin(), DEFAULT_BOTTOM_MARGIN);
        std::env::remove_var("WT_OVERLAY_MARGIN");
    }

    #[test]
    fn width_defaults_and_is_overridable() {
        std::env::remove_var("WT_OVERLAY_MAX_WIDTH_CHARS");
        assert_eq!(max_width_chars(), DEFAULT_MAX_WIDTH_CHARS);

        std::env::set_var("WT_OVERLAY_MAX_WIDTH_CHARS", "120");
        assert_eq!(max_width_chars(), 120);

        // Garbage falls back rather than panicking a long-running daemon.
        std::env::set_var("WT_OVERLAY_MAX_WIDTH_CHARS", "wide-ish");
        assert_eq!(max_width_chars(), DEFAULT_MAX_WIDTH_CHARS);
        std::env::remove_var("WT_OVERLAY_MAX_WIDTH_CHARS");
    }

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
    fn overlay_css_substitutes_every_token() {
        let css = overlay_css(&Theme::default());
        assert!(css.contains(&Theme::default().surface_text));
        for placeholder in ["{fg}", "{bg}"] {
            assert!(!css.contains(placeholder));
        }
    }

    /// The pill draws a themed background at the configured alpha, and is
    /// rounded. Both are what separate it from bare text on the wallpaper.
    #[test]
    fn the_pill_is_filled_and_rounded() {
        let theme = Theme::default();
        let css = overlay_css(&theme);

        assert!(
            css.contains(&crate::theme::with_alpha(
                &theme.surface_container,
                BACKGROUND_ALPHA
            )),
            "background is not the themed colour at the expected alpha"
        );
        assert!(css.contains(&format!("border-radius: {CORNER_RADIUS}px")));
    }

    /// The alpha and the compositor rule are coupled across repositories: the
    /// `layer-rule` in `niri/niri-tasks.kdl` blurs what is behind this surface,
    /// and that blur is only visible *through* the background. This does not
    /// forbid an opaque pill — it pins the relationship down so that whoever
    /// changes the alpha finds out that the rule's fate hangs on it.
    #[test]
    fn opacity_decides_whether_the_blur_rule_does_anything() {
        assert!(
            (0.0..=1.0).contains(&BACKGROUND_ALPHA),
            "alpha outside 0..=1 renders as a GTK parse error, not a colour"
        );
        if BACKGROUND_ALPHA >= 1.0 {
            // Dormant by choice. If the rule is ever deleted as dead config,
            // lowering the alpha again must bring it back with it.
            let kdl = include_str!("../niri/niri-tasks.kdl");
            assert!(
                kdl.contains("blur true"),
                "opaque pill plus no blur rule: lowering the alpha will now do nothing"
            );
        }
    }
}
