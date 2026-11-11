import { useCallback } from "react";
import { api, type LaunchEvent } from "./api";
import { useLaunchEvents } from "./lib/useLaunchEvents";

/** The main window. Until the shell arrives it is an empty window whose top strip drags it. */
export default function App() {
  const onLaunch = useCallback((event: LaunchEvent) => {
    if (event.type === "error") void api.logError(event.message);
  }, []);
  useLaunchEvents(onLaunch);
  return <div data-tauri-drag-region style={{ height: 52 }} aria-label="Launcher" />;
}
