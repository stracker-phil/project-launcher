import type { Filter, Project } from "../api";
import type { PageCommand } from "../commands/registry";
import { useLibrary } from "../state/library";
import { useStore } from "../state/store";
import { addProjects, removeProject, setLocked, toggleArchive, toggleStar } from "./actions";
import { copyPath, editProjectFile, openTerminal, revealProject } from "./launch";
import { editField } from "./projectMenu";

const current = () => useLibrary.getState().current;
const editable = () => {
  const project = current();
  return !!project && project.available && !project.locked;
};
const available = () => !!current()?.available;

/** Runs `run` on the project the detail shows. */
const onCurrent = (run: (project: Project) => unknown) => () => {
  const project = current();
  if (project) void run(project);
};

/** The sidebar and list show again first, so the field they need is on screen. */
function showSidebar() {
  if (useStore.getState().sidebarHidden) useStore.getState().toggleSidebar();
}

const showFilter = (filter: Filter) => () => {
  showSidebar();
  useLibrary.getState().setFilter(filter);
};

/** The library's and project's commands; their keys are menu accelerators (`menu/items.rs`). */
export const LIBRARY_COMMANDS: PageCommand[] = [
  { id: "project.new", title: "New Project…", run: addProjects },
  {
    id: "tag.new",
    title: "New Tag…",
    run: () => {
      showSidebar();
      useLibrary.getState().setTagEdit({ mode: "new" });
    },
  },
  {
    id: "view.search",
    title: "Search Projects",
    run: () => {
      showSidebar();
      useLibrary.getState().focusSearch();
    },
  },
  { id: "library.all", title: "All Projects", run: showFilter({ kind: "all" }) },
  { id: "library.starred", title: "Starred", run: showFilter({ kind: "starred" }) },
  { id: "library.archived", title: "Archived", run: showFilter({ kind: "archived" }) },
  { id: "project.rename", title: "Rename Project", when: editable, run: onCurrent((p) => editField(p, "name")) },
  { id: "project.icon", title: "Change Icon & Color…", when: editable, run: onCurrent((p) => editField(p, "icon")) },
  { id: "project.notes", title: "Edit Note", when: editable, run: onCurrent((p) => editField(p, "notes")) },
  { id: "project.star", title: "Star Project", when: available, run: onCurrent(toggleStar) },
  { id: "project.archive", title: "Archive Project", when: available, run: onCurrent(toggleArchive) },
  { id: "project.lock", title: "Lock Project Details", when: available, run: onCurrent((p) => setLocked(p, !p.locked)) },
  { id: "project.terminal", title: "Open Terminal Here", when: available, run: onCurrent(openTerminal) },
  { id: "project.reveal", title: "Reveal in Finder", when: () => !!current(), run: onCurrent(revealProject) },
  { id: "project.copyPath", title: "Copy Path", when: () => !!current(), run: onCurrent(copyPath) },
  { id: "project.editFile", title: "Edit Project File…", when: available, run: onCurrent(editProjectFile) },
  { id: "project.remove", title: "Remove from List", when: () => !!current(), run: onCurrent(removeProject) },
];
