# 0013. JSON stores repair themselves on load

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `json-stores-repair-on-load`); reference implementation: Daybook ADR 0013.

## Context

Launcher keeps its project registry, settings, tags and UI state as JSON in the config dir (ADR 0012). The
owner may edit these files by hand, an older build may have written them, and a crash may leave a half-written
file. None of that may stop the app from starting or silently lose the project list.

## Decision

Every app-owned JSON file goes through `JsonStore` over `JsonFile` (`src-tauri/src/store/`): atomic writes
(temp file + rename), mode 0600 in a 0700 dir, a missing file loads defaults, unknown keys are ignored, and an
unparsable file is moved aside as `<name>.<timestamp>.corrupt` before defaults load. `Settings` is one typed
camelCase model exported through ts-rs and repaired on every load and save (`Settings::repaired`: unknown
values fall back to defaults field by field). The project registry (`projects.json`) is repaired per entry: an
entry with an unusable path or id is dropped and logged, the rest load. Optional features keep their settings
in `<config>/<feature>.json`.

The per-project `.project-launcher.json` files are owned by the owner, not the app: they are never moved
aside or rewritten with defaults; a parse error is shown on the project (`configError`) and the file is left
alone.

- Rejected: failing on a bad file (one typo locks the owner out); keeping the prototype's untyped normalize
  functions in the page (the rule belongs in Rust, ADR 0001); one big file for all state (a bad feature value
  would quarantine core settings).

## Consequences

- Hand edits and old files never break a launch; the worst case is defaults plus a `.corrupt` file to inspect.
- Every new setting needs a default and a repair rule in `Settings::repaired`, with a test.
