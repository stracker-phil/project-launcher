/** Settings pages by id (ADR 0033) and the search over their rows (ADR 0046). */
import { useSyncExternalStore, type ComponentType } from "react";
import type { LucideIcon } from "lucide-react";
import { ranked } from "../lib/rank";

/** A row the search finds: `id` is the row's `anchor`, `title` its label. */
export interface SettingsRow {
  id: string;
  title: string;
  /** More words the search finds the row by. */
  keywords?: string[];
  /** The anchor to scroll to while the row isn't shown, e.g. a field only one choice of a pop-up adds. */
  under?: string;
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

export const findRow = (page: SettingsPage, id: string | undefined) => page.rows?.find((row) => row.id === id);

/** The named page, the page holding the named row, or the first when the name is absent or unknown. */
export function resolveSettingsPage(id: string | undefined): SettingsPage | undefined {
  const all = settingsPages();
  return all.find((page) => page.id === id) ?? all.find((page) => findRow(page, id)) ?? all[0];
}

/** A search hit: its page, and the row that matched, if any. */
export interface SettingsMatch {
  page: SettingsPage;
  row?: SettingsRow;
}

/** Every page and row matching `query`, best first (`rankMatch`). */
export function searchSettings(query: string): SettingsMatch[] {
  const entries = settingsPages().flatMap((page): SettingsMatch[] => [{ page }, ...(page.rows ?? []).map((row) => ({ page, row }))]);
  return ranked(entries, query, ({ page, row }) =>
    row ? [row.title, (row.keywords ?? []).join(" ")] : [page.title, [page.group, ...(page.keywords ?? [])].join(" ")],
  );
}

/** The page list under its group headings. */
export function settingsGroups(): { group: string; pages: SettingsPage[] }[] {
  const groups: { group: string; pages: SettingsPage[] }[] = [];
  for (const page of settingsPages()) {
    const last = groups.at(-1);
    if (last?.group === page.group) last.pages.push(page);
    else groups.push({ group: page.group, pages: [page] });
  }
  return groups;
}
