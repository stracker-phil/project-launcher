import { useEffect, useState } from "react";

/** The Appearance setting (ADR 0026); Rust stores it in the settings. */
export type Appearance = "system" | "light" | "dark";

/** The palette's scheme (`palettes/<scheme>.css`). */
export type Scheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

const systemDark = () => window.matchMedia?.(DARK_QUERY).matches ?? false;

/** Automatic follows macOS; Light and Dark are fixed. */
export function schemeFor(appearance: Appearance, darkSystem: boolean): Scheme {
  if (appearance === "system") return darkSystem ? "dark" : "light";
  return appearance;
}

/** Changes the scheme at once. WebKit leaves a color with a transition stale when only an inherited custom
 *  property changes, so transitions pause while the window restyles. */
function applyScheme(scheme: Scheme) {
  const root = document.documentElement;
  if (root.dataset.scheme === scheme) return;
  root.classList.add("restyling");
  root.dataset.scheme = scheme;
  // Restyles now, while nothing transitions.
  void root.offsetHeight;
  requestAnimationFrame(() => root.classList.remove("restyling"));
}

/** Sets `data-scheme` on <html> before the first render (from macOS, until the settings load), so the first
 *  paint has colors. */
export function applyInitialLook() {
  applyScheme(systemDark() ? "dark" : "light");
}

/** Keeps the window's scheme in step with `appearance` (on Automatic, with macOS). */
export function useAppearance(appearance: Appearance = "system") {
  const [dark, setDark] = useState(systemDark);
  useEffect(() => {
    const media = window.matchMedia?.(DARK_QUERY);
    if (!media) return;
    const follow = () => setDark(media.matches);
    media.addEventListener("change", follow);
    return () => media.removeEventListener("change", follow);
  }, []);
  useEffect(() => applyScheme(schemeFor(appearance, dark)), [appearance, dark]);
}
