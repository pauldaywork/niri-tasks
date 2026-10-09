//! niritasks — workspace-scoped Taskwarrior for niri.
//!
//! The logic lives here rather than in `main.rs` so it can be exercised by
//! integration tests.

pub mod actions;
pub mod daemon;
pub mod dirs;
pub mod github;
pub mod ideas;
pub mod ipc;
pub mod link;
pub mod niri;
pub mod notify;
pub mod panel;
pub mod project;
pub mod refine;
pub mod session;
pub mod speak;
pub mod tag;
pub mod taskbox;
pub mod task;
pub mod text;
pub mod work;
pub mod workspace;
