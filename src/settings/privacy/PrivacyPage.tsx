import type { SettingsRow } from "../../registry/settingsPages";
import { Group, Row } from "../controls";
import { ShowDataFolder } from "./ShowDataFolder";

export const PRIVACY_ROWS: SettingsRow[] = [
  { id: "storage", title: "On this Mac", keywords: ["storage"] },
  { id: "kept-settings", title: "Settings and tags", keywords: ["shortcuts"] },
  { id: "kept-projects", title: "Project list", keywords: ["last opened"] },
  { id: "kept-view", title: "Window and sidebar" },
  { id: "kept-details", title: "Project details", keywords: [".project-launcher.json"] },
  { id: "data-folder", title: "Data folder" },
  { id: "network", title: "Network" },
  { id: "no-network", title: "No network code", keywords: ["internet", "links"] },
];

/** Settings → Privacy & Data: what Launcher keeps where (ADR 0038, ADR 0043), and that it never reaches the network
 *  (ADR 0016). */
export function PrivacyPage() {
  return (
    <>
      <Group title="On this Mac" anchor="storage">
        <Row label="Settings and tags" anchor="kept-settings" description="Your settings, shortcuts and the tag list, in Launcher’s data folder." />
        <Row label="Project list" anchor="kept-projects" description="Which project files Launcher knows and when you last opened each, in the same folder." />
        <Row label="Window and sidebar" anchor="kept-view" description="The main window’s size and place and the sidebar choice, in the same folder." />
        <Row label="Project details" anchor="kept-details" description="Everything else about a project lives in its own .project-launcher.json, beside its code." />
        <Row label="Data folder" anchor="data-folder">
          <ShowDataFolder />
        </Row>
      </Group>
      <Group title="Network" anchor="network">
        <Row label="No network code" anchor="no-network" description="Launcher has no network code; links open in your browser." />
      </Group>
    </>
  );
}
