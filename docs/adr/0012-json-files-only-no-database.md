# 0012. Launcher stores its data in JSON files only, without a database

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `storage`, option JSON files only);
reference implementation: Daybook ADR 0012.

## Context

Launcher keeps a registry of tens to a few hundred projects with launcher-only metadata (name, icon, color,
tag, notes, starred, archived, last opened), plus settings and tags. Launch configuration stays in each
project's `.project-launcher.json` (ADR 0007). Git, manifest and file statistics are computed on demand and
cached in memory.

## Decision

App data lives in `~/Library/Application Support/<identifier>/`, files 0600 in a 0700 directory:
`projects.json` (registry and metadata) and `settings.json` (settings, tags), each through `JsonStore`. Search
and filtering run in Rust over the loaded registry. No database until rows or queries demand one; that change
gets its own record.

- Rejected: SQLite through rusqlite (migrations and a second storage technology for a list this small).
- Rejected: SQLite through sqlx (async pool for no side process; Launcher has no MCP reader).

## Consequences

- The data is readable and repairable by hand; backups are file copies.
- The prototype's `projects.json` can be imported almost as is.
- Full-text search across many projects or large histories would need a new record.
