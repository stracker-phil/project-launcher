import { useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { LoaderCircle, Play, Plus, X } from "lucide-react";
import type { Action, Project, ProjectConfig, ProjectStatus, RunsAs } from "../../api";
import { openContextMenu, openMenu, type MenuEntry } from "../../components/menu";
import { bindingLabel } from "../../commands/registry";
import { colorVar, iconFor } from "../../lib/icons";
import { sourceKey, useRuns } from "../../state/runs";
import { launchAction } from "../launch";
import { ActionEditor, type EditorTarget } from "./ActionEditor";
import { duplicateAction, moveAction, removeAction, setBuiltin } from "./actionEdits";

interface Props {
  project: Project;
  config: ProjectConfig;
  status: ProjectStatus | undefined;
  editable: boolean;
}

/** Opens the editor for `target` under the element that asked for it. */
type Edit = (target: EditorTarget, at: HTMLElement) => void;
/** The context menu of an action, or none while the project can't be edited. */
type MenuFor = (action: Action, list: Action[]) => ((e: MouseEvent<HTMLElement>) => void) | undefined;

/** The actions a state doesn't hide now. */
export function shownActions(actions: Action[], status: ProjectStatus | undefined): Action[] {
  const hidden = new Set(status?.hidden.map(sourceKey));
  return actions.filter((a) => !hidden.has(sourceKey(a.edit.source)));
}

/** What a tile opens, in a few words. */
function opens(action: Action): string {
  if (action.runsAs === "app") return action.app ?? "";
  if (action.runsAs === "url") {
    const url = action.url ?? "";
    try {
      return new URL(url).host || url;
    } catch {
      return url;
    }
  }
  return action.command ? `Terminal · ${action.command}` : "Terminal";
}

const editTarget = (action: Action): EditorTarget => ({
  source: action.edit.source,
  draft: action.edit.draft,
  preset: action.edit.preset ?? null,
  defaults: action.edit.defaults ?? null,
});

const newTarget = (runsAs: RunsAs): EditorTarget => ({ source: null, draft: { runsAs, label: "" }, preset: null, defaults: null });

function actionMenu(project: Project, action: Action, list: Action[], onEdit: () => void): MenuEntry[] {
  const index = list.indexOf(action);
  const previous = list[index - 1];
  const next = list[index + 1];
  const duplicate = (runsAs: RunsAs | null) => () => void duplicateAction(project, action, runsAs);
  return [
    { text: "Edit…", run: onEdit },
    { text: "Duplicate", run: duplicate(null) },
    ...(action.runsAs === "terminal" && action.command ? [{ text: "Duplicate as Script", run: duplicate("script") }] : []),
    ...(action.runsAs === "script" ? [{ text: "Duplicate as Terminal Command", run: duplicate("terminal") }] : []),
    "separator",
    { text: "Move Up", enabled: !!previous, run: () => previous && void moveAction(project, action, previous.edit.source.index) },
    { text: "Move Down", enabled: !!next, run: () => next && void moveAction(project, action, next.edit.source.index) },
    "separator",
    { text: "Remove", danger: true, run: () => void removeAction(project, action) },
  ];
}

/** New actions and the built-ins as toggles; one that can't run here shows why. */
function addMenuEntries(project: Project, config: ProjectConfig, onNew: (runsAs: RunsAs) => void): MenuEntry[] {
  return [
    { text: "New Action…", run: () => onNew("app") },
    { text: "New Script…", run: () => onNew("script") },
    "separator",
    { caption: "Built-in Apps" },
    ...config.builtins.map((b) => ({
      text: b.label,
      toggle: true,
      checked: b.enabled,
      enabled: b.enabled || !b.unavailableReason,
      hint: b.enabled ? undefined : (b.unavailableReason ?? undefined),
      run: () => void setBuiltin(project, b.preset, !b.enabled),
    })),
  ];
}

function ActionTile({ action, running, onContextMenu, onClick }: {
  action: Action;
  running: boolean;
  onContextMenu?: (e: MouseEvent<HTMLButtonElement>) => void;
  onClick: () => void;
}) {
  const Icon = iconFor(action.icon);
  return (
    <button
      type="button"
      className="action-tile"
      style={action.color ? ({ "--tile-color": colorVar(action.color) } as CSSProperties) : undefined}
      onClick={onClick}
      onContextMenu={onContextMenu}
    >
      <Icon size={16} aria-hidden className="action-tile__icon" />
      <span className="action-tile__text">
        <span className="action-tile__label">{action.label}</span>
        <span className="action-tile__opens">{opens(action)}</span>
      </span>
      <span className={running ? "action-tile__dot action-tile__dot--on" : "action-tile__dot"} role={running ? "img" : undefined} aria-label={running ? "Running" : undefined} />
      <span className="action-tile__key">{action.shortcut ? bindingLabel(action.shortcut) : ""}</span>
    </button>
  );
}

function OpenGrid({ project, config, apps, running, editable, edit, menuFor }: {
  project: Project;
  config: ProjectConfig;
  apps: Action[];
  running: Set<string>;
  editable: boolean;
  edit: Edit;
  menuFor: MenuFor;
}) {
  const addButton = useRef<HTMLButtonElement>(null);
  const openAddMenu = () => {
    const at = addButton.current;
    if (at) openMenu(addMenuEntries(project, config, (runsAs) => edit(newTarget(runsAs), at)), at, "Add Action");
  };
  return (
    <section className="detail-section">
      <div className="detail-section__head">
        <h2 className="detail-section__title">Open</h2>
        {editable && (
          <button ref={addButton} type="button" className="icon-btn tiny" aria-label="Add Action" title="Add Action" onClick={openAddMenu}>
            <Plus size={14} aria-hidden />
          </button>
        )}
      </div>
      {apps.length === 0 ? (
        <p className="detail-section__empty">{editable ? "Nothing to open yet: add an action or a built-in app with +." : "Nothing to open."}</p>
      ) : (
        <div className="action-grid">
          {apps.map((app) => (
            <ActionTile
              key={sourceKey(app.edit.source)}
              action={app}
              running={running.has(sourceKey(app.edit.source))}
              onClick={() => void launchAction(project, app)}
              onContextMenu={menuFor(app, apps)}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function ScriptRow({ action, running, onRun, onContextMenu }: {
  action: Action;
  running: boolean;
  onRun: () => void;
  onContextMenu?: (e: MouseEvent<HTMLDivElement>) => void;
}) {
  return (
    <div className="script-row" onContextMenu={onContextMenu}>
      <span className="script-row__label">{action.label}</span>
      <span className="script-row__command mono">{action.command}</span>
      <span className="script-row__key">{action.shortcut ? bindingLabel(action.shortcut) : ""}</span>
      <button type="button" className="icon-btn tiny" aria-label={running ? `${action.label} is running` : `Run ${action.label}`} disabled={running} onClick={onRun}>
        {running ? <LoaderCircle size={14} className="spin" aria-hidden /> : <Play size={14} aria-hidden />}
      </button>
    </div>
  );
}

function LastRunPanel({ project }: { project: Project }) {
  const last = useRuns((s) => s.last[project.id]);
  const dismiss = useRuns((s) => s.dismiss);
  if (!last) return null;
  const { result } = last;
  const outcome = result.ok ? "Finished" : result.timedOut ? "Stopped" : `Failed${result.exitCode === null ? "" : ` (exit ${result.exitCode})`}`;
  return (
    <div className={result.ok ? "last-run" : "last-run last-run--failed"} role="region" aria-label="Last Run">
      <div className="last-run__head">
        <span className="last-run__title">{last.label}</span>
        <span className="last-run__outcome">{outcome} · {(result.durationMs / 1000).toFixed(1)} s</span>
        <button type="button" className="icon-btn tiny" aria-label="Close Output" onClick={() => dismiss(project.id)}>
          <X size={14} aria-hidden />
        </button>
      </div>
      <pre className="last-run__output scroller mono">{result.output || "No output."}</pre>
    </div>
  );
}

function ScriptsSection({ project, scripts, menuFor }: { project: Project; scripts: Action[]; menuFor: MenuFor }) {
  const running = new Set(useRuns((s) => s.running[project.id]));
  const hasLastRun = useRuns((s) => project.id in s.last);
  if (scripts.length === 0 && !hasLastRun) return null;
  return (
    <section className="detail-section">
      <h2 className="detail-section__title">Scripts</h2>
      {scripts.length > 0 && (
        <div className="script-list">
          {scripts.map((script) => (
            <ScriptRow
              key={sourceKey(script.edit.source)}
              action={script}
              running={running.has(sourceKey(script.edit.source))}
              onRun={() => void launchAction(project, script)}
              onContextMenu={menuFor(script, scripts)}
            />
          ))}
        </div>
      )}
      <LastRunPanel project={project} />
    </section>
  );
}

/** The project's Open grid and Scripts: a click launches; while unlocked, + and each action's menu edit them. */
export function ActionsSection({ project, config, status, editable }: Props) {
  const [editing, setEditing] = useState<EditorTarget | null>(null);
  const anchor = useRef<HTMLElement | null>(null);
  const edit: Edit = (target, at) => {
    anchor.current = at;
    setEditing(target);
  };
  const menuFor: MenuFor = (action, list) =>
    editable
      ? (e) => {
          const at = e.currentTarget;
          openContextMenu(e, actionMenu(project, action, list, () => edit(editTarget(action), at)), action.label);
        }
      : undefined;
  return (
    <>
      <OpenGrid
        project={project}
        config={config}
        apps={shownActions(config.apps, status)}
        running={new Set(status?.running.map(sourceKey))}
        editable={editable}
        edit={edit}
        menuFor={menuFor}
      />
      <ScriptsSection project={project} scripts={shownActions(config.scripts, status)} menuFor={menuFor} />
      <ActionEditor project={project} target={editing} anchor={anchor} onClose={() => setEditing(null)} />
    </>
  );
}
