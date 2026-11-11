# 0039. A major version needs the owner's approval

## Status

Accepted (2026-11-11). Supersedes in part [0020](0020-every-release-build-gets-a-new-version.md) (the bump rule).
Inherited from the family catalog (release-versions; reference: TimeTracker ADR 0047).

## Context

ADR 0020 made a change that breaks existing use (the CLI and its `--json` shape, links, stored data without
migration, a removed feature) a major bump. Launcher is a personal tool; its only outside consumer is
Daybook, which the owner also maintains. Strict semver majors signal breakage to consumers it doesn't have.

## Decision

- Changes that break existing use are **minor**, like new features and visible behaviour.
- A **major** bump needs the owner's explicit approval, asked before releasing; never chosen from the log alone.
- Patch stays: fixes, polish, performance and docs only.
- A changed `launcher list --json` shape is named in the step's report, so Daybook can follow.
- Rejected: semver's strict meaning for breaking changes.

## Consequences

- `ship-step` asks before a major release; `scripts/release.sh` is unchanged.
