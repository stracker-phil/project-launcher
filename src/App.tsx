import { useCallback } from "react";
import { type LaunchEvent } from "./api";
import { useMainCommands } from "./commands";
import { ConfirmHost } from "./components/ConfirmSheet";
import { Toasts } from "./components/Toasts";
import { TooltipHost } from "./components/TooltipHost";
import { useAppearance } from "./lib/theme";
import { useLaunchEvents } from "./lib/useLaunchEvents";
import { Shell } from "./shell/Shell";
import { useStore } from "./state/store";

/** The main window: the panes, and the overlays every window has. */
export default function App() {
  useAppearance();
  useMainCommands();
  const onLaunch = useCallback((event: LaunchEvent) => {
    if (event.type === "error") useStore.getState().toast("error", event.message);
  }, []);
  useLaunchEvents(onLaunch);
  return (
    <>
      <Shell />
      <Toasts />
      <ConfirmHost />
      <TooltipHost />
    </>
  );
}
