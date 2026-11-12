import { api } from "../api";
import { useAppStatus } from "../queries/useAppStatus";
import type { SettingsRow } from "../registry/settingsPages";
import { attempt } from "../state/attempt";
import { Group, Row, Value } from "./controls";
import { ShowDataFolder } from "./privacy/ShowDataFolder";

export const ABOUT_ROWS: SettingsRow[] = [
  { id: "about-version", title: "Version" },
  { id: "about-data", title: "Data", keywords: ["data folder"] },
  { id: "about-logs", title: "Logs", keywords: ["log folder", "bug report"] },
];

/** Settings → About: the version, and the folders a bug report looks at. */
export function AboutPage() {
  const { data } = useAppStatus();
  return (
    <Group>
      <Row label="Version" anchor="about-version">
        <Value>{data?.version ?? ""}</Value>
      </Row>
      <Row label="Data" description="Settings and the projects list." anchor="about-data">
        <ShowDataFolder />
      </Row>
      <Row label="Logs" description="What Launcher did and what went wrong, for a bug report." anchor="about-logs">
        <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:logs"), "Could not open the log folder")}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}
