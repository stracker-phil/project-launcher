# 0018. Outside content is shown as text and actions run only by id

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `untrusted-content`); reference implementation: TimeTracker ADR 0033 and Taskboard ADR 0027.

## Context

Much of what Launcher shows comes from places anyone can write to: file names and commit subjects of
repositories cloned from elsewhere, values in a `.project-launcher.json` that came with a repository, and the
output of scripts. Launcher also starts apps and runs shell commands taken from those config files.

## Decision

- Outside text is rendered as text (React escaping); no outside HTML is rendered, no remote images are loaded.
- The webview never sends a command line or a path to execute. It sends ids: `project:<id>` plus an action
  or script index; Rust resolves the command from the registered project's config and runs it.
- Only projects the owner registered run anything. A newly registered project whose config defines scripts
  or commands shows them, but runs them only on an explicit owner action; nothing runs on registration,
  selection or file change.
- Interpolated values go through shell quoting in Rust (`shell::quote`); file paths opened from the inspector
  must stay inside the project folder.
- Links from config values open in the browser only when they are https (or http to localhost).

- Rejected: passing command strings from the page (any page bug becomes code execution); sanitizing HTML from
  config values (nothing needs rich text).

## Consequences

- A malicious config can still define a harmful script, but it never runs without the owner clicking it.
- Every new action type gets an id-based command and a test that the page can't pass a raw command.
