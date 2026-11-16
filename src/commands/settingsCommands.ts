import { closeWindow } from "./closeWindow";
import { isTextInput, layerIsOpen, type PageCommand } from "./registry";

/** Escape leaves a text field or an open layer first. */
const escapeIsFree = () => !isTextInput(document.activeElement) && !layerIsOpen();

/** The Settings window's own keys; ⌘W is the File menu's Close Window, ⌘Q hides rather than quits (ADR 0048). */
export const SETTINGS_COMMANDS: PageCommand[] = [
  { id: "settings.close", title: "Close Settings", defaultBinding: "Escape", when: escapeIsFree, run: closeWindow },
  { id: "settings.hide", title: "Hide Settings", defaultBinding: "Mod+Q", run: closeWindow },
  {
    id: "settings.search",
    title: "Search Settings",
    defaultBinding: "Mod+F",
    run: () => {
      const field = document.querySelector<HTMLInputElement>('input[aria-label="Search Settings"]');
      field?.focus();
      field?.select();
    },
  },
];
