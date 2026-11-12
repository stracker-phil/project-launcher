/** The rows of Settings → Shortcuts: the menu items' keys from Rust and the page commands no menu item runs. */
import type { MenuShortcut } from "../../api";
import { MAIN_COMMANDS } from "../../commands/mainCommands";
import { resolveBinding, type Overrides, type PageCommand } from "../../commands/registry";
import { SETTINGS_COMMANDS } from "../../commands/settingsCommands";

/** Where a key works: a menu key in every window, a page key only in its own. */
type ShortcutScope = "menu" | "main" | "settings";

export interface ShortcutRow {
  /** What `settings.json` stores the key under: the menu item's command, else the page command's id. */
  id: string;
  title: string;
  /** Where the key works. */
  group: string;
  /** The menu a menu item sits in. */
  detail?: string;
  scope: ShortcutScope;
  key: string | null;
  defaultKey: string | null;
  overridden: boolean;
}

const MENU_GROUP = "In every window";

const PAGE_GROUPS: { title: string; scope: ShortcutScope; commands: PageCommand[] }[] = [
  { title: "In the main window", scope: "main", commands: MAIN_COMMANDS },
  { title: "In the Settings window", scope: "settings", commands: SETTINGS_COMMANDS },
];

/** The id a row is opened and found by in Settings. */
export const rowAnchor = (id: string) => `shortcut:${id}`;

function menuRows(menu: MenuShortcut[]): ShortcutRow[] {
  return menu.map((item) => ({
    id: item.command,
    title: item.label.replace(/…$/, ""),
    group: MENU_GROUP,
    detail: `${item.menu} menu`,
    scope: "menu",
    key: item.key,
    defaultKey: item.defaultKey,
    overridden: item.key !== item.defaultKey,
  }));
}

/** The page commands of every window, except those a menu item already runs. */
function pageRows(menu: MenuShortcut[], overrides: Overrides): ShortcutRow[] {
  const inMenu = new Set(menu.map((item) => item.command));
  return PAGE_GROUPS.flatMap(({ title, scope, commands }) =>
    commands
      .filter((command) => !inMenu.has(`page:${command.id}`))
      .map((command) => {
        const key = resolveBinding(command.id, command.defaultBinding, overrides);
        const defaultKey = resolveBinding(command.id, command.defaultBinding, {});
        return { id: command.id, title: command.title, group: title, scope, key, defaultKey, overridden: key !== defaultKey };
      }),
  );
}

export function shortcutRows(menu: MenuShortcut[], overrides: Overrides): ShortcutRow[] {
  return [...menuRows(menu), ...pageRows(menu, overrides)];
}

/** Rows by group, groups in the order they first appear. */
export function groupRows(rows: ShortcutRow[]): { title: string; rows: ShortcutRow[] }[] {
  const groups = new Map<string, ShortcutRow[]>();
  for (const row of rows) {
    const grouped = groups.get(row.group) ?? [];
    if (grouped.length === 0) groups.set(row.group, grouped);
    grouped.push(row);
  }
  return [...groups].map(([title, grouped]) => ({ title, rows: grouped }));
}

/** Menu keys work in every window, page keys only in their own. */
const sharesWindow = (a: ShortcutRow, b: ShortcutRow) => a.scope === "menu" || b.scope === "menu" || a.scope === b.scope;

/** The other row that already has `key` where `row` would use it. */
export function conflictFor(rows: ShortcutRow[], row: ShortcutRow, key: string): ShortcutRow | undefined {
  return rows.find((other) => other.id !== row.id && other.key === key && sharesWindow(row, other));
}
