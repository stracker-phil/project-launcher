import type { CSSProperties, ReactNode } from "react";
import { RotateCw } from "lucide-react";
import { api, type FileStats, type GitStats, type PackageScript, type Project, type QuickStats } from "../../api";
import { openContextMenu } from "../../components/menu";
import { colorVar } from "../../lib/icons";
import { useFileStats, useQuickStats, useRefreshStats } from "../../queries/useStats";
import { attempt } from "../../state/attempt";
import { useLibrary } from "../../state/library";
import { ago, bytes, count, short } from "./format";
import { Section } from "./Section";

function Row({ label, value, title }: { label: string; value: ReactNode; title?: string }) {
  return (
    <div className="inspector-row">
      <span className="inspector-row__label">{label}</span>
      <span className="inspector-row__value" title={title ?? (typeof value === "string" ? value : undefined)}>{value}</span>
    </div>
  );
}

/** Rows of the file scan show a placeholder while it reads, so nothing moves when it arrives. */
const whenScanned = (files: FileStats | undefined, value: (files: FileStats) => ReactNode) => (files ? value(files) : "…");

function Overview({ quick, files }: { quick: QuickStats | undefined; files: FileStats | undefined }) {
  const last = files?.recent[0];
  return (
    <Section title="Overview">
      {quick?.kind && <Row label="Type" value={quick.kind} />}
      {quick && quick.requirements.length > 0 && <Row label="Requires" value={quick.requirements.join(" · ")} />}
      {quick?.packages && <Row label="Packages" value={quick.packages} />}
      <Row label="Files" value={whenScanned(files, (f) => `${count(f.files)}${f.truncated ? "+" : ""}`)} />
      <Row label="Lines" value={whenScanned(files, (f) => count(f.lines))} />
      <Row label="Source size" value={whenScanned(files, (f) => bytes(f.sourceBytes))} />
      <Row label="On disk" value={whenScanned(files, (f) => (f.diskBytes === null ? "–" : bytes(f.diskBytes)))} />
      <Row label="Last edited" value={whenScanned(files, () => (last ? ago(last.atMs) : "–"))} title={last?.path} />
    </Section>
  );
}

function Languages({ files }: { files: FileStats }) {
  if (files.languages.length === 0) return null;
  const total = files.languages.reduce((sum, l) => sum + l.bytes, 0) || 1;
  const share = (b: number) => `${Math.round((b / total) * 100)}%`;
  const tone = (color: string | null) => ({ "--lang-color": color ? colorVar(color) : "var(--text-faint)" }) as CSSProperties;
  return (
    <Section title="Languages">
      <div className="language-bar" role="img" aria-label={files.languages.map((l) => l.name).join(", ")}>
        {files.languages.map((l) => (
          <span key={l.name} className="language-bar__part" style={{ ...tone(l.color), flexGrow: l.bytes / total }} />
        ))}
      </div>
      <div className="language-table">
        <div className="language-table__row language-table__head">
          <span>Language</span>
          <span>Files</span>
          <span>Lines</span>
          <span>Size</span>
        </div>
        {files.languages.map((l) => (
          <div key={l.name} className="language-table__row" title={`${l.name}: ${share(l.bytes)}`}>
            <span className="language-table__name">
              <span className="language-table__dot" style={tone(l.color)} aria-hidden />
              <span className="language-table__label">{l.name}</span>
            </span>
            <span>{short(l.files)}</span>
            <span>{short(l.lines)}</span>
            <span>{bytes(l.bytes)}</span>
          </div>
        ))}
      </div>
    </Section>
  );
}

function Activity({ weekly }: { weekly: number[] }) {
  const most = Math.max(1, ...weekly);
  const total = weekly.reduce((a, b) => a + b, 0);
  return (
    <div className="activity">
      <div className="activity__bars" role="img" aria-label={`${total} commits in ${weekly.length} weeks`}>
        {weekly.map((commits, week) => (
          <span key={week} className={commits === 0 ? "activity__bar activity__bar--empty" : "activity__bar"} style={{ height: `${Math.max(8, (commits / most) * 100)}%` }} />
        ))}
      </div>
      <p className="activity__caption">
        {total} {total === 1 ? "commit" : "commits"} in the last {weekly.length} weeks
      </p>
    </div>
  );
}

function changesText(git: GitStats): string {
  const parts = [git.modified > 0 && `${git.modified} modified`, git.untracked > 0 && `${git.untracked} untracked`].filter(Boolean);
  return parts.length === 0 ? "Clean" : parts.join(" · ");
}

function remoteText(git: GitStats): string {
  if (!git.hasUpstream) return "No remote";
  const parts = [git.ahead > 0 && `${git.ahead} ahead`, git.behind > 0 && `${git.behind} behind`].filter(Boolean);
  return parts.length === 0 ? "Up to date" : parts.join(" · ");
}

function Git({ quick }: { quick: QuickStats }) {
  const git = quick.git;
  if (!git) return null;
  const last = git.lastCommit;
  return (
    <Section title="Git">
      <Row label="Branch" value={git.branch ?? "Detached"} />
      <Row label="Changes" value={changesText(git)} />
      <Row label="Remote" value={remoteText(git)} />
      <Row label="Commits" value={count(git.commits)} />
      {git.started !== null && <Row label="Started" value={git.started} />}
      {git.latestTag && <Row label="Latest tag" value={git.latestTag} />}
      {last && (
        <div className="inspector-commit">
          <span className="inspector-row__label">Last commit</span>
          <p className="inspector-commit__subject" title={last.subject}>{last.subject}</p>
          <p className="inspector-commit__meta" title={last.author}>
            {ago(last.atMs)} · {last.author}
          </p>
        </div>
      )}
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
  // Infinity until a read arrives.
  const oldestRead = Math.min(quick?.computedAtMs ?? Infinity, files?.computedAtMs ?? Infinity);
  return (
    <div className="inspector">
      <div className="inspector-head">
        <h2 className="inspector-head__title">Details</h2>
        <button type="button" className="icon-btn inspector-refresh" aria-label="Refresh" title="Refresh" onClick={() => void attempt(refresh(), "Couldn’t refresh the details")}>
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
