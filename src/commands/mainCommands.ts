import { useStore } from "../state/store";
import type { PageCommand } from "./registry";

/** The main window's commands. Their keys are the View menu's accelerators, so they carry no binding here. */
export const MAIN_COMMANDS: PageCommand[] = [
  { id: "view.toggleSidebar", title: "Toggle Sidebar", run: () => useStore.getState().toggleSidebar() },
  { id: "view.toggleInspector", title: "Toggle Inspector", run: () => useStore.getState().toggleInspector() },
];
