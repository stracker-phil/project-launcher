import { api, toAppError, type Action, type Outcome, type Project } from "../api";
import { attempt } from "../state/attempt";
import { useRuns } from "../state/runs";
import { useStore, type Store } from "../state/store";

const toast: Store["toast"] = (...args) => useStore.getState().toast(...args);

const seconds = (ms: number) => `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;

function report(action: Action, outcome: Outcome) {
  if (outcome.kind === "ran") {
    const { result } = outcome;
    if (result.ok) toast("success", `${action.label} finished in ${seconds(result.durationMs)}`);
    else if (result.timedOut) toast("error", `${action.label} was stopped after ${seconds(result.durationMs)}`);
    else {
      const code = result.exitCode === null ? "" : ` with exit code ${result.exitCode}`;
      toast("error", `${action.label} failed${code}`);
    }
    return;
  }
  if (outcome.how === "needsPermission") {
    toast("info", "To bring the running tab to the front, allow Launcher to control your terminal in System Settings → Privacy & Security → Automation.");
  }
}

/** Opens an app, link or terminal command; a script runs in the background and its output shows under Scripts. */
export async function launchAction(project: Project, action: Action) {
  const { source } = action.edit;
  const script = action.runsAs === "script";
  const runs = useRuns.getState();
  if (script) runs.start(project.id, source);
  try {
    const outcome = await api.launchAction(project.id, source);
    if (outcome.kind === "ran") runs.finish(project.id, source, { label: action.label, result: outcome.result });
    report(action, outcome);
  } catch (e) {
    if (script) runs.finish(project.id, source);
    toast("error", `Couldn’t run “${action.label}”: ${toAppError(e).message}`);
  }
}

export function openTerminal(project: Project) {
  return attempt(api.openTerminal(project.id), "Couldn’t open a terminal");
}

export function openClaudeManager(project: Project) {
  return attempt(api.openClaudeManager(project.id), "Couldn’t open Claude Manager");
}

export function revealProject(project: Project) {
  return attempt(api.revealProject(project.id), "Couldn’t show the folder in Finder");
}

export async function copyPath(project: Project) {
  if (await attempt(api.copyProjectPath(project.id), "Couldn’t copy the path")) toast("success", "Copied the project’s path");
}

export function editProjectFile(project: Project) {
  return attempt(api.editProjectFile(project.id), "Couldn’t open the project file");
}
