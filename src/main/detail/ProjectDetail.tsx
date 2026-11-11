import { GitBranch, Lock, LockOpen, PanelRight, Star } from "lucide-react";
import { useMemo } from "react";
import type { Action, Library, Project, ProjectConfig, ProjectStatus } from "../../api";
import { InlineText } from "../../components/InlineText";
import { useProjectConfig } from "../../queries/useProjectConfig";
import { useDevStatus } from "../../queries/useDevStatus";
import { useProjectStatus } from "../../queries/useProjectStatus";
import { useQuickStats } from "../../queries/useStats";
import { useLibrary, type EditField } from "../../state/library";
import { useStore } from "../../state/store";
import { addProjects, rename, setLocked, setMeta, toggleStar } from "../actions";
import { ActionsSection, shownActions } from "./ActionsSection";
import { IconPicker } from "./IconPicker";
import { DevSection } from "./DevSection";
import { EnvSection, InfoSection } from "./InfoSection";
import { StatesRow } from "./StatesRow";
import { TagPill } from "./TagPill";
import { useActionShortcuts } from "./useActionShortcuts";

export function DetailTools({ project }: { project: Project | null }) {
  const toggleInspector = useStore((s) => s.toggleInspector);
  const lockLabel = project?.locked ? "Unlock Project Details" : "Lock Project Details";
  const starLabel = project?.meta.starred ? "Unstar Project" : "Star Project";
  return (
    <div className="detail-tools">
      {project?.available && (
        <>
          <button
            type="button"
            className={project.locked ? "icon-btn icon-btn--on" : "icon-btn"}
            aria-label={lockLabel}
            title={lockLabel}
            onClick={() => void setLocked(project, !project.locked)}
          >
            {project.locked ? <Lock size={16} /> : <LockOpen size={16} />}
          </button>
          <button
            type="button"
            className={project.meta.starred ? "icon-btn star-btn star-btn--on" : "icon-btn star-btn"}
            aria-label={starLabel}
            aria-pressed={project.meta.starred}
            title={starLabel}
            onClick={() => void toggleStar(project)}
          >
            <Star size={16} />
          </button>
        </>
      )}
      <button type="button" className="icon-btn" aria-label="Toggle Inspector" title="Toggle Inspector" onClick={toggleInspector}>
        <PanelRight size={16} />
      </button>
    </div>
  );
}

function EmptyDetail({ library }: { library: Library | undefined }) {
  const none = library && library.sidebar.all === 0 && library.sidebar.archived === 0;
  if (none) {
    return (
      <div className="detail-empty">
        <h2>No projects yet</h2>
        <p>Add a project folder to launch its editor, terminal, git client and scripts from one place.</p>
        <button type="button" className="btn btn-primary" onClick={() => void addProjects()}>
          Add Project Folder…
        </button>
      </div>
    );
  }
  return (
    <div className="detail-empty">
      <h2>No Project Selected</h2>
      <p>Choose a project from the list.</p>
    </div>
  );
}

const NO_ACTIONS: Action[] = [];

/** The actions shown now, in one stable list per config and status. */
function useShownActions(config: ProjectConfig | undefined, status: ProjectStatus | undefined): Action[] {
  return useMemo(() => (config ? shownActions([...config.apps, ...config.scripts], status) : NO_ACTIONS), [config, status]);
}

/** The selected project; its header fields are edited in place. */
export function ProjectDetail({ project, library }: { project: Project | null; library: Library | undefined }) {
  const editing = useLibrary((s) => s.editing);
  const edit = useLibrary((s) => s.edit);
  const stopEditing = useLibrary((s) => s.stopEditing);
  const shownId = project?.available ? project.id : null;
  const { data: config } = useProjectConfig(shownId);
  const { data: status } = useProjectStatus(shownId);
  const { data: dev } = useDevStatus(shownId);
  const branch = useQuickStats(shownId).data?.git?.branch;
  const shortcutActions = useShownActions(config, status);
  useActionShortcuts(shownId ? project : null, shortcutActions);
  if (!project) return <EmptyDetail library={library} />;

  const readOnly = project.locked || !project.available;
  const tags = library?.sidebar.tags.map((t) => t.name) ?? [];
  const isEditing = (field: EditField) => editing?.id === project.id && editing.field === field;
  const setEditing = (field: EditField) => (on: boolean) => (on ? edit(project.id, field) : stopEditing());
  const notes = project.meta.notes.join("\n");
  const showNotes = !readOnly || notes !== "";

  return (
    <div className="detail">
      <header className="detail-header">
        <IconPicker project={project} readOnly={readOnly} open={isEditing("icon")} onOpenChange={setEditing("icon")} />
        <div className="detail-header__text">
          <InlineText
            className="detail-header__name"
            value={project.name}
            placeholder={project.path.split("/").pop() ?? ""}
            label="Project name"
            readOnly={readOnly}
            editing={isEditing("name")}
            onEditingChange={setEditing("name")}
            onSave={(name) => void rename(project, name.trim())}
          />
          <div className="detail-header__badges">
            <TagPill project={project} tags={tags} readOnly={readOnly} editing={isEditing("tag")} onEditingChange={setEditing("tag")} />
            {branch && (
              <span className="branch-pill" title="Git branch">
                <GitBranch size={11} aria-hidden />
                {branch}
              </span>
            )}
            {project.meta.archived && <span className="badge">Archived</span>}
            {project.locked && <span className="badge">Locked</span>}
          </div>
          {showNotes && (
            <InlineText
              className="detail-header__notes"
              multiline
              value={notes}
              placeholder="Add a note…"
              label="Notes"
              readOnly={readOnly}
              editing={isEditing("notes")}
              onEditingChange={setEditing("notes")}
              onSave={(text) => void setMeta(project, { field: "notes", value: text ? text.split("\n") : [] })}
            />
          )}
        </div>
      </header>
      {project.fileError && <div className="callout callout--error">{project.fileError}</div>}
      {status && <StatesRow states={status.states} />}
      <DevSection project={project} status={dev} />
      {config && <ActionsSection project={project} config={config} status={status} editable={!readOnly} />}
      <InfoSection project={project} config={config} readOnly={readOnly} />
      {config && <EnvSection config={config} />}
    </div>
  );
}
