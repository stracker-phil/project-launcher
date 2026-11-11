import { useEffect } from "react";
import { resolveKeydown } from "./registry";

/** A context menu takes its own keys. */
const inMenu = (target: EventTarget | null) => target instanceof Element && !!target.closest('[role="menu"]');

/** ↵ and Space activate the focused button, link or checkbox rather than a command. */
function activatesControl(e: KeyboardEvent): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey || (e.key !== "Enter" && e.key !== " ")) return false;
  return e.target instanceof Element && !!e.target.closest('button, a[href], input[type="checkbox"], [role="button"], [role="option"]');
}

function handleKeydown(e: KeyboardEvent) {
  if (activatesControl(e) || inMenu(e.target)) return;
  const command = resolveKeydown(e);
  if (!command) return;
  e.preventDefault();
  e.stopPropagation();
  // A held key is swallowed rather than repeated.
  if (e.repeat) return;
  void command.run();
}

/** Routes the window's keys to the page commands while mounted (capture phase). */
export function useShortcuts() {
  useEffect(() => {
    window.addEventListener("keydown", handleKeydown, true);
    return () => window.removeEventListener("keydown", handleKeydown, true);
  }, []);
}
