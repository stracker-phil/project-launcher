import { skipToken, useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Read again on `dev_changed`, `projects_changed`, `settings_changed` and every few seconds while shown. */
export const DEV_STATUS_KEY = ["devStatus"] as const;

const REFRESH_MS = 5000;

/** Docker, the project's server and its database; off while no project is given. */
export function useDevStatus(id: string | null) {
  return useQuery({
    queryKey: [...DEV_STATUS_KEY, id],
    queryFn: id === null ? skipToken : () => api.devStatus(id),
    refetchInterval: REFRESH_MS,
  });
}
