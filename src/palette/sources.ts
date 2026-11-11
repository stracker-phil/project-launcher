import type { ReactNode } from "react";
import { api, toAppError, type Action, type Filter, type MenuCommand, type Project, type Sidebar } from "../api";
import { bindingLabel } from "../commands/registry";
import { settingsPages } from "../registry/settingsPages";
import type { PaletteMode } from "./prefixes";

export interface PaletteCommand {
  id: string;
  mode: PaletteMode;
  label: string;
  /** Its group while nothing is typed: the menu, "Projects", the project's name. */
  section: string;
  /** Searched with the label and shown after it while searching. */
  detail?: string;
  /** At the right, muted: "Current", a count. */
  hint?: string;
  /** The key that runs it as well ("⌃⌘S"). */
  shortcut?: string;
  /** An option that is on. */
  checked?: boolean;
  /** Before the label; a command's bolt without one. */
  icon?: ReactNode;
  run: () => void;
}

const PAGE = "page:";
const PROJECT_MENU = "Project";
const PROJECTS = "Projects";
const LIBRARY = "Library";
const SETTINGS = "Settings";

/** Runs a bus command; a failure goes to the log (the palette is gone by then). */
export function runBus(id: string) {
  api.runCommand(id).catch((e: unknown) => void api.logError(`Palette: ${id} failed: ${toAppError(e).message}`));
}

function fromMenuItem(item: MenuCommand, mode: PaletteMode, section: string): PaletteCommand {
  return {
    id: item.command,
    mode,
    label: item.label,
    section,
    shortcut: item.key ? bindingLabel(item.key) : undefined,
    run: () => runBus(item.command),
  };
}

/** The menus' commands, except the Project menu's (they show with the project's actions). */
export function menuCommands(menu: MenuCommand[]): PaletteCommand[] {
  return menu
    .filter((item) => item.menu !== PROJECT_MENU && item.command !== "app:palette")
    .map((item) => fromMenuItem(item, "commands", item.menu));
}

/** Each Settings page opens at its own place; its keywords and rows are searched too ("log" finds Status). */
export function settingsCommands(): PaletteCommand[] {
  return settingsPages().map((page) => {
    const id = `app:settings:${page.id}`;
    return {
      id,
      mode: "commands",
      label: `Settings: ${page.title}`,
      section: SETTINGS,
      detail: [...(page.keywords ?? []), ...(page.rows ?? []).map((row) => row.title)].join(", ") || undefined,
      run: () => runBus(id),
    };
  });
}

function projectHint(project: Project, currentId: string | null): string | undefined {
  if (project.id === currentId) return "Current";
  if (project.meta.archived) return "Archived";
  return project.meta.tag ?? undefined;
}

/** Every project, archived ones last; choosing one shows it in the main window. */
export function projectCommands(projects: Project[], currentId: string | null, icon: (project: Project) => ReactNode): PaletteCommand[] {
  return [...projects]
    .sort((a, b) => Number(a.meta.archived) - Number(b.meta.archived))
    .map((project) => ({
      id: `project:${project.id}`,
      mode: "projects",
      label: project.name,
      section: PROJECTS,
      detail: [project.meta.tag ?? "", project.path, ...project.meta.notes].join(" "),
      hint: projectHint(project, currentId),
      icon: icon(project),
      run: () => runBus(`project:show:${project.id}`),
    }));
}

const filterTitle = (filter: Filter) =>
  filter.kind === "tag" ? filter.tag : { all: "All Projects", starred: "Starred", untagged: "Untagged", archived: "Archived" }[filter.kind];

/** The sidebar's filters and tags with their counts; choosing one shows it in the main window. */
export function libraryCommands(sidebar: Sidebar, icon: (filter: Filter) => ReactNode): PaletteCommand[] {
  const untagged: [Filter, number][] = sidebar.untagged === null ? [] : [[{ kind: "untagged" }, sidebar.untagged]];
  const filters: [Filter, number][] = [
    [{ kind: "all" }, sidebar.all],
    [{ kind: "starred" }, sidebar.starred],
    ...sidebar.tags.map((tag): [Filter, number] => [{ kind: "tag", tag: tag.name }, tag.count]),
    ...untagged,
    [{ kind: "archived" }, sidebar.archived],
  ];
  return filters.map(([filter, count]) => ({
    id: `filter:${JSON.stringify(filter)}`,
    mode: "library",
    label: filterTitle(filter),
    section: LIBRARY,
    detail: filter.kind === "tag" ? "tag" : "library",
    hint: String(count),
    icon: icon(filter),
    run: () => runBus(`${PAGE}filter:${JSON.stringify(filter)}`),
  }));
}

/** The current project's actions, then the Project menu's commands, under the project's name. */
export function actionCommands(
  project: Project,
  actions: Action[],
  menu: MenuCommand[],
  icon: (action: Action) => ReactNode,
): PaletteCommand[] {
  const own = actions.map((action): PaletteCommand => {
    const { list, index } = action.edit.source;
    return {
      id: `action:${list}:${index}`,
      mode: "actions",
      label: action.label,
      section: project.name,
      detail: action.runsAs === "script" ? "run script" : undefined,
      shortcut: action.shortcut ? bindingLabel(action.shortcut) : undefined,
      icon: icon(action),
      run: () => runBus(`${PAGE}action:${list}:${index}`),
    };
  });
  const commands = menu.filter((item) => item.menu === PROJECT_MENU).map((item) => fromMenuItem(item, "actions", project.name));
  return [...own, ...commands];
}
