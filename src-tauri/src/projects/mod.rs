//! Projects: the registry of known project files (`projects.json`), each project's own
//! `.project-launcher.json`, and the `Project` the page shows (ADR 0038).

pub mod config;
pub mod file;
pub mod registry;
pub mod view;

pub use file::{FILE_NAME, FileKey, FileRead, MetaKey};
pub use registry::{KnownProject, Registry};
pub use view::{Project, ProjectMeta};
