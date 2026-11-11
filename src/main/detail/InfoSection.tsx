import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { api, type DevMode, type Project, type ProjectConfig } from "../../api";
import { InlineText } from "../../components/InlineText";
import { openMenu } from "../../components/menu";
import { attempt } from "../../state/attempt";
import { setMeta } from "../actions";

const DEV_MODES: { mode: DevMode; label: string }[] = [
  { mode: "none", label: "None" },
  { mode: "docker", label: "Docker" },
  { mode: "ddev", label: "Docker, DDEV" },
  { mode: "wp-env", label: "Docker, wp-env" },
];

function InfoRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt className="info__label">{label}</dt>
      <dd className="info__value">{children}</dd>
    </>
  );
}

function LinkRow({ project, field, label, value, readOnly }: {
  project: Project;
  field: "url" | "repoUrl";
  label: string;
  value: string | null;
  readOnly: boolean;
}) {
  if (readOnly && !value) return null;
  return (
    <InfoRow label={label}>
      <InlineText
        value={value ?? ""}
        placeholder="Add URL…"
        label={label}
        readOnly={readOnly}
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
  return (
    <section className="detail-section">
      <h2 className="detail-section__title">Info</h2>
      <dl className="info">
        <LinkRow project={project} field="url" label="Website" value={url} readOnly={readOnly} />
        <LinkRow project={project} field="repoUrl" label="Repository" value={repoUrl} readOnly={readOnly} />
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
      <dl className="info info--mono">
        {config.env.map((v) => (
          <InfoRow key={v.name} label={v.name}>
            <span className="mono">{v.value}</span>
          </InfoRow>
        ))}
      </dl>
    </section>
  );
}
