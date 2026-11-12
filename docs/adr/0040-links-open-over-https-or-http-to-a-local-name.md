# 0040. Links open over https, or over http to a local name

## Status

Accepted (2026-11-12). Supersedes in part [0016](0016-launcher-has-no-network-code.md) ("`open_url`, https
only") and [0018](0018-outside-content-is-shown-as-text-and-run-only-by-id.md) ("https (or http to
localhost)").

## Context

Links come from project files, which may arrive with a cloned repository: a project's website, its repository
and URL actions. Until now `open_url` let any `http` link through, and URL actions handed their value to
`open` unchecked, so a project file could open a `file://` app or pass a flag to `open`. ADRs 0016 and 0018
said https only, or http to localhost. Local servers often speak plain http on other names, too: a container
such as an automation server reached through a name the owner mapped in `/etc/hosts`, a `.test` or
`.ddev.site` site.

## Decision

Every link that leaves the app goes through `links::is_web_url`: `open_url` (Website, Repository) and URL
actions alike. It allows:

- https to any host;
- http to a local name: `localhost`, a loopback, private or link-local address, a name without a dot, a name
  ending in `.localhost`, `.local`, `.test`, `.internal`, `.lan`, `.home.arpa` or `.ddev.site`, and any name
  listed in `/etc/hosts` (read on each click).

Everything else is refused with "Not a web link". Links open in the browser; Launcher itself still makes no
connection (ADR 0016).

- Rejected: https only (breaks local containers that only speak http); http anywhere (a project file from
  elsewhere could send the owner to a plain-http site that looks like the project's); custom URL schemes
  (`file://`, app schemes) for URL actions (a cloned project file could open an app; an app action covers the
  legitimate case).

## Consequences

- A URL action with a custom scheme or an unfilled `${url}` shows an error instead of opening.
- A new hand-mapped name works at once; nothing to configure in Launcher.
