//! Shared plumbing for every VIctory crate and tool.
//!
//! * [`config`]: where the user's files live (never inside the repository).
//! * [`policy`]: the runtime half of rule #1, refusing to write
//!   game-derived data into the source tree.
//! * [`logging`]: one tracing setup for all binaries.
//! * [`wine`]: whether we run under Wine/Proton (`VICTORY_PROTON`).

pub mod config;
pub mod logging;
pub mod policy;
pub mod wine;

pub use config::Config;
pub use policy::{PolicyError, ensure_outside_repo};
