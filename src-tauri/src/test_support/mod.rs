//! Helpers shared by unit tests and integration tests through the `test-util` feature; the demo seed and
//! the fakes for outside sources also ship in debug builds (ADR 0021, ADR 0022).

#[cfg(any(test, feature = "test-util"))]
mod fixtures;

#[cfg(any(test, feature = "test-util"))]
pub use fixtures::*;

/// The demo seed; debug builds write it on their first launch and on `launcher dev seed --yes`.
pub mod demo;
