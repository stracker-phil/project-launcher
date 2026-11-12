import { useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { useAppearance } from "../../lib/theme";
import { PalettePanel } from "../../palette/PalettePanel";
import { createQueryClient } from "../../queries/queryClient";
import { useEvents } from "../../queries/useEvents";
import { useSettings } from "../../queries/useSettings";
import { usePaletteCommands } from "./usePaletteCommands";

const windowClient = createQueryClient();

/** One opening of the palette: which one, and the project the main window showed then. */
interface Opening {
  count: number;
  currentId: string | null;
}

function PaletteRoot() {
  const { data: settings } = useSettings();
  useAppearance(settings?.appearance, settings?.accent);
  useEvents();
  const [opening, setOpening] = useState<Opening>({ count: 0, currentId: null });
  useEffect(() => {
    const stop = api.on("palette_opened", (currentId) => setOpening((o) => ({ count: o.count + 1, currentId })));
    return () => void stop.then((unlisten) => unlisten());
  }, []);
  // The first frame commits the fresh panel, the second paints it; then Rust makes the window visible.
  useEffect(() => {
    if (opening.count === 0) return;
    requestAnimationFrame(() => requestAnimationFrame(() => void api.paletteShown()));
  }, [opening.count]);
  const commands = usePaletteCommands(opening.currentId);
  return <PalettePanel key={opening.count} commands={commands} onClose={() => void api.closePalette()} />;
}

/** Tests pass a fresh client. */
export function PaletteApp({ client = windowClient }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <PaletteRoot />
    </QueryClientProvider>
  );
}
