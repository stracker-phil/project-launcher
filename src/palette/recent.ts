/** Searches that led to a command, newest first; kept for the page's lifetime only. */
const searches: string[] = [];
const LIMIT = 5;

export function rememberSearch(text: string) {
  const query = text.trim();
  if (!query) return;
  const at = searches.indexOf(query);
  if (at !== -1) searches.splice(at, 1);
  searches.unshift(query);
  searches.length = Math.min(searches.length, LIMIT);
}

export const recentSearches = (): string[] => [...searches];

/** Forgets every search; for tests. */
export function clearRecentSearches() {
  searches.length = 0;
}
