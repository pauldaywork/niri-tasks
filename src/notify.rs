//! Desktop notifications, with a stderr fallback.
//!
//! Ported from `task_notify` / `notify` in the shell scripts. Failure to notify
//! is never fatal — the notification is feedback about work that already
//! happened, so a missing `notify-send` must not turn a completed action into
//! an error.

use std::process::Command;

pub fn notify(summary: &str, body: &str) {
    let sent = Command::new("notify-send")
        .arg(summary)
        .arg(body)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !sent {
        eprintln!("{body}");
    }
}

pub fn tasks(body: &str) {
    notify("Tasks", body);
}

/// An error, to both channels.
///
/// stderr because a script or an agent gets nothing else: `niritasks tag --session`
/// fails differently depending on whether you are outside any herdr session or
/// project folder, or on a workspace that has been renamed, and an exit code
/// cannot say which. The
/// desktop because these also run from keybinds, where there is no terminal for
/// stderr to reach. Sending it to one of the two leaves whichever caller it was
/// not holding a bare 1.
pub fn error(body: &str) {
    eprintln!("{body}");
    // No stderr fallback needed here — the line above is unconditional.
    let _ = Command::new("notify-send").arg("Tasks").arg(body).status();
}

pub fn project(body: &str) {
    notify("Open Project", body);
}
