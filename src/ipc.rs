//! Asking the running daemon to open the task box.
//!
//! The box is a GTK window, and building one costs about 2.6s on a cold start
//! and 0.6s warm — every time, because each `niritasks task add` was its own process.
//! The daemon is already a warm GTK process holding the overlay, so it can put
//! the window up immediately instead.
//!
//! This is the same shape as the `dms ipc call taskBox` boundary that used to
//! exist, with one difference that matters: the daemon is ours. If it is not
//! running, the CLI builds the box itself and everything still works, which is
//! the property that made dropping the DMS plugin worth doing. A daemon you can
//! do without is a cache; one you cannot is a dependency.
//!
//! The protocol is one line per request, because it only ever carries a mode
//! and a uuid:
//!
//! ```text
//! add
//! edit <uuid>
//! note <uuid>
//! ```

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// What the CLI asked the daemon to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Add,
    Edit(String),
    Note(String),
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Add => "add".into(),
            Request::Edit(uuid) => format!("edit {uuid}"),
            Request::Note(uuid) => format!("note {uuid}"),
        }
    }

    pub fn decode(line: &str) -> Option<Self> {
        let mut parts = line.trim().splitn(2, ' ');
        match (parts.next()?, parts.next()) {
            ("add", _) => Some(Request::Add),
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

/// Ask the daemon to open the box. `Err` means no daemon — callers fall back to
/// building the window themselves rather than reporting a failure.
pub fn send(req: &Request) -> Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("no daemon listening on {}", path.display()))?;
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
            Request::Add,
            Request::Edit("abc-123".into()),
            Request::Note("def-456".into()),
        ] {
            assert_eq!(Request::decode(&req.encode()), Some(req));
        }
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
        assert_eq!(Request::decode("add\n"), Some(Request::Add));
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

    #[test]
    fn socket_lives_under_the_runtime_dir() {
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1234");
        assert_eq!(socket_path(), PathBuf::from("/run/user/1234/niri-tasks.sock"));
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
