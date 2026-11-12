import { useEffect, useState } from "react";
import type { Accent, Appearance } from "../api";

export type { Accent, Appearance };

/** The palette's scheme (`palettes/<scheme>.css`). */
export type Scheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

const systemDark = () => window.matchMedia?.(DARK_QUERY).matches ?? false;

/** Automatic follows macOS; Light and Dark are fixed. */
export function schemeFor(appearance: Appearance, darkSystem: boolean): Scheme {
  if (appearance === "system") return darkSystem ? "dark" : "light";
  return appearance;
}

/** Changes a look attribute on <html> at once. WebKit leaves a color with a transition stale when only an
 *  inherited custom property changes, so transitions pause while the window restyles. */
function applyLook(attribute: "scheme" | "accent", value: string) {
  const root = document.documentElement;
  if (root.dataset[attribute] === value) return;
  root.classList.add("restyling");
  root.dataset[attribute] = value;
  // Restyles now, while nothing transitions.
  void root.offsetHeight;
  requestAnimationFrame(() => root.classList.remove("restyling"));
}

/** Sets `data-scheme` (from macOS) and `data-accent` (System) on <html> before the first render, until the
 *  settings load, so the first paint has colors. */
export function applyInitialLook() {
  applyLook("scheme", systemDark() ? "dark" : "light");
  applyLook("accent", "system");
}

/** Keeps the window's scheme in step with `appearance` (on Automatic, with macOS) and its accent with `accent`
 *  (`styles/accent.css`; System follows the macOS accent color). */
export function useAppearance(appearance: Appearance = "system", accent: Accent = "system") {
  const [dark, setDark] = useState(systemDark);
  useEffect(() => {
    const media = window.matchMedia?.(DARK_QUERY);
    if (!media) return;
    const follow = () => setDark(media.matches);
    media.addEventListener("change", follow);
    return () => media.removeEventListener("change", follow);
  }, []);
  useEffect(() => applyLook("scheme", schemeFor(appearance, dark)), [appearance, dark]);
  useEffect(() => applyLook("accent", accent), [accent]);
}
