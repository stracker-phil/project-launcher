#!/bin/zsh
# Signs a built Launcher bundle so macOS keeps its grants across rebuilds (ADR 0019). Uses
# $LAUNCHER_SIGN_IDENTITY, else a valid "Apple Development" certificate (Xcode → Settings → Accounts →
# Manage Certificates; a free Apple ID is enough), else the "Launcher Local Signing" certificate of
# scripts/create-signing-cert.sh, else signs ad hoc (the grant then lasts one build).
# Only the Apple certificate carries a Team ID, so the Keychain trusts every build of the team; with the local
# certificate, each new build asks once for the login password before it reads its token.
# Signs with the hardened runtime: no injected libraries or debugger attach. src-tauri/entitlements.plist, when
# present, grants what the runtime would block (AppleScript, Calendars; ADR 0017).
# Usage: scripts/sign-app.sh path/to/App.app bundle.identifier
set -e
APP=$1 IDENTIFIER=$2
IDENTITY=${LAUNCHER_SIGN_IDENTITY:-}
if [[ -z "$IDENTITY" ]]; then
  # By hash: an expired certificate of the same name would make the name ambiguous.
  IDENTITY=$(security find-identity -v -p codesigning | awk '/"Apple Development: / { print $2; exit }')
fi
if [[ -z "$IDENTITY" ]] && security find-identity -p codesigning | grep -q '"Launcher Local Signing"'; then
  IDENTITY="Launcher Local Signing"
fi
ENTITLEMENTS="${0:A:h:h}/src-tauri/entitlements.plist"
EXTRA=()
[[ -f "$ENTITLEMENTS" ]] && EXTRA=(--entitlements "$ENTITLEMENTS")
codesign --force --options runtime "${EXTRA[@]}" --sign "${IDENTITY:--}" --identifier "$IDENTIFIER" "$APP" 2>/dev/null
[[ -n "$IDENTITY" ]] || echo "Signed ad hoc: grant Accessibility again after each rebuild (see scripts/sign-app.sh)"
