import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { api, type Filter } from "../api";

/** Read again on `projects_changed` and `settings_changed` (the tag list). */
export const LIBRARY_KEY = ["library"] as const;

/** The sidebar, title and list sections for a filter and a search; keeps showing the last answer while the next loads. */
export function useLibraryView(filter: Filter, search: string) {
  return useQuery({
    queryKey: [...LIBRARY_KEY, filter, search],
    queryFn: () => api.library(filter, search),
    placeholderData: keepPreviousData,
  });
}
