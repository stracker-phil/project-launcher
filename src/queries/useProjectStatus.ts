import { skipToken, useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Read again on `projects_changed` (a launch, a changed file) and every few seconds while shown. */
export const PROJECT_STATUS_KEY = ["projectStatus"] as const;

const REFRESH_MS = 5000;

/** The project's states, hidden actions and running terminal commands; off while no project is given. */
export function useProjectStatus(id: string | null) {
  return useQuery({
    queryKey: [...PROJECT_STATUS_KEY, id],
    queryFn: id === null ? skipToken : () => api.projectStatus(id),
    refetchInterval: REFRESH_MS,
  });
}
