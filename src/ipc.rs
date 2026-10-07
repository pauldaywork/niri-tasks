//! Asking the running daemon to open the task box, to hand the task panel
//! the keyboard, or to show its project list.
//!
//! The box is a GTK window, and building one costs about 2.6s on a cold start
//! and 0.6s warm — every time, because each `niritasks task add` was its own process.
//! The daemon is already a warm GTK process holding the task panels, so it can put
//! the window up immediately instead.
//!
//! The daemon is the one process that draws: the panels, the box and the
//! project list. The CLI asks it over this socket and, when nothing answers,
//! says so with [`NO_DAEMON`] rather than building a window of its own, which
//! would mean a second copy of what the box does on submit.
//!
//! The protocol is one line per request, because it only ever carries a mode,
//! a uuid and the add box's refine flag:
//!
//! ```text
//! add
//! add refine
//! edit <uuid>
//! note <uuid>
//! panel
//! projects
//! ```

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// What the CLI asked the daemon to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// The add box; `refine` makes Add & refine its default button.
    Add { refine: bool },
    Edit(String),
    Note(String),
    /// Slide out the focused monitor's task panel and give it the keyboard.
    Panel,
    /// Show the focused monitor's project list, for opening a project, with
    /// the keyboard: Mod+Alt+W.
    Projects,
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Add { refine: false } => "add".into(),
            Request::Add { refine: true } => "add refine".into(),
            Request::Edit(uuid) => format!("edit {uuid}"),
            Request::Note(uuid) => format!("note {uuid}"),
            Request::Panel => "panel".into(),
            Request::Projects => "projects".into(),
        }
    }

    pub fn decode(line: &str) -> Option<Self> {
        let mut parts = line.trim().splitn(2, ' ');
        match (parts.next()?, parts.next()) {
            ("add", rest) => Some(Request::Add { refine: rest == Some("refine") }),
            ("panel", _) => Some(Request::Panel),
            ("projects", _) => Some(Request::Projects),
            ("edit", Some(uuid)) if !uuid.is_empty() => Some(Request::Edit(uuid.to_string())),
            ("note", Some(uuid)) if !uuid.is_empty() => Some(Request::Note(uuid.to_string())),
            _ => None,
        }
    }
}

/// Under $XDG_RUNTIME_DIR so it is per-user, per-boot, and cleaned up for us.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join("niri-tasks.sock")
}

/// What every GUI command says when the daemon is not there to draw for it.
pub const NO_DAEMON: &str =
    "The niri-tasks daemon is not running, so there is nothing to draw the window. Start it with `systemctl --user start niri-tasks`.";

/// Ask the daemon to draw something. `Err` means no daemon is listening; the
/// daemon is the only process that draws, so the caller reports it rather
/// than drawing itself.
pub fn send(req: &Request) -> Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).context(NO_DAEMON)?;
    writeln!(stream, "{}", req.encode())?;
    stream.flush()?;
    Ok(())
}

/// Listen for requests, handing each to `on_request`.
///
/// Runs on its own thread; `on_request` is responsible for getting back onto
/// the GTK main loop.
pub fn listen<F>(on_request: F) -> Result<()>
where
    F: Fn(Request) + Send + 'static,
{
    let path = socket_path();

    // A socket left behind by a killed daemon would make bind() fail with
    // EADDRINUSE forever. Only remove it when nothing is actually listening,
    // so two daemons racing cannot steal each other's socket.
    if path.exists() && UnixStream::connect(&path).is_err() {
        let _ = std::fs::remove_file(&path);
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("could not listen on {}", path.display()))?;

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                if let Some(req) = Request::decode(&line) {
                    on_request(req);
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_request() {
        for req in [
            Request::Add { refine: false },
            Request::Add { refine: true },
            Request::Edit("abc-123".into()),
            Request::Note("def-456".into()),
            Request::Panel,
            Request::Projects,
        ] {
            assert_eq!(Request::decode(&req.encode()), Some(req));
        }
    }

    /// `add` alone stays a plain add, so a box asked for by an older CLI
    /// opens as it always did.
    #[test]
    fn add_carries_whether_to_refine() {
        assert_eq!(Request::Add { refine: false }.encode(), "add");
        assert_eq!(Request::Add { refine: true }.encode(), "add refine");
        assert_eq!(Request::decode("add\n"), Some(Request::Add { refine: false }));
        assert_eq!(Request::decode("add refine\n"), Some(Request::Add { refine: true }));
        assert_eq!(Request::decode("add sideways"), Some(Request::Add { refine: false }));
    }

    /// Mod+Alt+W's project list is a request of its own, with no uuid.
    #[test]
    fn projects_is_one_word() {
        assert_eq!(Request::Projects.encode(), "projects");
        assert_eq!(Request::decode("projects\n"), Some(Request::Projects));
    }

    #[test]
    fn rejects_malformed_lines() {
        assert_eq!(Request::decode(""), None);
        assert_eq!(Request::decode("nonsense"), None);
        // A mode that needs a uuid and hasn't got one must not open a box
        // pointed at nothing.
        assert_eq!(Request::decode("edit"), None);
        assert_eq!(Request::decode("edit "), None);
        assert_eq!(Request::decode("note"), None);
    }

    #[test]
    fn tolerates_the_trailing_newline_writeln_adds() {
        assert_eq!(Request::decode("add\n"), Some(Request::Add { refine: false }));
        assert_eq!(
            Request::decode("edit abc\n"),
            Some(Request::Edit("abc".into()))
        );
    }

    /// uuids have no spaces, but splitting on the first one keeps the rest
    /// intact rather than silently truncating if that ever changes.
    #[test]
    fn keeps_everything_after_the_mode() {
        assert_eq!(
            Request::decode("note abc def"),
            Some(Request::Note("abc def".into()))
        );
    }

    /// Tests run on parallel threads and both of these set XDG_RUNTIME_DIR,
    /// so each holds this while the variable is theirs.
    static RUNTIME_DIR: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn socket_lives_under_the_runtime_dir() {
        let _env = RUNTIME_DIR.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1234");
        assert_eq!(socket_path(), PathBuf::from("/run/user/1234/niri-tasks.sock"));
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    /// With no daemon the error is the one sentence every GUI command shows.
    #[test]
    fn no_daemon_is_one_sentence() {
        let _env = RUNTIME_DIR.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("XDG_RUNTIME_DIR", "/nonexistent-niri-tasks-test");
        let err = send(&Request::Panel).unwrap_err();
        assert_eq!(err.to_string(), NO_DAEMON);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
