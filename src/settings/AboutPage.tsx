import { api } from "../api";
import { useAppStatus } from "../queries/useAppStatus";
import { attempt } from "../state/attempt";
import { Group, Row, Value } from "./controls";
import { ShowDataFolder } from "./privacy/ShowDataFolder";

/** Settings → About: the version, and the folders a bug report looks at. */
export function AboutPage() {
  const { data } = useAppStatus();
  return (
    <Group>
      <Row label={data?.name ?? "Launcher"}>
        <Value>{data ? `Version ${data.version}` : ""}</Value>
      </Row>
      <Row label="Data" description="Settings and the projects list.">
        <ShowDataFolder />
      </Row>
      <Row label="Logs" description="What Launcher did and what went wrong, for a bug report.">
        <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:logs"), "Could not open the log folder")}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}
