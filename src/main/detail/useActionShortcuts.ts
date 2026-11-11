import { useEffect } from "react";
import type { Action, Project } from "../../api";
import { bindingFromEvent, isTextInput, normalizeBinding } from "../../commands/registry";
import { useStore } from "../../state/store";
import { launchAction } from "../launch";

/** Launches an action by its key while the main window has the keys. */
export function useActionShortcuts(project: Project | null, actions: Action[]) {
  useEffect(() => {
    if (!project) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.repeat) return;
      const { menu, confirmation } = useStore.getState();
      if (menu || confirmation) return;
      if (isTextInput(e.target)) return;
      if (e.target instanceof Element && e.target.closest(".popover, [data-recording]")) return;
      const pressed = normalizeBinding(bindingFromEvent(e) ?? "");
      if (!pressed) return;
      const action = actions.find((a) => a.shortcut && normalizeBinding(a.shortcut) === pressed);
      if (!action) return;
      e.preventDefault();
      void launchAction(project, action);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [project, actions]);
}
