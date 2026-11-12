# 0041. Debug builds record DataBee like every other launch

## Status

Accepted (2026-11-12). Supersedes in part [0011](0011-siblings-talk-through-their-clis.md) (the debug build's
`databee-debug` or stand-in script, `LAUNCHER_REAL_DATABEE`, `src-tauri/src/databee/`) and
[0022](0022-the-debug-build-is-a-separate-app-on-demo-data.md) (DataBee as a stand-in script,
`_REAL_DATABEE`).

## Context

ADRs 0011 and 0022 planned a DataBee stand-in script with its own switch. The code went another way: DataBee
is opened like an editor or a terminal, as a process through the `System` port, so the debug build's
`Recorder` already catches it. A separate script and switch would fake the same thing twice.

## Decision

The debug build opens DataBee through the `System` port like every other launch (`devenv/database.rs`
builds the `databee` command; `run/system.rs` runs it). Its `Recorder` records the command and shows it in a
toast instead of starting DataBee; `LAUNCHER_REAL_LAUNCH=1` makes every launch real, DataBee included. The
password still travels in `DATABEE_PASSWORD`, named by `--password-env`, never in argv.

- Rejected: a stand-in `databee` script (a second fake for what the `Recorder` covers); a switch per tool (one
  switch for every outside launch is enough for checking the real path).

## Consequences

- Checking the real DataBee path from a debug build also opens real editors and terminals; that is the point
  of the switch.
- `docs/testing.md` lists the switches: `LAUNCHER_REAL_LAUNCH` and `LAUNCHER_REAL_DOCKER`.
