import { api, toAppError, type Filter } from "../api";
import { runPageCommand } from "../commands/registry";
import { useLibrary } from "../state/library";
import { useStore } from "../state/store";
import { setTag } from "./actions";
import { launchAction } from "./launch";

/** `action:<list>:<index>` launches one of the current project's actions; `filter:<json>` shows a sidebar filter;
 *  `tag:<json>` moves the current project to a tag (`null`: none); anything else is a page command. The palette
 *  sends these through the bus (`page:<id>`). */
export async function runPageRequest(id: string) {
  if (id.startsWith("action:")) return launchCurrentAction(id.slice("action:".length));
  if (id.startsWith("filter:")) return showFilter(id.slice("filter:".length));
  if (id.startsWith("tag:")) return moveToTag(id.slice("tag:".length));
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

function parse(json: string): unknown {
  try {
    return JSON.parse(json);
  } catch {
    return undefined;
  }
}

function showFilter(json: string) {
  const filter = parse(json) as Filter | undefined;
  if (!filter) return;
  const store = useStore.getState();
  if (store.sidebarHidden) store.toggleSidebar();
  useLibrary.getState().setFilter(filter);
}

/** As the right-click's "Move to Tag"; a locked project keeps its tag. */
async function moveToTag(json: string) {
  const tag = parse(json);
  const project = useLibrary.getState().current;
  if (!project?.available || project.locked || (tag !== null && typeof tag !== "string")) return;
  await setTag(project, tag);
}
