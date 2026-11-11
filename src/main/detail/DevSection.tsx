import { Database, Globe, LoaderCircle } from "lucide-react";
import { api, type Activity, type DatabaseInfo, type DevAction, type DevStatus, type Project, type ServerState, type ServerStatus } from "../../api";
import { devAction, openDatabase } from "../devEnvironment";

const ACTIVITY: Record<Activity, string> = { starting: "Starting…", stopping: "Stopping…", restarting: "Restarting…" };
const STATES: Record<ServerState, string> = { running: "Running", paused: "Paused", stopped: "Stopped", notConfigured: "Not set up", unknown: "Unknown" };
const SOURCES: Record<DatabaseInfo["source"], string> = { ddev: "DDEV", "wp-env": "wp-env", config: "Project file" };

/** A server that isn't set up names what is missing. */
const serverState = (server: ServerStatus) => (server.state === "notConfigured" && server.detail) || STATES[server.state];

function host(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

function Dot({ on }: { on: boolean }) {
  return <span className={on ? "dev-dot dev-dot--on" : "dev-dot"} aria-hidden />;
}

function DatabaseRow({ project, database }: { project: Project; database: DatabaseInfo }) {
  return (
    <div className="dev-row">
      <span className="dev-row__name">
        <Database size={13} className="dev-row__icon" aria-hidden />
        Database
      </span>
      <span className="dev-row__state">
        <span>{`${database.engine} · ${SOURCES[database.source]}`}</span>
        {database.detail && <span className="dev-row__detail mono">{database.detail}</span>}
        {database.error && <span className="dev-row__error">{database.error}</span>}
      </span>
      <span className="dev-row__buttons">
        <button type="button" className="btn" aria-label="Open in DataBee" title="Open in DataBee" disabled={!!database.error} onClick={() => void openDatabase(project)}>
          Open
        </button>
      </span>
    </div>
  );
}

/** Every button keeps its slot: disabled, never removed. */
function ControlRow({ label, running, activity, state, site, project, start, stop, restart, canStart = true }: {
  label: string;
  running: boolean;
  activity: Activity | null;
  state: string;
  /** A server's row: its site's URL once it runs. */
  site?: { url: string | null };
  project: Project;
  start: DevAction;
  stop: DevAction;
  restart: DevAction;
  canStart?: boolean;
}) {
  const busy = activity !== null;
  return (
    <div className="dev-row">
      <span className="dev-row__name">
        {busy ? <LoaderCircle size={11} className="spin dev-row__icon" aria-hidden /> : <Dot on={running} />}
        {label}
      </span>
      <span className="dev-row__state">{busy ? ACTIVITY[activity] : state}</span>
      <span className="dev-row__buttons">
        {site && (
          <button
            type="button"
            className="icon-btn round-btn"
            aria-label="Open in Browser"
            title={site.url ?? undefined}
            disabled={!site.url}
            onClick={() => site.url && void api.openUrl(site.url)}
          >
            <Globe size={14} aria-hidden />
          </button>
        )}
        <button type="button" className="btn" disabled={busy || !running} onClick={() => void devAction(project, restart)}>
          Restart
        </button>
        <button type="button" className="btn dev-row__toggle" disabled={busy || (!running && !canStart)} onClick={() => void devAction(project, running ? stop : start)}>
          {running ? "Stop" : "Start"}
        </button>
      </span>
    </div>
  );
}

/** Docker, the project's DDEV or wp-env server and its database; nothing for a project without them. */
export function DevSection({ project, status }: { project: Project; status: DevStatus | undefined }) {
  if (!status || (!status.docker && !status.server && !status.database)) return null;
  const { docker, server, database } = status;
  const serverRunning = server?.state === "running";
  return (
    <section className="detail-section">
      <h2 className="detail-section__title">Dev Environment</h2>
      <div className="card">
        {docker && (
          <ControlRow
            label="Docker"
            running={docker.running}
            activity={docker.activity}
            state={docker.running ? `Running · ${docker.manager}` : `Stopped · ${docker.manager}`}
            project={project}
            start="dockerStart"
            stop="dockerStop"
            restart="dockerRestart"
          />
        )}
        {server && (
          <ControlRow
            label={server.label}
            running={serverRunning}
            activity={server.activity}
            state={serverRunning && server.url ? `Running · ${host(server.url)}` : serverState(server)}
            site={{ url: server.url }}
            project={project}
            start="serverStart"
            stop="serverStop"
            restart="serverRestart"
            canStart={server.state !== "notConfigured"}
          />
        )}
        {database && <DatabaseRow project={project} database={database} />}
      </div>
    </section>
  );
}
