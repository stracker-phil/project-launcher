import { useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { useSettingsCommands } from "../../commands";
import { Toasts } from "../../components/Toasts";
import { TooltipHost } from "../../components/TooltipHost";
import { useAppearance } from "../../lib/theme";
import { createQueryClient } from "../../queries/queryClient";
import { SettingsWindow } from "../../settings/SettingsWindow";

const windowClient = createQueryClient();

/** The page Rust opened the window at (`?page=`); later pages arrive as `open_settings`. */
function initialPage(): string | undefined {
  return new URLSearchParams(window.location.search).get("page") ?? undefined;
}

function SettingsRoot() {
  const [page, setPage] = useState(initialPage);
  useAppearance();
  useSettingsCommands();
  useEffect(() => {
    const stopListening = api.on("open_settings", setPage);
    return () => void stopListening.then((stop) => stop());
  }, []);
  return (
    <>
      <SettingsWindow target={page} onPage={setPage} />
      <Toasts />
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
