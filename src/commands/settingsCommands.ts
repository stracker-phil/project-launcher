import { api, toAppError } from "../api";
import { useStore } from "../state/store";
import { isTextInput, type PageCommand } from "./registry";

/** Escape leaves a text field or a sheet first. */
const escapeIsFree = () => !isTextInput(document.activeElement) && !document.querySelector('[role="dialog"]');

function closeWindow() {
  api.runCommand("window:close").catch((e: unknown) => useStore.getState().toast("error", `Could not close the window: ${toAppError(e).message}`));
}

/** The Settings window's own keys; ⌘W is the File menu's Close Window. */
export const SETTINGS_COMMANDS: PageCommand[] = [
  { id: "settings.close", title: "Close Settings", defaultBinding: "Escape", when: escapeIsFree, run: closeWindow },
  {
    id: "settings.search",
    title: "Search Settings",
    defaultBinding: "Mod+F",
    run: () => document.querySelector<HTMLInputElement>('input[aria-label="Search Settings"]')?.focus(),
  },
];
