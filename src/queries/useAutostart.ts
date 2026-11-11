import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const AUTOSTART_KEY = ["autostart"] as const;

/** Whether Launcher opens at login; only asked where it can be (`enabled`). */
export function useAutostart(enabled: boolean) {
  return useQuery({ queryKey: AUTOSTART_KEY, queryFn: api.autostartEnabled, enabled });
}
