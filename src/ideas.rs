//! The Ideas tab's notepads: one plain-text file per workspace tag, for ideas
//! that are not tasks yet.
//!
//! Plain files under the user's data directory rather than Taskwarrior, which
//! has nowhere to keep text that is not a task. Read and written whole: a
//! notepad is small, and the panel saves it only once typing stops.

use crate::dirs::Dirs;
use std::io;
use std::path::{Path, PathBuf};

/// Where a workspace tag's ideas live: `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md`,
/// else under `~/.local/share`. A relative `XDG_DATA_HOME` is ignored, as the
/// XDG spec says.
///
/// None for anything but a workspace tag: an empty one, which no workspace
/// with tasks has, or a string `tag::workspace_tag` would have changed. A tag
/// is only lowercase letters, digits and `_`, so it is always a plain file
/// name and never a path.
pub fn file(dirs: &Dirs, tag: &str) -> Option<PathBuf> {
    if tag.is_empty() || crate::tag::workspace_tag(tag) != tag {
        return None;
    }
    Some(dirs.data().join("niri-tasks/ideas").join(format!("{tag}.md")))
}

/// [`file`], from this process's `HOME` and `XDG_DATA_HOME`. None without a
/// `HOME`.
pub fn file_for(tag: &str) -> Option<PathBuf> {
    file(&Dirs::from_env().ok()?, tag)
}

/// A notepad's text. A file not there yet is a notepad with nothing in it.
pub fn read(path: &Path) -> io::Result<String> {
    match std::fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        other => other,
    }
}

/// Save `text` as the whole notepad, making its folder first. Written beside
/// it and renamed into place, so a daemon stopped mid-write leaves the ideas
/// as they were rather than half of the new ones.
pub fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A home of `/home/x` with no XDG overrides.
    fn home() -> Dirs {
        Dirs::at(Path::new("/home/x"))
    }

    #[test]
    fn ideas_live_under_xdg_data_home() {
        assert_eq!(
            file(&home().with_data(Path::new("/data")), "niri_tasks"),
            Some(PathBuf::from("/data/niri-tasks/ideas/niri_tasks.md"))
        );
    }

    #[test]
    fn without_xdg_data_home_they_live_under_local_share() {
        assert_eq!(
            file(&home(), "web"),
            Some(PathBuf::from("/home/x/.local/share/niri-tasks/ideas/web.md"))
        );
    }

    #[test]
    fn a_relative_xdg_data_home_is_ignored() {
        assert_eq!(
            file(&home().with_data(Path::new("data")), "web"),
            Some(PathBuf::from("/home/x/.local/share/niri-tasks/ideas/web.md"))
        );
    }

    /// One notepad per workspace: two tags never share a file.
    #[test]
    fn each_tag_has_its_own_file() {
        assert_ne!(file(&home(), "web"), file(&home(), "docs"));
    }

    #[test]
    fn only_a_workspace_tag_names_a_file() {
        for tag in ["", "../escape", "a/b", "Web", "has space", "_web"] {
            assert_eq!(file(&home(), tag), None, "{tag:?}");
        }
    }

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-ideas-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_missing_file_reads_as_no_ideas() {
        let dir = scratch("missing");
        assert_eq!(read(&dir.join("web.md")).unwrap(), "");
    }

    #[test]
    fn written_ideas_read_back_making_their_folder() {
        let dir = scratch("roundtrip");
        let path = dir.join("niri-tasks/ideas/web.md");
        write(&path, "one\ntwo\n").unwrap();
        assert_eq!(read(&path).unwrap(), "one\ntwo\n");
        write(&path, "shorter").unwrap();
        assert_eq!(read(&path).unwrap(), "shorter", "a save replaces the file whole");
        assert!(!path.with_extension("md.tmp").exists(), "no temporary file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
