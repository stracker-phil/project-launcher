import { parsePrefix, PREFIXES, type PaletteMode, type Prefix } from "./prefixes";
import { ranked } from "../lib/rank";
import type { PaletteCommand } from "./sources";

export type PaletteItem = { kind: "command"; command: PaletteCommand } | { kind: "recent"; text: string };

export interface PaletteSection {
  heading: string;
  items: PaletteItem[];
}

/** The order the kinds show in, while searching and without a query. */
const MODES: PaletteMode[] = ["projects", "library", "actions", "commands"];
const HEADINGS: Record<PaletteMode, string> = { projects: "Projects", library: "Library", actions: "Actions", commands: "Commands" };

const asItems = (list: PaletteCommand[]) => list.map((command): PaletteItem => ({ kind: "command", command }));
const searched = (c: PaletteCommand): [string, string] => [c.label, `${c.section} ${c.detail ?? ""}`];

/** The prefixes with something to find. */
export function sourcedPrefixes(commands: PaletteCommand[]): Prefix[] {
  return PREFIXES.filter((p) => commands.some((c) => c.mode === p.mode));
}

function bySection(commands: PaletteCommand[]): PaletteSection[] {
  const sections = new Map<string, PaletteCommand[]>();
  for (const mode of MODES) {
    for (const command of commands.filter((c) => c.mode === mode)) {
      const list = sections.get(command.section);
      if (list) list.push(command);
      else sections.set(command.section, [command]);
    }
  }
  return [...sections].map(([heading, list]) => ({ heading, items: asItems(list) }));
}

/** Each kind's ranked matches under its heading; the current project's actions under its name. */
function searchSections(commands: PaletteCommand[], query: string): PaletteSection[] {
  return MODES.flatMap((mode) => {
    const found = ranked(
      commands.filter((c) => c.mode === mode),
      query,
      searched,
    );
    if (found.length === 0) return [];
    const heading = mode === "actions" ? found[0].section : HEADINGS[mode];
    return [{ heading, items: asItems(found) }];
  });
}

/** Recent searches (without a prefix), then everything by section. */
function browseSections(commands: PaletteCommand[], recent: string[], prefixed: boolean): PaletteSection[] {
  const searches: PaletteSection[] =
    !prefixed && recent.length > 0 ? [{ heading: "Recent Searches", items: recent.map((r): PaletteItem => ({ kind: "recent", text: r })) }] : [];
  return [...searches, ...bySection(commands)];
}

/** The palette's sections for `text`; a prefix keeps one kind. */
export function paletteSections(commands: PaletteCommand[], recent: string[], text: string): PaletteSection[] {
  const { prefix, query } = parsePrefix(text);
  const shown = prefix ? commands.filter((c) => c.mode === prefix.mode) : commands;
  return query ? searchSections(shown, query) : browseSections(shown, recent, prefix !== null);
}
