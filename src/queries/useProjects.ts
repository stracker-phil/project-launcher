import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const PROJECTS_KEY = ["projects"] as const;

/** Every known project with the name and details from its file; read again on `projects_changed`. */
export function useProjects() {
  return useQuery({ queryKey: PROJECTS_KEY, queryFn: api.listProjects });
}
