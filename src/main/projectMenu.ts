import type { Project } from "../api";
import type { MenuEntry } from "../components/menu";
import { useLibrary } from "../state/library";
import { removeProject, setLocked, setTag, toggleArchive, toggleStar } from "./actions";
import { copyPath, editProjectFile, openClaudeManager, openTerminal, revealProject } from "./launch";

const separator: MenuEntry = "separator";

/** Selects the project and puts one of its detail fields into edit mode. */
export function editField(project: Project, field: "name" | "icon" | "tag" | "notes") {
  const library = useLibrary.getState();
  library.select(project.id);
  library.edit(project.id, field);
}

/** "Move to Tag": no tag, each tag (the current one checked) and a new one. */
export function tagMenu(project: Project, tags: string[]): MenuEntry[] {
  const current = project.meta.tag;
  return [
    { text: "No Tag", checked: current === null, run: () => void setTag(project, null) },
    ...(tags.length > 0 ? [separator] : []),
    ...tags.map((tag) => ({ text: tag, checked: current === tag, run: () => void setTag(project, tag) })),
    separator,
    { text: "New Tag…", run: () => editField(project, "tag") },
  ];
}

/** The project's menu: a row's right-click. A locked project can't be renamed, recoloured or retagged. */
export function projectMenu(project: Project, tags: string[]): MenuEntry[] {
  const editable = project.available && !project.locked;
  return [
    { text: project.meta.starred ? "Unstar Project" : "Star Project", enabled: project.available, run: () => void toggleStar(project) },
    {
      text: project.meta.archived ? "Unarchive Project" : "Archive Project",
      enabled: project.available,
      run: () => void toggleArchive(project),
    },
    { text: "Move to Tag", enabled: editable, items: tagMenu(project, tags) },
    "separator",
    { text: "Rename Project", shortcut: "⌘E", enabled: editable, run: () => editField(project, "name") },
    { text: "Change Icon & Color…", enabled: editable, run: () => editField(project, "icon") },
    { text: "Edit Note", enabled: editable, run: () => editField(project, "notes") },
    {
      text: project.locked ? "Unlock Project Details" : "Lock Project Details",
      shortcut: "⇧⌘L",
      enabled: project.available,
      run: () => void setLocked(project, !project.locked),
    },
    "separator",
    { text: "Open Terminal Here", enabled: project.available, run: () => void openTerminal(project) },
    { text: "Reveal in Finder", run: () => void revealProject(project) },
    { text: "Copy Path", shortcut: "⌥⌘C", run: () => void copyPath(project) },
    {
      text: "Manage Claude",
      shortcut: "⇧⌘M",
      enabled: project.available && project.canManageClaude,
      run: () => void openClaudeManager(project),
    },
    { text: "Edit Project File…", enabled: project.available, run: () => void editProjectFile(project) },
    "separator",
    { text: "Remove from List", danger: true, run: () => void removeProject(project) },
  ];
}
