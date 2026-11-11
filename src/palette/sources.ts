import { api, toAppError, type MenuCommand } from "../api";
import { allCommands, bindingFor, bindingLabel, getCommand, runPageCommand, type PageCommand } from "../commands/registry";
import { isItem, type MenuEntry } from "../state/menuTypes";
import { settingsPages } from "../registry/settingsPages";
import { useStore } from "../state/store";
import { paletteMenus, type PaletteMenu } from "./menus";

export interface PaletteCommand {
  id: string;
  label: string;
  /** Its menu, or the part of the page it comes from. */
  section: string;
  /** Searched with the label and shown after it. */
  detail?: string;
  /** The key that runs it as well ("⌃⌘S"). */
  shortcut?: string;
  /** An option that is on. */
  checked?: boolean;
  run: () => void;
}

const PAGE = "page:";
/** The palette's own command; the palette never lists itself. */
const PALETTE_COMMAND = "palette.open";
const PAGE_SECTION = "Window";
const SETTINGS_SECTION = "Settings";

const keyOf = (command: PageCommand) => bindingLabel(bindingFor(command.id)) || undefined;

/** Runs a bus command; a failure shows as an error toast. */
function runBus(id: string, label: string) {
  api.runCommand(id).catch((e: unknown) => useStore.getState().toast("error", `Could not run “${label}”: ${toAppError(e).message}`));
}

const runPage = (id: string) => () => void runPageCommand(id);

/** What the page offers at one moment. */
export interface PageSnapshot {
  /** The page's commands that could run. */
  runnable: Set<string>;
  /** The page's commands with a title, by id. */
  commands: PageCommand[];
  /** The menus on screen and the Settings pages. */
  own: PaletteCommand[];
}

/** Taken before the palette is on screen: some commands only run while no dialog or text field has the focus. */
export function snapshotPage(): PageSnapshot {
  const commands = allCommands().filter((c) => c.title && c.id !== PALETTE_COMMAND);
  return {
    runnable: new Set(commands.flatMap((c) => ((c.when?.() ?? true) ? [c.id] : []))),
    commands,
    own: [...paletteMenus().flatMap((menu) => flatten(menu, menu.entries())), ...settingsCommands()],
  };
}

/** A `page:` item runs the page's command (left out when it couldn't run), any other item the bus command. */
function fromMenu(item: MenuCommand, page: PageSnapshot): PaletteCommand | null {
  const base = { id: item.command, label: item.label, section: item.menu };
  if (!item.command.startsWith(PAGE)) return { ...base, shortcut: item.key ?? undefined, run: () => runBus(item.command, item.label) };
  const command = getCommand(item.command.slice(PAGE.length));
  if (!command || !page.runnable.has(command.id)) return null;
  return { ...base, shortcut: keyOf(command) ?? item.key ?? undefined, run: runPage(command.id) };
}

/** The page's commands no menu item runs. */
function pageCommands(menu: MenuCommand[], page: PageSnapshot): PaletteCommand[] {
  const inMenus = new Set(menu.map((item) => item.command));
  return page.commands
    .filter((c) => page.runnable.has(c.id) && !inMenus.has(`${PAGE}${c.id}`))
    .map((c) => ({ id: `${PAGE}${c.id}`, label: c.title, section: PAGE_SECTION, shortcut: keyOf(c), run: runPage(c.id) }));
}

/** A menu's rows, submenus flattened: "Project: Move to Tag › Clients". Captions, separators and gray rows drop. */
function flatten(menu: PaletteMenu, entries: MenuEntry[], path: string[] = []): PaletteCommand[] {
  return entries.flatMap((entry): PaletteCommand[] => {
    if (!isItem(entry) || entry.enabled === false) return [];
    const at = [...path, entry.text];
    if (entry.items) return flatten(menu, entry.items, at);
    const run = entry.run;
    if (!run) return [];
    const label = `${menu.title}: ${at.join(" › ")}`;
    return [{ id: `${menu.section}:${label}`, label, section: menu.section, detail: entry.hint, shortcut: entry.shortcut, checked: entry.checked, run }];
  });
}

/** Each Settings page opens at its own place; its keywords and rows are searched too ("log" finds Status). */
function settingsCommands(): PaletteCommand[] {
  return settingsPages().map((page) => {
    const id = `app:settings:${page.id}`;
    const label = `Settings: ${page.title}`;
    return {
      id,
      label,
      section: SETTINGS_SECTION,
      detail: [...(page.keywords ?? []), ...(page.rows ?? []).map((row) => row.title)].join(", ") || undefined,
      run: () => runBus(id, label),
    };
  });
}

export function collectCommands(menu: MenuCommand[], page: PageSnapshot): PaletteCommand[] {
  return [...menu.flatMap((item) => fromMenu(item, page) ?? []), ...pageCommands(menu, page), ...page.own];
}
