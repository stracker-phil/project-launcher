# 0019. Builds are signed with an Apple Development certificate

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `signing`); reference implementation: Taskboard ADR 0053.

## Context

macOS ties privacy grants (Automation for AppleScript, ADR 0017) and Keychain "Always Allow" to the code
signature. With ad hoc or self-signed signatures every rebuild looks like a new app and asks again.

## Decision

`scripts/sign-app.sh` signs with `$LAUNCHER_SIGN_IDENTITY` if set, else the first **valid** "Apple
Development" identity picked by its SHA-1 hash (an expired one with the same name makes the name ambiguous),
else "Launcher Local Signing" created by `scripts/create-signing-cert.sh` (which also runs
`security set-key-partition-list` so codesign stops prompting), else ad hoc. Tauri builds with
`signingIdentity: "-"`; install and launch re-sign. The running app quits before its files are re-signed.

- Rejected: a local certificate only (trusted per build hash only); ad hoc only; Developer ID plus
  notarization (not needed for a personal tool).

## Consequences

- The Team ID keeps Automation grants across rebuilds.
- The first build with a new identity resets privacy grants; the owner grants Automation once more.
