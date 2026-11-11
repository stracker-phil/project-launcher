import { api, type Action, type ActionDraft, type ActionSource, type Builtin, type Project, type RunsAs } from "../../api";
import { attempt } from "../../state/attempt";

/** Adds (`source` null) or changes an action; a refusal (a missing field) rejects, for the editor to show. */
export function saveAction(project: Project, source: ActionSource | null, draft: ActionDraft, preset: Builtin | null) {
  return api.saveAction(project.id, source, draft, preset);
}

/** A copy without its key: "<label> Copy", or as `runsAs` under the same label. */
export function duplicateAction(project: Project, action: Action, runsAs: RunsAs | null) {
  return attempt(api.duplicateAction(project.id, action.edit.source, runsAs), "Couldn’t duplicate the action");
}

export function removeAction(project: Project, action: Action) {
  return attempt(api.removeAction(project.id, action.edit.source), `Couldn’t remove “${action.label}”`);
}

export function moveAction(project: Project, action: Action, to: number) {
  return attempt(api.moveAction(project.id, action.edit.source, to), `Couldn’t move “${action.label}”`);
}

export function setBuiltin(project: Project, preset: Builtin, enabled: boolean) {
  return attempt(api.setBuiltinEnabled(project.id, preset, enabled), "Couldn’t change the built-in apps");
}
