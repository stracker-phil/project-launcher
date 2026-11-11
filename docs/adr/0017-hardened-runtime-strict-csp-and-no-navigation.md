# 0017. Hardened runtime, strict CSP and no navigation

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `hardened-runtime`); reference implementation: Taskboard ADR 0049.

## Context

Launcher's webview shows text from folders the owner cloned (file names, commit subjects, config values) and
can call commands that start processes. It also drives other apps (terminal, editor, git client) through
AppleScript where a CLI isn't enough, which the hardened runtime blocks without an entitlement.

## Decision

`scripts/sign-app.sh` signs with `--options runtime` and `src-tauri/entitlements.plist`, which holds only
`com.apple.security.automation.apple-events`. The CSP in `tauri.conf.json` is strict: `script-src 'self'`,
`frame-src 'none'`, `object-src 'none'`, `base-uri 'none'`, `form-action 'none'`, `style-src-elem 'self'` in
`csp` only, `img-src 'self' data:`. A navigation-guard plugin on every webview (`window/navigation.rs`:
`guard()`, `is_app_page`, tested) keeps each window on its app page; links go through `open_url`. AppleScript
failures degrade gracefully: the target app is activated instead and the error names where to grant Automation.

- Rejected: App Sandbox (breaks the socket, sibling CLIs and starting the owner's tools); notarization (not
  needed for a personal tool); CSP alone without the navigation guard.

## Consequences

- A missing entitlement fails silently (AppleScript error -1743): every AppleScript path is tested in the
  **release** build, not only debug.
- A new entitlement (e.g. for calendars) needs an ADR and a line in `entitlements.plist`.
