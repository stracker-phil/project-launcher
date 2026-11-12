# 0047. The CLI output siblings read is a contract

## Status

Accepted (2026-11-12). Owner's decision. Extends [0011](0011-siblings-talk-through-their-clis.md).

## Context

Daybook calls `launcher list --json` and `launcher open <id>` (ADR 0011). A renamed or dropped field, or a
changed argument, breaks Daybook without a failing test in either app; only the text table had tests.

## Decision

- What a sibling reads is a contract: `launcher list --json` entries `{ id, name, path, tag, starred,
  archived }` and `launcher open <id>`. Tests pin the field names, their JSON types and the argument.
- Adding a field is compatible. Renaming, removing or retyping a field, or changing an argument, needs the
  owner's approval first; the step's report names it, with what the sibling has to change.
- Rejected: versioned output (`--json=v2`): one reader, owned by the same owner, needs no versions; relying on
  the report alone (nothing failed when the shape changed).

## Consequences

- The contract tests (`src-tauri/src/cli/tests.rs`) compare the whole output, so every change, an added field
  too, updates them on purpose; only the breaking ones need the owner's approval, which `ship-step` asks for.
