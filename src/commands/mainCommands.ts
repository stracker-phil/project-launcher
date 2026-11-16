import { useStore } from "../state/store";
import { closeWindow } from "./closeWindow";
import { isTextInput, layerIsOpen, type PageCommand } from "./registry";

/** Escape leaves a text field, a sheet or a menu first; the empty project search lets it through. */
function escapeIsFree(): boolean {
  const focused = document.activeElement;
  const emptySearch = focused instanceof HTMLInputElement && focused.type === "search" && focused.value === "";
  return (emptySearch || !isTextInput(focused)) && !layerIsOpen();
}

/** The main window's commands. The View toggles take their keys from the View menu, so they carry no binding here.
 * ⌘Q hides rather than quits (ADR 0048). */
export const MAIN_COMMANDS: PageCommand[] = [
  { id: "view.toggleSidebar", title: "Toggle Sidebar", run: () => useStore.getState().toggleSidebar() },
  { id: "view.toggleInspector", title: "Toggle Inspector", run: () => useStore.getState().toggleInspector() },
  { id: "main.close", title: "Close Window", defaultBinding: "Escape", when: escapeIsFree, run: closeWindow },
  { id: "main.hide", title: "Hide Window", defaultBinding: "Mod+Q", run: closeWindow },
];
