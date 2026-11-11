# Testing and dev tooling

## Commands

Coding agents don't run these themselves. They hand test writing to the `write-ts-unit-test` and
`write-rust-unit-test` agents and runs to the `ci` agent (`.claude/agents/`), so test code and test output
stay out of the main context ([`.claude/rules/testing.md`](../.claude/rules/testing.md)).

| Command | Runs |
|---|---|
| `bun run test` | Vitest UI tests (`test:ui`), then the Rust tests (`test:rust`) |
| `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` | the slow `#[ignore]` tests on temp folders or a temp database; the `ci` agent's integration suite |
| `bun run lint` | regenerates the IPC types and fails when `src/api/types.gen.ts` changed, then `tsc --noEmit`, `eslint .` and `cargo clippy --lib --bins -- -D warnings` (production code only) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | Rust formatting (run before lint) |

Unit tests and lint never run at the same time: both regenerate `src/api/types.gen.ts`.

## Agents

| Agent | Does |
|---|---|
| `write-rust-unit-test` | writes Rust unit tests (`mod tests`) and integration tests; never runs them |
| `write-ts-unit-test` | writes Vitest tests next to components, mocking only `src/api`; never runs them |
| `ci` | runs one suite and replies with a pass/fail summary |
| `review-code-cleanliness` | reviews a changed file for structure, comment noise, boundaries and over-engineering |

The `ship-step` skill chains cleanup, review, `cargo fmt`, `ci`, build, launch and commit at the end of every
step.

## Rust

- **Unit tests** sit next to the code in `#[cfg(test)] mod tests`, named as behavior sentences, with exact
  and table-driven assertions.
- **Helpers** live in `src-tauri/src/test_support/`: `temp_config_dir`, builders, fakes and the demo seed,
  compiled under `cfg(any(test, feature = "test-util", debug_assertions))` ([ADR 0021](adr/0021-test-doubles-compile-only-behind-test-util.md)).
- **Contracts:** `lib.rs` tests compare `APP_NAME`, `IDENTIFIER` and `LINK_SCHEME` with both Tauri configs and
  keep the version equal in `Cargo.toml`, `tauri.conf.json` and `package.json`; each `#[ts(export)]` type has
  an `export_bindings_*` test (`bun run types`).
- Performance tests (`tests/performance.rs`, ignored) arrive once there is data worth timing (search over a
  large registry, the stats scan).

## Frontend

- `*.test.ts(x)` next to their code; `src/test-setup.ts` stubs `ResizeObserver`, `matchMedia` and the
  clipboard, and pretends to be a Mac (`Mod` is ⌘).
- Only `src/api` is mocked, with `createApiMock()` (`src/test/apiMock.ts`); fixtures in `src/test/`.
- Testing Library queries by role or text, `userEvent` for interactions.
- Guards: `src/test/boundary.test.ts` (Tauri imports only in `api.ts` files), `src/styles/colors.test.ts`
  (no color literal outside the palettes, matching palettes), `src/styles/contrast.test.ts`.

## Demo data

The debug app, the tests and the README screenshots share one made-up data set in
`src-tauri/src/test_support/` ([ADR 0022](adr/0022-the-debug-build-is-a-separate-app-on-demo-data.md), [`demo-data.md`](../.claude/rules/demo-data.md)).
The debug app seeds it on its first launch with an empty data folder; `launcher dev seed --yes` (the debug
binary) starts over. Every outside source has a fake by default: git, Docker/DDEV/wp-env, the editor, terminal and app
launchers (recorded, not opened) and DataBee (a stand-in for `databee-debug`). `LAUNCHER_REAL_LAUNCH`, `LAUNCHER_REAL_DOCKER` and `LAUNCHER_REAL_DATABEE` (`=1`)
switches one back to the real source for checking that code path.

## Driving the app

The dev scripts work without stealing focus. They are macOS-only (zsh, Swift, `screencapture`, `python3`).

| Script | Does |
|---|---|
| `bun run build:app` | builds `src-tauri/target/debug/bundle/macos/Launcher (Debug).app` (`tauri.debug.conf.json`, `VITE_DEV_HOOKS=1`) |
| `scripts/dev/launch.sh [args]` | signs and restarts the debug app next to the terminal without focus; refuses a bundle older than the code |
| `scripts/dev/js.sh '<js>'` | runs JavaScript in the debug app over its socket (`@<label>` on the first line targets another window) |
| `scripts/dev/screenshot.sh <png> [--overlay]` | captures the debug window, also on another Space; never the release app |
| `scripts/dev/e2e.sh` | clicks controls twice and fails when the panel, toolbar, first row or the control moved |

## Signing

`scripts/sign-app.sh` signs with `LAUNCHER_SIGN_IDENTITY`, else an Apple Development certificate, else the
`Launcher Local Signing` certificate from `scripts/create-signing-cert.sh`, else ad hoc ([ADR 0019](adr/0019-builds-are-signed-with-an-apple-development-certificate.md)), with
the hardened runtime ([ADR 0017](adr/0017-hardened-runtime-strict-csp-and-no-navigation.md)).
