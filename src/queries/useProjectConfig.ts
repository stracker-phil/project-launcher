import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Read again on `projects_changed` and `settings_changed`: the default editor and git client resolve into it. */
export const PROJECT_CONFIG_KEY = ["projectConfig"] as const;

/** One project's resolved launch config; off while no project is given. */
export function useProjectConfig(id: string | null) {
  return useQuery({
    queryKey: [...PROJECT_CONFIG_KEY, id],
    queryFn: () => api.projectConfig(id ?? ""),
    enabled: id !== null,
  });
}
