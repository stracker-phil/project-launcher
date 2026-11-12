import { Group, Row } from "../controls";
import { ShowDataFolder } from "./ShowDataFolder";

/** Settings → Privacy & Data: what Launcher keeps where (ADR 0038, ADR 0043), and that it never reaches the network
 *  (ADR 0016). */
export function PrivacyPage() {
  return (
    <>
      <Group title="On this Mac" anchor="storage">
        <Row label="Settings and tags" description="Your settings, shortcuts and the tag list, in Launcher’s data folder." />
        <Row label="Project list" description="Which project files Launcher knows and when you last opened each, in the same folder." />
        <Row label="Window and sidebar" description="The main window’s size and place and the sidebar choice, in the same folder." />
        <Row label="Project details" description="Everything else about a project lives in its own .project-launcher.json, beside its code." />
        <Row label="Data folder" anchor="data-folder">
          <ShowDataFolder />
        </Row>
      </Group>
      <Group title="Network" anchor="network">
        <Row label="No network code" description="Launcher has no network code; links open in your browser." />
      </Group>
    </>
  );
}
