import { parsePrefix, PREFIXES, type PaletteMode, type Prefix } from "./prefixes";
import { ranked } from "./rank";
import type { PaletteCommand } from "./sources";

export type PaletteItem = { kind: "command"; command: PaletteCommand } | { kind: "recent"; text: string };

export interface PaletteSection {
  heading: string;
  items: PaletteItem[];
}

const isSourced = (mode: PaletteMode) => mode === "commands";
/** The prefixes with something to find. */
export const SOURCED_PREFIXES: Prefix[] = PREFIXES.filter((p) => isSourced(p.mode));

const asItems = (list: PaletteCommand[]) => list.map((command): PaletteItem => ({ kind: "command", command }));
const searched = (c: PaletteCommand): [string, string] => [c.label, `${c.section} ${c.detail ?? ""}`];

function bySection(commands: PaletteCommand[]): PaletteSection[] {
  const sections = new Map<string, PaletteCommand[]>();
  for (const command of commands) {
    const list = sections.get(command.section);
    if (list) list.push(command);
    else sections.set(command.section, [command]);
  }
  return [...sections].map(([heading, list]) => ({ heading, items: asItems(list) }));
}

/** The palette's sections for `text`: ranked matches for a query, else recent searches and commands by section. */
export function paletteSections(commands: PaletteCommand[], recent: string[], text: string): PaletteSection[] {
  const { prefix, query } = parsePrefix(text);
  if (prefix && !isSourced(prefix.mode)) return [];
  if (query) {
    const found = ranked(commands, query, searched);
    return found.length > 0 ? [{ heading: "Commands", items: asItems(found) }] : [];
  }
  const searches: PaletteSection[] =
    !prefix && recent.length > 0 ? [{ heading: "Recent Searches", items: recent.map((r): PaletteItem => ({ kind: "recent", text: r })) }] : [];
  return [...searches, ...bySection(commands)];
}
