import { toAppError, type CommandLine } from "../api";
import { useAppStatus } from "../queries/useAppStatus";
import { Group, Row, Value } from "./controls";

function commandLineText({ link, path }: CommandLine): string {
  switch (link) {
    case "installed":
      return `Installed at ${path}`;
    case "notInstalled":
      return `Not installed (${path})`;
    case "notApplicable":
      return "Not linked for debug builds";
    default:
      return link satisfies never;
  }
}

export function StatusPage() {
  const { data, error } = useAppStatus();
  if (error && !data) {
    return (
      <p className="settings-empty" role="alert">
        Could not read the status: {toAppError(error).message}
      </p>
    );
  }
  if (!data) return null;
  return (
    <>
      <Group title={data.name}>
        <Row label="Version" anchor="version">
          <Value>{data.version}</Value>
        </Row>
        <Row label="Build" anchor="build">
          <Value>{data.debug ? "Debug" : "Release"}</Value>
        </Row>
      </Group>
      <Group title="Folders">
        <Row label="Data folder" description="Settings and the projects list." anchor="data">
          <Value>{data.dataFolder}</Value>
        </Row>
        <Row label="Log folder" anchor="logs">
          <Value>{data.logFolder}</Value>
        </Row>
      </Group>
      <Group title="Command line">
        <Row label="Command" description="Lets the shell and other apps list and open projects." anchor="cli">
          <Value>{commandLineText(data.commandLine)}</Value>
        </Row>
      </Group>
    </>
  );
}
