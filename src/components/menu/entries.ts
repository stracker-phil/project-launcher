import { isItem, type MenuEntry } from "../../state/menuTypes";

export interface Section {
  caption?: string;
  /** Each entry with its index in the shown entries. */
  entries: [MenuEntry, number][];
}

export const isEnabled = (entry: MenuEntry | undefined) => !!entry && isItem(entry) && entry.enabled !== false;

/** The rows whose text contains `query` (ignoring case). A caption stays while one of its rows does; a separator
 *  stays only between rows that do, so a row after it never joins the caption above. */
export function matching(entries: MenuEntry[], query: string): MenuEntry[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return entries;
  const kept: MenuEntry[] = [];
  let caption: MenuEntry | null = null;
  let separated = false;
  for (const entry of entries) {
    if (entry === "separator") {
      caption = null;
      separated = kept.length > 0;
      continue;
    }
    if ("caption" in entry) {
      caption = entry;
      continue;
    }
    if (!entry.text.toLowerCase().includes(needle)) continue;
    if (separated) kept.push("separator");
    if (caption) kept.push(caption);
    caption = null;
    separated = false;
    kept.push(entry);
  }
  return kept;
}

/** A caption starts a captioned section, which runs to the next caption or separator. */
export function sectionsOf(entries: MenuEntry[]): Section[] {
  const sections: Section[] = [{ entries: [] }];
  entries.forEach((entry, index) => {
    const last = sections[sections.length - 1];
    if (entry !== "separator" && "caption" in entry) sections.push({ caption: entry.caption, entries: [] });
    else if (entry === "separator" && last.caption !== undefined) sections.push({ entries: [[entry, index]] });
    else last.entries.push([entry, index]);
  });
  return sections.filter((section) => section.caption !== undefined || section.entries.length > 0);
}
