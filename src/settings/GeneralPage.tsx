import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type AppSetting } from "../api";
import { AUTOSTART_KEY, useAutostart } from "../queries/useAutostart";
import { useSettings } from "../queries/useSettings";
import { attempt } from "../state/attempt";
import { Group, Row } from "./controls";
import { Switch } from "./Switch";

/** The debug app never installs a login item (ADR 0036). */
const RELEASE = import.meta.env.VITE_DEV_HOOKS !== "1";

const APPS: { setting: AppSetting; label: string; description: string; placeholder: string }[] = [
  { setting: "defaultEditor", label: "Default editor", description: "Edit Code opens projects with it, unless a project names its own.", placeholder: "Visual Studio Code or code" },
  { setting: "gitClient", label: "Git client", description: "Open Git Client opens the project folder with it.", placeholder: "Sublime Merge or smerge" },
  { setting: "terminalApp", label: "Terminal", description: "Terminal actions and Open Terminal Here use it.", placeholder: "Terminal" },
  { setting: "configEditor", label: "Project file editor", description: "Edit Project File… opens .project-launcher.json with it; empty uses the editor.", placeholder: "TextEdit" },
];

/** An app name; saved when the field is left or ↩ is pressed. */
function AppField({ setting, label, value, placeholder }: { setting: AppSetting; label: string; value: string; placeholder: string }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const save = () => {
    if (draft.trim() !== value) void attempt(api.setAppSetting(setting, draft), `Couldn’t change the ${label.toLowerCase()}`);
  };
  return (
    <input
      className="field settings-app-field"
      aria-label={label}
      placeholder={placeholder}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={save}
      onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
    />
  );
}

function OpenAtLogin() {
  const client = useQueryClient();
  const { data: on = false } = useAutostart(RELEASE);
  const change = (next: boolean) => {
    void attempt(
      api.setAutostart(next).then((now) => client.setQueryData(AUTOSTART_KEY, now)),
      "Couldn’t change Open at Login",
    );
  };
  return (
    <Row label="Open at login" description={RELEASE ? "Starts in the menu bar, without a window." : "Only the installed app opens at login."} anchor="login">
      <Switch label="Open at login" checked={on} disabled={!RELEASE} onChange={change} />
    </Row>
  );
}

export function GeneralPage() {
  const { data: settings } = useSettings();
  return (
    <>
      <Group title="Apps">
        {APPS.map((app) => (
          <Row key={app.setting} label={app.label} description={app.description} anchor={app.setting}>
            <AppField setting={app.setting} label={app.label} value={settings?.[app.setting] ?? ""} placeholder={app.placeholder} />
          </Row>
        ))}
      </Group>
      <Group title="Startup">
        <OpenAtLogin />
      </Group>
    </>
  );
}
