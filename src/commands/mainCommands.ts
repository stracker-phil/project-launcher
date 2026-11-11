import { useStore } from "../state/store";
import type { PageCommand } from "./registry";

const togglePalette = () => {
  const { paletteOpen, setPaletteOpen } = useStore.getState();
  setPaletteOpen(!paletteOpen);
};

/** The main window's commands. Their keys are the View menu's accelerators, so they carry no binding here. */
export const MAIN_COMMANDS: PageCommand[] = [
  { id: "view.toggleSidebar", title: "Toggle Sidebar", run: () => useStore.getState().toggleSidebar() },
  { id: "view.toggleInspector", title: "Toggle Inspector", run: () => useStore.getState().toggleInspector() },
  // ⌘K closes it again (ADR 0037).
  { id: "palette.open", title: "Command Palette", run: togglePalette },
];
