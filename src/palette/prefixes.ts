/** What a prefix narrows the palette to. */
export type PaletteMode = "commands" | "projects" | "library" | "actions";

export interface Prefix {
  char: string;
  mode: PaletteMode;
  label: string;
  placeholder: string;
}

/** Typed first, each limits the palette to one kind (docs/ux.md → Command palette). */
export const PREFIXES: Prefix[] = [
  { char: ">", mode: "commands", label: "Commands", placeholder: "Run a menu command" },
  { char: ":", mode: "projects", label: "Projects", placeholder: "Find a project" },
  { char: "#", mode: "library", label: "Library", placeholder: "Go to a filter or tag" },
  { char: "/", mode: "actions", label: "Actions", placeholder: "Run an action of the current project" },
];

/** The prefix in the text's first character, if any, and the query after it. */
export function parsePrefix(text: string): { prefix: Prefix | null; query: string } {
  const prefix = PREFIXES.find((p) => text.startsWith(p.char)) ?? null;
  return { prefix, query: (prefix ? text.slice(prefix.char.length) : text).trim() };
}
