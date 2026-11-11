import { QueryClient } from "@tanstack/react-query";

/** Data changes only when Rust says so (ADR 0014): no retries, never stale, no refetch on focus. One per window;
 *  tests make a fresh one. */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  });
}
