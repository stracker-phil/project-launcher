import { useCallback, useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { useSettingsCommands } from "../../commands";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { MenuHost } from "../../components/menu";
import { Toasts } from "../../components/Toasts";
import { TooltipHost } from "../../components/TooltipHost";
import { useAppearance } from "../../lib/theme";
import { trackWindowFocus } from "../../lib/windowFocus";
import { createQueryClient } from "../../queries/queryClient";
import { useEvents } from "../../queries/useEvents";
import { useSettings } from "../../queries/useSettings";
import { SettingsWindow, type SettingsRequest } from "../../settings/SettingsWindow";

const windowClient = createQueryClient();

/** The page or row Rust opened the window at (`?page=`). */
function initialRequest(): SettingsRequest {
  return { id: new URLSearchParams(window.location.search).get("page") ?? undefined };
}

function SettingsRoot() {
  const [request, setRequest] = useState(initialRequest);
  const open = useCallback((id: string) => setRequest({ id }), []);
  const { data: settings } = useSettings();
  useAppearance(settings?.appearance, settings?.accent);
  useEvents();
  useSettingsCommands();
  useEffect(trackWindowFocus, []);
  useEffect(() => {
    const stopListening = api.on("open_settings", open);
    return () => void stopListening.then((stop) => stop());
  }, [open]);
  return (
    <>
      <SettingsWindow request={request} onOpen={open} />
      <Toasts />
      <ConfirmHost />
      <MenuHost />
      <TooltipHost />
    </>
  );
}

/** Tests pass a fresh client; pages must be registered before this renders. */
export function SettingsApp({ client = windowClient }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <SettingsRoot />
    </QueryClientProvider>
  );
}
