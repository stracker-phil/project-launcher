import { api, toAppError, type MetaEdit, type Project } from "../api";
import { attempt } from "../state/attempt";
import { ALL, useLibrary } from "../state/library";
import { useStore, type Store } from "../state/store";

const toast: Store["toast"] = (...args) => useStore.getState().toast(...args);

/** Selects the first new project and starts renaming it, else the first folder that was listed already. */
export async function addProjects() {
  let result;
  try {
    result = await api.addProjectFolders();
  } catch (e) {
    toast("error", `Couldn’t add project: ${toAppError(e).message}`);
    return;
  }
  const { added, existing } = result;
  const library = useLibrary.getState();
  const first = added[0] ?? existing[0];
  if (!first) return;
  library.setFilter(ALL);
  library.select(first);
  if (added.length > 0) library.edit(first, "name");
  if (added.length > 1) toast("success", `Added ${added.length} projects`);
  if (added.length === 0) {
    toast("info", existing.length > 1 ? "These folders are already in your list" : "This folder is already in your list");
  }
}

/** Takes it off the list at once; the toast's Undo puts it back. */
export async function removeProject(project: Project) {
  if (!(await attempt(api.removeProject(project.id), "Couldn’t remove project"))) return;
  toast("success", `Removed “${project.name}”. The folder and its config file are untouched.`, {
    label: "Undo",
    run: () => void attempt(api.undo(), "Couldn’t restore project"),
  });
}

export function setMeta(project: Project, edit: MetaEdit, failure = "Couldn’t update project") {
  return attempt(api.setProjectMeta(project.id, edit), failure);
}

export function rename(project: Project, name: string) {
  return attempt(api.setProjectName(project.id, name), "Couldn’t update project");
}

export function toggleStar(project: Project) {
  return setMeta(project, { field: "starred", value: !project.meta.starred });
}

export async function toggleArchive(project: Project) {
  const archive = !project.meta.archived;
  if (await setMeta(project, { field: "archived", value: archive })) {
    toast("success", `${archive ? "Archived" : "Unarchived"} ${project.name}`);
  }
}

export function setLocked(project: Project, locked: boolean) {
  return attempt(api.setProjectLocked(project.id, locked), locked ? "Couldn’t lock the project" : "Couldn’t unlock the project");
}

/** A new tag gets onto the tag list too. */
export function setTag(project: Project, tag: string | null) {
  return setMeta(project, { field: "tag", value: tag }, "Couldn’t change tag");
}
