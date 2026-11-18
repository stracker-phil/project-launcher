import { useState, type ReactNode } from "react";
import { ChevronDown, Globe, Link, Pencil } from "lucide-react";
import { api, type DevMode, type Project, type ProjectConfig } from "../../api";
import { InlineText } from "../../components/InlineText";
import { openMenu } from "../../components/menu";
import { attempt } from "../../state/attempt";
import { setMeta } from "../actions";
import { openClaudeManager } from "../launch";

const DEV_MODES: { mode: DevMode; label: string }[] = [
  { mode: "none", label: "None" },
  { mode: "docker", label: "Docker" },
  { mode: "ddev", label: "Docker, DDEV" },
  { mode: "wp-env", label: "Docker, wp-env" },
];

function InfoRow({ label, children, trailing }: { label: string; children: ReactNode; trailing?: ReactNode }) {
  return (
    <div className="info__row">
      <dt className="info__label">{label}</dt>
      <dd className="info__value">
        <span className="info__content">{children}</span>
        {trailing}
      </dd>
    </div>
  );
}

function LinkRow({ project, field, label, value, readOnly, editing, onEditingChange, icon }: {
  project: Project;
  field: "url" | "repoUrl";
  label: string;
  value: string | null;
  readOnly: boolean;
  editing: boolean;
  onEditingChange: (editing: boolean) => void;
  icon: ReactNode;
}) {
  if (readOnly && !value) return null;
  const openLabel = field === "url" ? "Open Website" : "Open Repository";
  return (
    <InfoRow
      label={label}
      trailing={
        <button type="button" className="icon-btn tiny" aria-label={openLabel} title={openLabel} disabled={!value} onClick={() => value && void api.openUrl(value)}>
          {icon}
        </button>
      }
    >
      <InlineText
        value={value ?? ""}
        placeholder="Add URL…"
        label={label}
        readOnly={readOnly}
        editing={editing}
        onEditingChange={onEditingChange}
        onSave={(v) => void setMeta(project, { field, value: v || null }, "Couldn’t update link")}
      />
    </InfoRow>
  );
}

function openDevModeMenu(project: Project, current: DevMode, anchor: HTMLElement) {
  openMenu(
    DEV_MODES.map(({ mode, label }) => ({
      text: label,
      checked: current === mode,
      run: () => void attempt(api.setDevMode(project.id, mode), "Couldn’t change the dev environment"),
    })),
    anchor,
    "Dev Environment",
  );
}

/** The links and the dev environment are edited here; the rest is read-only. */
export function InfoSection({ project, config, readOnly }: { project: Project; config: ProjectConfig | undefined; readOnly: boolean }) {
  const { url, repoUrl, editor } = project.meta;
  const devLabel = DEV_MODES.find((d) => d.mode === config?.dev)?.label ?? "None";
  const [editingLink, setEditingLink] = useState<"url" | "repoUrl" | null>(null);
  return (
    <section className="detail-section">
      <div className="detail-section__head">
        <h2 className="detail-section__title">Info</h2>
        {!readOnly && (
          <button type="button" className="icon-btn tiny" aria-label="Edit Links" title="Edit Links" onClick={() => setEditingLink("url")}>
            <Pencil size={13} aria-hidden />
          </button>
        )}
      </div>
      <dl className="info card">
        <LinkRow project={project} field="url" label="Website" value={url} readOnly={readOnly} editing={editingLink === "url"} onEditingChange={(on) => setEditingLink(on ? "url" : null)} icon={<Globe size={14} aria-hidden />} />
        <LinkRow project={project} field="repoUrl" label="Repository" value={repoUrl} readOnly={readOnly} editing={editingLink === "repoUrl"} onEditingChange={(on) => setEditingLink(on ? "repoUrl" : null)} icon={<Link size={14} aria-hidden />} />
        {config && (!readOnly || config.dev !== "none") && (
          <InfoRow label="Dev Environment">
            {readOnly ? (
              <span>{devLabel}</span>
            ) : (
              <button
                type="button"
                className="info__select"
                aria-haspopup="menu"
                onClick={(e) => openDevModeMenu(project, config.dev, e.currentTarget)}
              >
                {devLabel}
                <ChevronDown size={12} aria-hidden />
              </button>
            )}
          </InfoRow>
        )}
        {config?.claude && (
          <InfoRow
            label="Claude"
            trailing={
              <button
                type="button"
                className="btn"
                title={project.canManageClaude ? "Open in Claude Manager" : "Claude Manager isn’t installed"}
                disabled={!project.canManageClaude}
                onClick={() => void openClaudeManager(project)}
              >
                Manage
              </button>
            }
          >
            <span className="info__path mono" title={config.claude.profile}>{config.claude.profile}</span>
          </InfoRow>
        )}
        <InfoRow label="Location">
          <span className="info__path mono" title={project.path}>{project.path}</span>
        </InfoRow>
        {editor && <InfoRow label="Editor">{editor}</InfoRow>}
      </dl>
    </section>
  );
}

/** The project's environment variables, as written (placeholders filled in). */
export function EnvSection({ config }: { config: ProjectConfig }) {
  if (config.env.length === 0) return null;
  return (
    <section className="detail-section">
      <h2 className="detail-section__title">Environment Variables</h2>
      <dl className="info info--mono card">
        {config.env.map((v) => (
          <InfoRow key={v.name} label={v.name}>
            <span className="mono">{v.value}</span>
          </InfoRow>
        ))}
      </dl>
    </section>
  );
}
