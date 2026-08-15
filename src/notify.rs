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

pub fn project(body: &str) {
    notify("Open Project", body);
}
