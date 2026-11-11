import type { ReactNode } from "react";
import { LoaderCircle } from "lucide-react";
import { api, type Activity, type DatabaseInfo, type DevAction, type DevStatus, type Project, type ServerState, type ServerStatus } from "../../api";
import { devAction, openDatabase } from "../devEnvironment";

const ACTIVITY: Record<Activity, string> = { starting: "Starting…", stopping: "Stopping…", restarting: "Restarting…" };
const STATES: Record<ServerState, string> = { running: "Running", paused: "Paused", stopped: "Stopped", notConfigured: "Not set up", unknown: "Unknown" };

/** A server that isn't set up names what is missing. */
const serverState = (server: ServerStatus) => (server.state === "notConfigured" && server.detail) || STATES[server.state];

function UrlButton({ url }: { url: string }) {
  return (
    <button type="button" className="link-btn dev-row__url" onClick={() => void api.openUrl(url)}>
      {url}
    </button>
  );
}

function DatabaseRow({ project, database }: { project: Project; database: DatabaseInfo }) {
  return (
    <div className="dev-row">
      <span className="dev-row__label">Database</span>
      <span className="dev-row__state">
        <span>{database.engine}</span>
        {database.detail && <span className="dev-row__detail mono">{database.detail}</span>}
        {database.error && <span className="dev-row__error">{database.error}</span>}
      </span>
      <span className="dev-row__buttons">
        <button type="button" className="btn" disabled={!!database.error} onClick={() => void openDatabase(project)}>
          Open in DataBee
        </button>
      </span>
    </div>
  );
}

function Dot({ on }: { on: boolean }) {
  return <span className={on ? "dev-dot dev-dot--on" : "dev-dot"} aria-hidden />;
}

/** A row with its state on the left and Start/Stop and Restart on the right; both buttons keep their slot. */
function ControlRow({ label, running, activity, state, detail, project, start, stop, restart, canStart = true }: {
  label: string;
  running: boolean;
  activity: Activity | null;
  state: string;
  detail?: ReactNode;
  project: Project;
  start: DevAction;
  stop: DevAction;
  restart: DevAction;
  canStart?: boolean;
}) {
  const busy = activity !== null;
  return (
    <div className="dev-row">
      <span className="dev-row__label">{label}</span>
      <span className="dev-row__state">
        {busy ? <LoaderCircle size={12} className="spin" aria-hidden /> : <Dot on={running} />}
        <span>{busy ? ACTIVITY[activity] : state}</span>
        {detail}
      </span>
      <span className="dev-row__buttons">
        <button type="button" className="btn dev-row__toggle" disabled={busy || (!running && !canStart)} onClick={() => void devAction(project, running ? stop : start)}>
          {running ? "Stop" : "Start"}
        </button>
        <button type="button" className="btn" disabled={busy || !running} onClick={() => void devAction(project, restart)}>
          Restart
        </button>
      </span>
    </div>
  );
}

/** Docker, the project's DDEV or wp-env server and its database; nothing for a project without them. */
export function DevSection({ project, status }: { project: Project; status: DevStatus | undefined }) {
  if (!status || (!status.docker && !status.server && !status.database)) return null;
  const { docker, server, database } = status;
  return (
    <section className="detail-section">
      <h2 className="detail-section__title">Dev Environment</h2>
      <div className="dev-list">
        {docker && (
          <ControlRow
            label={docker.manager}
            running={docker.running}
            activity={docker.activity}
            state={docker.running ? "Running" : "Stopped"}
            project={project}
            start="dockerStart"
            stop="dockerStop"
            restart="dockerRestart"
          />
        )}
        {server && (
          <ControlRow
            label={server.label}
            running={server.state === "running"}
            activity={server.activity}
            state={serverState(server)}
            detail={server.url && <UrlButton url={server.url} />}
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
