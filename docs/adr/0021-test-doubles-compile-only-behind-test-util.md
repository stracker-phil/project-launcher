# 0021. Test doubles compile only behind test-util

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `test-doubles-behind-test-util`); reference implementation: Taskboard ADR 0006.

## Context

Unit tests, integration tests in `src-tauri/tests/` and the debug build all need fakes: a git runner, Docker
and DDEV probes, an app launcher that records instead of opening, a DataBee stand-in, demo project folders.
The release build must ship none of them.

## Decision

Test doubles, fixtures and the demo seed live in `src-tauri/src/test_support/` and compile under
`cfg(any(test, feature = "test-util", debug_assertions))`. The crate lists itself as a dev-dependency with the
`test-util` feature, so integration tests see them. Production code reaches outside tools through small traits
only where a fake exists today (ADR 0006).

- Rejected: compiling the doubles unconditionally (they would ship); a separate test-support crate (more
  plumbing for no gain).

## Consequences

- Release binaries contain no fake launchers or demo data.
- A double used only by tests in one module may stay in that module's `#[cfg(test)]` block.
