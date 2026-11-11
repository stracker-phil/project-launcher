import { skipToken, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api";

/** Stats stay fresh this long; Rust caches them as long too. */
const STALE_MS = 5 * 60 * 1000;

export const quickStatsKey = (id: string | null) => ["stats", id, "quick"] as const;
export const fileStatsKey = (id: string | null) => ["stats", id, "files"] as const;

/** Manifests and git of the project; off while no project is given. */
export function useQuickStats(id: string | null) {
  return useQuery({
    queryKey: quickStatsKey(id),
    queryFn: id === null ? skipToken : () => api.quickStats(id, false),
    staleTime: STALE_MS,
  });
}

/** Files, languages and disk usage of the project; off while no project is given. */
export function useFileStats(id: string | null) {
  return useQuery({
    queryKey: fileStatsKey(id),
    queryFn: id === null ? skipToken : () => api.fileStats(id, false),
    staleTime: STALE_MS,
  });
}

/** Reads both again past Rust's cache. */
export function useRefreshStats(id: string | null) {
  const client = useQueryClient();
  return async () => {
    if (id === null) return;
    const [quick, files] = await Promise.all([api.quickStats(id, true), api.fileStats(id, true)]);
    client.setQueryData(quickStatsKey(id), quick);
    client.setQueryData(fileStatsKey(id), files);
  };
}
