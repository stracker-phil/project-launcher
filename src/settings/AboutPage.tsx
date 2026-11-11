import { api, toAppError } from "../api";
import { useAppStatus } from "../queries/useAppStatus";
import { useStore } from "../state/store";
import { Group, Row, Value } from "./controls";

function run(command: string, failure: string) {
  api.runCommand(command).catch((e: unknown) => useStore.getState().toast("error", `${failure}: ${toAppError(e).message}`));
}

/** Settings → About: the version, and the folders a bug report looks at. */
export function AboutPage() {
  const { data } = useAppStatus();
  return (
    <Group title="About">
      <Row label={data?.name ?? "Launcher"}>
        <Value>{data ? `Version ${data.version}` : ""}</Value>
      </Row>
      <Row label="Data" description="Settings and the projects list.">
        <button type="button" className="btn" onClick={() => run("app:data", "Could not show the data folder")}>
          Show Data Folder
        </button>
      </Row>
      <Row label="Logs" description="What Launcher did and what went wrong, for a bug report.">
        <button type="button" className="btn" onClick={() => run("app:logs", "Could not open the log folder")}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}
