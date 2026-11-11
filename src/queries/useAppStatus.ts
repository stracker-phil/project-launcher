import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const APP_STATUS_KEY = ["appStatus"] as const;

/** Version, build, folders and the command line link; read again whenever the window comes back, since the
 *  link can change outside the app. */
export function useAppStatus() {
  return useQuery({ queryKey: APP_STATUS_KEY, queryFn: api.appStatus, refetchOnWindowFocus: "always" });
}
