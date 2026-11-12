import { skipToken, useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Read again on `dev_changed`, `projects_changed`, `settings_changed` and every few seconds while shown. */
export const DEV_STATUS_KEY = ["devStatus"] as const;

const REFRESH_MS = 5000;

const RUNNING_REFRESH_MS = 15000;

/** The ids of the projects whose dev server runs; under the same key, so the same events read it again. */
export function useDevRunning() {
  return useQuery({
    queryKey: [...DEV_STATUS_KEY, "running"],
    queryFn: () => api.devRunning(),
    refetchInterval: RUNNING_REFRESH_MS,
  });
}

/** Docker, the project's server and its database; off while no project is given. */
export function useDevStatus(id: string | null) {
  return useQuery({
    queryKey: [...DEV_STATUS_KEY, id],
    queryFn: id === null ? skipToken : () => api.devStatus(id),
    refetchInterval: REFRESH_MS,
  });
}

/** The `dev:` commands that apply to the project now, for the palette; off while no project is given. */
export function useDevCommands(id: string | null) {
  return useQuery({
    queryKey: [...DEV_STATUS_KEY, "commands", id],
    queryFn: id === null ? skipToken : () => api.devCommands(id),
    refetchInterval: REFRESH_MS,
  });
}
