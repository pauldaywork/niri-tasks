//! Where things are on this machine: the home folder, the XDG data and cache
//! folders, and `~/Projects`. The one module that reads `HOME`, so a path
//! rule lives here once and every test can point it at a scratch folder.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

/// The folders this process works from. Build one with [`Dirs::from_env`]
/// where a command needs paths, or [`Dirs::at`] in a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    home: PathBuf,
    /// `XDG_DATA_HOME`, when set and absolute.
    data: Option<PathBuf>,
    /// `XDG_CACHE_HOME`, when set and absolute.
    cache: Option<PathBuf>,
}

impl Dirs {
    /// From this process's environment. `HOME` is required; the XDG folders
    /// are optional, and a relative one is ignored, as the XDG spec says.
    pub fn from_env() -> Result<Dirs> {
        let home = std::env::var_os("HOME").context("HOME is unset")?;
        Ok(Dirs {
            home: PathBuf::from(home),
            data: xdg("XDG_DATA_HOME"),
            cache: xdg("XDG_CACHE_HOME"),
        })
    }

    /// A home with no XDG overrides: what tests build on a scratch folder.
    pub fn at(home: &Path) -> Dirs {
        Dirs { home: home.to_path_buf(), data: None, cache: None }
    }

    /// With `XDG_DATA_HOME` set to `p`. A relative `p` is ignored, as it is
    /// when it comes from the environment.
    pub fn with_data(mut self, p: &Path) -> Dirs {
        self.data = p.is_absolute().then(|| p.to_path_buf());
        self
    }

    /// With `XDG_CACHE_HOME` set to `p`. A relative `p` is ignored, as it is
    /// when it comes from the environment.
    pub fn with_cache(mut self, p: &Path) -> Dirs {
        self.cache = p.is_absolute().then(|| p.to_path_buf());
        self
    }

    /// The home folder, as `HOME` gave it.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// `~/Projects`: one folder per project, each the name of its workspace.
    pub fn projects(&self) -> PathBuf {
        self.home.join("Projects")
    }

    /// Where this tool keeps data: `$XDG_DATA_HOME`, else `~/.local/share`.
    pub fn data(&self) -> PathBuf {
        self.data.clone().unwrap_or_else(|| self.home.join(".local/share"))
    }

    /// Where this tool keeps caches: `$XDG_CACHE_HOME`, else `~/.cache`.
    pub fn cache(&self) -> PathBuf {
        self.cache.clone().unwrap_or_else(|| self.home.join(".cache"))
    }

    /// Where a workspace's terminals start.
    ///
    /// `~/Projects/<workspace>` -> `~/Projects` -> `~`. Uses the *raw* workspace
    /// name, not the sanitised one. Always an existing directory, which
    /// `ghostty +new-window` needs: it resolves the path before asking the
    /// running ghostty for a window, and fails outright on one that is not there.
    pub fn start_dir(&self, workspace_raw: &str) -> PathBuf {
        let projects = self.projects();
        let in_project = projects.join(workspace_raw);
        if !workspace_raw.is_empty() && in_project.is_dir() {
            return in_project;
        }
        if projects.is_dir() {
            return projects;
        }
        self.home.clone()
    }

    /// The project folder `cwd` is inside, if it is inside one: the first path
    /// component under `~/Projects`. [`Dirs::start_dir`] run backwards.
    pub fn project_from_cwd(&self, cwd: &Path) -> Option<String> {
        let rest = cwd.strip_prefix(self.projects()).ok()?;
        match rest.components().next()? {
            Component::Normal(name) => name.to_str().map(str::to_string),
            _ => None,
        }
    }
}

/// An XDG folder variable, when set and absolute.
fn xdg(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_and_cache_prefer_absolute_xdg_and_fall_back_to_home() {
        let home = Path::new("/home/x");
        assert_eq!(Dirs::at(home).data(), PathBuf::from("/home/x/.local/share"));
        assert_eq!(Dirs::at(home).cache(), PathBuf::from("/home/x/.cache"));

        assert_eq!(Dirs::at(home).with_data(Path::new("/data")).data(), PathBuf::from("/data"));
        assert_eq!(Dirs::at(home).with_cache(Path::new("/cache")).cache(), PathBuf::from("/cache"));

        // The XDG spec says a relative path is to be ignored.
        assert_eq!(
            Dirs::at(home).with_data(Path::new("rel")).data(),
            PathBuf::from("/home/x/.local/share")
        );
        assert_eq!(Dirs::at(home).with_cache(Path::new("rel")).cache(), PathBuf::from("/home/x/.cache"));
    }

    #[test]
    fn start_dir_falls_back_through_the_chain() {
        let tmp = std::env::temp_dir().join(format!("niritasks-dirs-test-{}", std::process::id()));
        let home = tmp.join("home");
        let projects = home.join("Projects");
        std::fs::create_dir_all(projects.join("alpha")).unwrap();
        std::fs::create_dir_all(projects.join("my project")).unwrap();
        let dirs = Dirs::at(&home);

        // Exact project folder wins, by its raw name.
        assert_eq!(dirs.start_dir("alpha"), projects.join("alpha"));
        assert_eq!(dirs.start_dir("my project"), projects.join("my project"));
        // Unknown project, or an unnamed workspace, falls back to ~/Projects.
        assert_eq!(dirs.start_dir("nope"), projects);
        assert_eq!(dirs.start_dir(""), projects);

        // Without ~/Projects at all, fall back to home.
        let bare = tmp.join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        assert_eq!(Dirs::at(&bare).start_dir("anything"), bare);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn project_from_cwd_is_the_folder_under_projects() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(
            dirs.project_from_cwd(Path::new("/home/x/Projects/alpha")),
            Some("alpha".to_string())
        );
        assert_eq!(
            dirs.project_from_cwd(Path::new("/home/x/Projects/alpha/src/deep")),
            Some("alpha".to_string())
        );
        assert_eq!(
            dirs.project_from_cwd(Path::new("/home/x/Projects/my project")),
            Some("my project".to_string())
        );
        assert_eq!(dirs.project_from_cwd(Path::new("/home/x/Projects")), None);
        assert_eq!(dirs.project_from_cwd(Path::new("/home/x")), None);
        assert_eq!(dirs.project_from_cwd(Path::new("/tmp/alpha")), None);
    }
}
