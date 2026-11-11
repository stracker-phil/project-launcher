/** Settings pages by id (ADR 0033). Pages register here before the Settings window renders. */
import type { ComponentType } from "react";
import type { LucideIcon } from "lucide-react";

/** A row of a page that the search finds by its title; `open_settings` with its id opens the page there. */
export interface SettingsRow {
  id: string;
  title: string;
}

export interface SettingsPage {
  id: string;
  title: string;
  icon: LucideIcon;
  /** Lower sorts first; ties break by title. */
  order: number;
  /** More words the search finds the page by. */
  keywords?: string[];
  rows?: SettingsRow[];
  component: ComponentType;
}

const pages = new Map<string, SettingsPage>();

/** Adds a page, or replaces the one with the same id. */
export function registerSettingsPage(page: SettingsPage): void {
  pages.set(page.id, page);
}

/** Sorted by `order`, ties broken by title. */
export function settingsPages(): SettingsPage[] {
  return [...pages.values()].sort((a, b) => a.order - b.order || a.title.localeCompare(b.title));
}

export const hasRow = (page: SettingsPage, id: string | undefined) => page.rows?.some((row) => row.id === id) ?? false;

/** The named page, the page holding the named row, or the first when the name is absent or unknown. */
export function resolveSettingsPage(id: string | undefined): SettingsPage | undefined {
  const all = settingsPages();
  return all.find((page) => page.id === id) ?? all.find((page) => hasRow(page, id)) ?? all[0];
}

/** A page in the list; `row` when the search found the page by that row's title. */
export interface SettingsMatch {
  page: SettingsPage;
  row?: SettingsRow;
}

/** Pages whose title, a keyword or a row title contains `query` (any case), in list order. */
export function searchSettingsPages(query: string): SettingsMatch[] {
  const needle = query.trim().toLowerCase();
  const has = (text: string) => text.toLowerCase().includes(needle);
  return settingsPages().flatMap((page) => {
    if (has(page.title) || page.keywords?.some(has)) return [{ page }];
    const row = page.rows?.find((candidate) => has(candidate.title));
    return row ? [{ page, row }] : [];
  });
}
