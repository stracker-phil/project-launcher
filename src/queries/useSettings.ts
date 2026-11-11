import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const SETTINGS_KEY = ["settings"] as const;

/** The settings and the tag list; read again on `settings_changed`. */
export function useSettings() {
  return useQuery({ queryKey: SETTINGS_KEY, queryFn: api.getSettings });
}
