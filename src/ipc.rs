//! Asking the running daemon to open the task box, to hand the task panel
//! the keyboard, to show its project list, or to peek at a task.
//!
//! The box once cost 0.6s to 2.6s to appear when each `niritasks task add`
//! built it in a process of its own; the daemon's warm GTK process puts it
//! up at once.
//!
//! The daemon is the one process that draws: the panels, the box and the
//! project list. The CLI asks it over this socket and, when nothing answers,
//! says so with [`NO_DAEMON`] rather than building a window of its own, which
//! would mean a second copy of what the box does on submit.
//!
//! The protocol is one line per request, because it only ever carries a mode,
//! a workspace tag, a uuid and the add box's refine flag:
//!
//! ```text
//! add
//! add refine
//! edit <uuid>
//! note <uuid>
//! panel
//! panel <uuid>
//! projects
//! peek <tag> <uuid>
//! peek <tag>
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
    /// Slide out the focused monitor's task panel and give it the keyboard,
    /// focused on this task's card when one is named, else the first card.
    Panel(Option<String>),
    /// Show the focused monitor's project list, for opening a project, with
    /// the keyboard: Mod+Alt+W.
    Projects,
    /// Peek at this task's card on the edge of the panel showing this
    /// workspace tag, while it is tucked away, or at nothing: herdr's focused
    /// pane, from task-docs' focus hooks. Kept per tag, so a workspace on no
    /// monitor yet shows it once a monitor shows that workspace.
    Peek { tag: String, uuid: Option<String> },
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Add { refine: false } => "add".into(),
            Request::Add { refine: true } => "add refine".into(),
            Request::Edit(uuid) => format!("edit {uuid}"),
            Request::Note(uuid) => format!("note {uuid}"),
            Request::Panel(None) => "panel".into(),
            Request::Panel(Some(uuid)) => format!("panel {uuid}"),
            Request::Projects => "projects".into(),
            Request::Peek { tag, uuid: None } => format!("peek {tag}"),
            Request::Peek { tag, uuid: Some(uuid) } => format!("peek {tag} {uuid}"),
        }
    }

    pub fn decode(line: &str) -> Option<Self> {
        let mut parts = line.trim().splitn(2, ' ');
        match (parts.next()?, parts.next()) {
            ("add", rest) => Some(Request::Add { refine: rest == Some("refine") }),
            ("panel", rest) => Some(Request::Panel(rest.map(str::trim).filter(|u| !u.is_empty()).map(String::from))),
            ("projects", _) => Some(Request::Projects),
            ("edit", Some(uuid)) if !uuid.is_empty() => Some(Request::Edit(uuid.to_string())),
            ("note", Some(uuid)) if !uuid.is_empty() => Some(Request::Note(uuid.to_string())),
            ("peek", Some(rest)) => {
                let mut words = rest.split_whitespace();
                let tag = words.next()?.to_string();
                Some(Request::Peek { tag, uuid: words.next().map(String::from) })
            }
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
    writeln!(stream, "{}", req.encode()).context(NO_DAEMON)?;
    stream.flush().context(NO_DAEMON)?;
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

    /// A bare `panel`, from Mod+Alt+Ctrl+T or an older CLI, still opens on
    /// the first card; `panel <uuid>` names the card to open on.
    #[test]
    fn panel_carries_an_optional_uuid() {
        assert_eq!(Request::Panel(None).encode(), "panel");
        assert_eq!(Request::Panel(Some("abc".into())).encode(), "panel abc");
        assert_eq!(Request::decode("panel\n"), Some(Request::Panel(None)));
        assert_eq!(Request::decode("panel \n"), Some(Request::Panel(None)));
        assert_eq!(Request::decode("panel abc\n"), Some(Request::Panel(Some("abc".into()))));
    }

    /// A peek names the workspace tag it is for, then the task, or none to
    /// peek at nothing; with no tag it is no request.
    #[test]
    fn peek_carries_a_tag_and_an_optional_uuid() {
        let peek = |uuid: Option<&str>| Request::Peek { tag: "e2e".into(), uuid: uuid.map(String::from) };
        assert_eq!(peek(Some("abc")).encode(), "peek e2e abc");
        assert_eq!(peek(None).encode(), "peek e2e");
        assert_eq!(Request::decode("peek e2e abc\n"), Some(peek(Some("abc"))));
        assert_eq!(Request::decode("peek e2e\n"), Some(peek(None)));
        assert_eq!(Request::decode("peek\n"), None);
        assert_eq!(Request::decode("peek \n"), None);
    }

    #[test]
    fn round_trips_every_request() {
        for req in [
            Request::Add { refine: false },
            Request::Add { refine: true },
            Request::Edit("abc-123".into()),
            Request::Note("def-456".into()),
            Request::Panel(None),
            Request::Panel(Some("9e400fbc-d371-4da6-afe1-6320df840239".into())),
            Request::Projects,
            Request::Peek { tag: "web".into(), uuid: Some("9e400fbc-d371-4da6-afe1-6320df840239".into()) },
            Request::Peek { tag: "web".into(), uuid: None },
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
        let err = send(&Request::Panel(None)).unwrap_err();
        assert_eq!(err.to_string(), NO_DAEMON);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
