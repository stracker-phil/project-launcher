//! The launch config in a project's `.project-launcher.json` (apps, scripts, env, states, dev services,
//! database, locked), in the prototype's format: its vocabulary (`model`), placeholders (`vars`), the
//! resolver the page and the launchers read (`resolve`, with the built-ins' targets in `builtins`) and the
//! action editor's changes (`edit`).

mod builtins;
pub mod claude;
pub mod edit;
pub mod model;
pub mod resolve;
pub mod vars;

pub use model::{ActionDraft, ActionList, ActionSource, Builtin, DevMode, RunsAs};
pub use resolve::{Context, ProjectConfig, resolve};
