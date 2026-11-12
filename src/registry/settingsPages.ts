/** Settings pages by id (ADR 0033). Pages register here before the Settings window renders. */
import { useSyncExternalStore, type ComponentType } from "react";
import type { LucideIcon } from "lucide-react";

/** A row of a page that the search finds by its title; `open_settings` with its id opens the page there. */
export interface SettingsRow {
  id: string;
  title: string;
}

export interface SettingsPage {
  id: string;
  title: string;
  /** The heading the page is listed under (`GROUP_ORDER`). */
  group: string;
  /** One line under the page title: what the page is for. */
  intro?: string;
  icon: LucideIcon;
  /** Position inside its group; lower sorts first, ties break by title. */
  order: number;
  /** More words the search finds the page by. */
  keywords?: string[];
  rows?: SettingsRow[];
  component: ComponentType;
}

/** The list's group order; any other group follows them, by name. */
const GROUP_ORDER = ["App", "System"];

const pages = new Map<string, SettingsPage>();
const listeners = new Set<() => void>();
let version = 0;

/** Adds a page, or replaces the one with the same id. */
export function registerSettingsPage(page: SettingsPage): void {
  pages.set(page.id, page);
  version += 1;
  listeners.forEach((notify) => notify());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

/** Re-renders the caller when a page is registered, so a page may register again after the window opened. */
export function useSettingsPagesVersion(): number {
  return useSyncExternalStore(subscribe, () => version);
}

function groupRank(group: string): number {
  const index = GROUP_ORDER.indexOf(group);
  return index === -1 ? GROUP_ORDER.length : index;
}

/** By group (known groups first), then `order`, then title. */
function compareSettingsPages(a: SettingsPage, b: SettingsPage): number {
  return groupRank(a.group) - groupRank(b.group) || a.group.localeCompare(b.group) || a.order - b.order || a.title.localeCompare(b.title);
}

export function settingsPages(): SettingsPage[] {
  return [...pages.values()].sort(compareSettingsPages);
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

/** Pages whose title, group, a keyword or a row title contains `query` (any case), in list order. */
export function searchSettingsPages(query: string): SettingsMatch[] {
  const needle = query.trim().toLowerCase();
  const has = (text: string) => text.toLowerCase().includes(needle);
  return settingsPages().flatMap((page) => {
    if (has(page.title) || has(page.group) || page.keywords?.some(has)) return [{ page }];
    const row = page.rows?.find((candidate) => has(candidate.title));
    return row ? [{ page, row }] : [];
  });
}

/** The list's groups with their matches, narrowed to `query`; every page for "". */
export function settingsGroups(query = ""): { group: string; matches: SettingsMatch[] }[] {
  const groups: { group: string; matches: SettingsMatch[] }[] = [];
  for (const match of searchSettingsPages(query)) {
    const last = groups.at(-1);
    if (last?.group === match.page.group) last.matches.push(match);
    else groups.push({ group: match.page.group, matches: [match] });
  }
  return groups;
}
