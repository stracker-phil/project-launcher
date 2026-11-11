import { api, toAppError, type Filter } from "../api";
import { runPageCommand } from "../commands/registry";
import { useLibrary } from "../state/library";
import { useStore } from "../state/store";
import { launchAction } from "./launch";

/** `action:<list>:<index>` launches one of the current project's actions; `filter:<json>` shows a sidebar filter;
 *  anything else is a page command. The palette sends these through the bus (`page:<id>`). */
export async function runPageRequest(id: string) {
  if (id.startsWith("action:")) return launchCurrentAction(id.slice("action:".length));
  if (id.startsWith("filter:")) return showFilter(id.slice("filter:".length));
  runPageCommand(id);
}

async function launchCurrentAction(spec: string) {
  const project = useLibrary.getState().current;
  const [list, index] = spec.split(":");
  if (!project || (list !== "apps" && list !== "scripts")) return;
  try {
    const config = await api.projectConfig(project.id);
    const action = [...config.apps, ...config.scripts].find((a) => a.edit.source.list === list && a.edit.source.index === Number(index));
    if (action) await launchAction(project, action);
  } catch (e) {
    useStore.getState().toast("error", `Couldn’t run the action: ${toAppError(e).message}`);
  }
}

function showFilter(json: string) {
  let filter: Filter;
  try {
    filter = JSON.parse(json) as Filter;
  } catch {
    return;
  }
  const store = useStore.getState();
  if (store.sidebarHidden) store.toggleSidebar();
  useLibrary.getState().setFilter(filter);
}
