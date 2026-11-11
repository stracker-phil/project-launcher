import type { CSSProperties, ReactNode } from "react";
import { RotateCw } from "lucide-react";
import { api, type FileStats, type PackageScript, type Project, type QuickStats } from "../../api";
import { openContextMenu } from "../../components/menu";
import { colorVar } from "../../lib/icons";
import { useFileStats, useQuickStats, useRefreshStats } from "../../queries/useStats";
import { attempt } from "../../state/attempt";
import { useLibrary } from "../../state/library";
import { ago, bytes, count } from "./format";

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="inspector-section" aria-label={title}>
      <h3 className="inspector-section__title">{title}</h3>
      {children}
    </section>
  );
}

function Row({ label, value, title }: { label: string; value: ReactNode; title?: string }) {
  return (
    <div className="inspector-row">
      <span className="inspector-row__label">{label}</span>
      <span className="inspector-row__value" title={title}>{value}</span>
    </div>
  );
}

/** Rows of the file scan show a placeholder while it reads, so nothing moves when it arrives. */
const whenScanned = (files: FileStats | undefined, value: (files: FileStats) => ReactNode) => (files ? value(files) : "…");

function Overview({ quick, files }: { quick: QuickStats | undefined; files: FileStats | undefined }) {
  const last = files?.recent[0];
  return (
    <Section title="Overview">
      {quick?.kind && <p className="inspector-kind">{quick.kind}</p>}
      {quick && quick.requirements.length > 0 && <p className="inspector-requirements">{quick.requirements.join(" · ")}</p>}
      <Row label="Files" value={whenScanned(files, (f) => `${count(f.files)}${f.truncated ? "+" : ""}`)} />
      <Row label="Source size" value={whenScanned(files, (f) => bytes(f.sourceBytes))} />
      <Row label="On disk" value={whenScanned(files, (f) => (f.diskBytes === null ? "–" : bytes(f.diskBytes)))} />
      <Row label="Lines of code" value={whenScanned(files, (f) => count(f.lines))} />
      <Row label="Last edited" value={whenScanned(files, () => (last ? ago(last.atMs) : "–"))} title={last?.path} />
    </Section>
  );
}

function Languages({ files }: { files: FileStats }) {
  if (files.languages.length === 0) return null;
  const total = files.languages.reduce((sum, l) => sum + l.bytes, 0) || 1;
  const tone = (color: string | null) => ({ "--lang-color": color ? colorVar(color) : "var(--text-faint)" }) as CSSProperties;
  return (
    <Section title="Languages">
      <div className="language-bar" role="img" aria-label={files.languages.map((l) => l.name).join(", ")}>
        {files.languages.map((l) => (
          <span key={l.name} className="language-bar__part" style={{ ...tone(l.color), flexGrow: l.bytes / total }} />
        ))}
      </div>
      {files.languages.map((l) => (
        <div key={l.name} className="language-row">
          <span className="language-row__dot" style={tone(l.color)} aria-hidden />
          <span className="language-row__name">{l.name}</span>
          <span className="language-row__share">{Math.round((l.bytes / total) * 100)}%</span>
          <span className="language-row__detail">{count(l.files)} files · {count(l.lines)} lines</span>
        </div>
      ))}
    </Section>
  );
}

function Activity({ weekly }: { weekly: number[] }) {
  const most = Math.max(1, ...weekly);
  return (
    <div className="activity" role="img" aria-label={`${weekly.reduce((a, b) => a + b, 0)} commits in ${weekly.length} weeks`}>
      {weekly.map((commits, week) => (
        <span key={week} className="activity__bar" style={{ height: `${Math.max(8, (commits / most) * 100)}%`, opacity: commits === 0 ? 0.35 : 1 }} />
      ))}
    </div>
  );
}

function Git({ quick }: { quick: QuickStats }) {
  const git = quick.git;
  if (!git) return null;
  const changes = git.modified + git.untracked === 0 ? "None" : `${git.modified} modified, ${git.untracked} untracked`;
  return (
    <Section title="Git">
      <Row label="Branch" value={git.branch ?? "Detached"} />
      <Row label="Changes" value={changes} />
      {git.hasUpstream && <Row label="Upstream" value={git.ahead + git.behind === 0 ? "Up to date" : `${git.ahead} ahead, ${git.behind} behind`} />}
      {git.lastCommit && <Row label="Last commit" value={`${git.lastCommit.subject} · ${ago(git.lastCommit.atMs)}`} title={git.lastCommit.author} />}
      <Row label="Commits" value={count(git.commits)} />
      {git.firstCommitMs !== null && <Row label="First commit" value={ago(git.firstCommitMs)} />}
      {git.latestTag && <Row label="Latest tag" value={git.latestTag} />}
      <Activity weekly={git.weekly} />
    </Section>
  );
}

function Recent({ project, files }: { project: Project; files: FileStats }) {
  if (files.recent.length === 0) return null;
  return (
    <Section title="Recently Edited">
      {files.recent.map((file) => (
        <button key={file.path} type="button" className="inspector-file" title={file.path} onClick={() => void attempt(api.openProjectFile(project.id, file.path), "Couldn’t open the file")}>
          <span className="inspector-file__name">{file.path.split("/").pop()}</span>
          <span className="inspector-file__when">{ago(file.atMs)}</span>
        </button>
      ))}
    </Section>
  );
}

function Scripts({ project, scripts }: { project: Project; scripts: PackageScript[] }) {
  const requestNewAction = useLibrary((s) => s.requestNewAction);
  if (scripts.length === 0) return null;
  const editable = project.available && !project.locked;
  return (
    <Section title="Package Scripts">
      {scripts.map((script) => (
        <button
          key={`${script.manager}:${script.name}`}
          type="button"
          className="inspector-file"
          title={script.script}
          onClick={() => void attempt(api.runPackageScript(project.id, script.manager, script.name), `Couldn’t run ${script.name}`)}
          onContextMenu={(e) =>
            openContextMenu(e, [{ text: "Add as Action…", enabled: editable, run: () => requestNewAction({ runsAs: "terminal", label: script.name, command: script.command }) }], script.name)
          }
        >
          <span className="inspector-file__name mono">{script.name}</span>
          <span className="inspector-file__when">{script.manager}</span>
        </button>
      ))}
    </Section>
  );
}

/** What the project is: its kind, size, languages, git, recent files and package scripts. */
export function Inspector({ project }: { project: Project | null }) {
  const id = project?.available ? project.id : null;
  const { data: quick } = useQuickStats(id);
  const { data: files } = useFileStats(id);
  const refresh = useRefreshStats(id);
  if (!project) return <p className="inspector-empty">No Project Selected</p>;
  if (!project.available) return <p className="inspector-empty">{project.fileError}</p>;
  // The older of the two reads, once either arrived.
  const oldestRead = Math.min(quick?.computedAtMs ?? Infinity, files?.computedAtMs ?? Infinity);
  return (
    <div className="inspector">
      <div className="inspector-head">
        <h2 className="inspector-head__title">Details</h2>
        <button type="button" className="icon-btn" aria-label="Refresh" title="Refresh" onClick={() => void attempt(refresh(), "Couldn’t refresh the details")}>
          <RotateCw size={14} aria-hidden />
        </button>
      </div>
      <Overview quick={quick} files={files} />
      {files && <Languages files={files} />}
      {quick && <Git quick={quick} />}
      {files && <Recent project={project} files={files} />}
      {quick && <Scripts project={project} scripts={quick.scripts} />}
      {Number.isFinite(oldestRead) && <p className="inspector-updated">Updated {ago(oldestRead)}</p>}
    </div>
  );
}
