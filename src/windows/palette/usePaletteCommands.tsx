import { Archive, FolderClosed, Globe, Play, RotateCw, Square, Star, Tag, Tags } from "lucide-react";
import type { Action, Filter, MenuCommand } from "../../api";
import { colorVar, iconFor, ProjectIcon } from "../../lib/icons";
import { shownActions } from "../../main/detail/ActionsSection";
import { actionCommands, libraryCommands, menuCommands, projectCommands, settingsCommands, type PaletteCommand } from "../../palette/sources";
import { useDevCommands } from "../../queries/useDevStatus";
import { useLibraryView } from "../../queries/useLibraryView";
import { useMenuCommands } from "../../queries/useMenuCommands";
import { useProjectConfig } from "../../queries/useProjectConfig";
import { useProjectStatus } from "../../queries/useProjectStatus";
import { useProjects } from "../../queries/useProjects";

const ALL: Filter = { kind: "all" };
const FILTER_ICONS = { all: FolderClosed, starred: Star, untagged: Tags, archived: Archive, tag: Tag };

function filterIcon(filter: Filter) {
  const Icon = FILTER_ICONS[filter.kind];
  return <Icon size={16} strokeWidth={1.8} aria-hidden />;
}

function actionIcon(action: Action) {
  const Icon = iconFor(action.icon);
  return <Icon size={16} strokeWidth={1.8} aria-hidden style={action.color ? { color: colorVar(action.color) } : undefined} />;
}

/** `dev:<step>:<id>`: the step's verb as its icon. */
function devIcon(item: MenuCommand) {
  const step = item.command.split(":")[1] ?? "";
  const Icon = step.endsWith("-stop") ? Square : step.endsWith("-restart") ? RotateCw : step === "open-site" ? Globe : Play;
  return <Icon size={16} strokeWidth={1.8} aria-hidden />;
}

/** Everything the palette lists, with the actions of the project the main window shows (`currentId`). */
export function usePaletteCommands(currentId: string | null): PaletteCommand[] {
  const { data: projects = [] } = useProjects();
  const { data: library } = useLibraryView(ALL, "");
  const { data: menu = [] } = useMenuCommands(currentId);
  const { data: config } = useProjectConfig(currentId);
  const { data: status } = useProjectStatus(currentId);
  const { data: dev = [] } = useDevCommands(currentId);
  const current = projects.find((p) => p.id === currentId && p.available);
  return [
    ...projectCommands(projects, currentId, (p) => <ProjectIcon icon={p.meta.icon} color={p.meta.color} size={20} />),
    ...(library ? libraryCommands(library.sidebar, filterIcon) : []),
    ...(current && config ? actionCommands(current, shownActions([...config.apps, ...config.scripts], status), menu, actionIcon, dev, devIcon) : []),
    ...menuCommands(menu),
    ...settingsCommands(),
  ];
}
