import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type EventName } from "../api";
import { LIBRARY_KEY } from "./useLibraryView";
import { MENU_SHORTCUTS_KEY } from "./useMenuShortcuts";
import { PROJECT_CONFIG_KEY } from "./useProjectConfig";
import { PROJECT_STATUS_KEY } from "./useProjectStatus";
import { PROJECTS_KEY } from "./useProjects";
import { SETTINGS_KEY } from "./useSettings";

/** Which reads each Rust event makes stale (ADR 0014). */
export const INVALIDATES: Partial<Record<EventName, readonly (readonly string[])[]>> = {
  projects_changed: [PROJECTS_KEY, PROJECT_CONFIG_KEY, PROJECT_STATUS_KEY, LIBRARY_KEY],
  settings_changed: [SETTINGS_KEY, MENU_SHORTCUTS_KEY, PROJECT_CONFIG_KEY, LIBRARY_KEY],
};

/** Refetches the queries a Rust event makes stale, while mounted; every window mounts it once. */
export function useEvents() {
  const client = useQueryClient();
  useEffect(() => {
    const stops = Object.entries(INVALIDATES).map(([event, keys]) =>
      api.on(event as EventName, () => {
        for (const queryKey of keys ?? []) void client.invalidateQueries({ queryKey });
      }),
    );
    return () => stops.forEach((stop) => void stop.then((unlisten) => unlisten()));
  }, [client]);
}
