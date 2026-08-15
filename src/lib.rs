//! wt — workspace-scoped Taskwarrior for niri.
//!
//! The logic lives here rather than in `main.rs` so it can be exercised by
//! integration tests — in particular `tests/differential.rs`, which checks the
//! pure functions against the shell pipelines they were ported from.

pub mod project;
pub mod session;
pub mod tag;
pub mod text;
