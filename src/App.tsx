import { useCallback } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { type LaunchEvent } from "./api";
import { useMainCommands } from "./commands";
import { ConfirmHost } from "./components/ConfirmSheet";
import { MenuHost } from "./components/menu";
import { Toasts } from "./components/Toasts";
import { TooltipHost } from "./components/TooltipHost";
import { useAppearance } from "./lib/theme";
import { useLaunchEvents } from "./lib/useLaunchEvents";
import { useReportCurrentProject, useSelectProjectRequests } from "./main/useCurrentProject";
import { createQueryClient } from "./queries/queryClient";
import { useEvents } from "./queries/useEvents";
import { useSettings } from "./queries/useSettings";
import { Shell } from "./shell/Shell";
import { useStore } from "./state/store";

const windowClient = createQueryClient();

/** The main window: the panes and the overlays every window has. */
function MainWindow() {
  const { data: settings } = useSettings();
  useAppearance(settings?.appearance);
  useEvents();
  useMainCommands();
  useReportCurrentProject();
  useSelectProjectRequests();
  const onLaunch = useCallback((event: LaunchEvent) => {
    if (event.type === "error") useStore.getState().toast("error", event.message);
  }, []);
  useLaunchEvents(onLaunch);
  return (
    <>
      <Shell />
      <MenuHost />
      <Toasts />
      <ConfirmHost />
      <TooltipHost />
    </>
  );
}

/** Tests pass a fresh client. */
export default function App({ client = windowClient }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <MainWindow />
    </QueryClientProvider>
  );
}
