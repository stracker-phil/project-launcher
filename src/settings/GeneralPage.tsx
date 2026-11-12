import { useId } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type AppSetting, type DockerManager } from "../api";
import { PopUpButton } from "../components/menu";
import { AUTOSTART_KEY, useAutostart } from "../queries/useAutostart";
import { useSettings } from "../queries/useSettings";
import { attempt } from "../state/attempt";
import { Group, Row, SwitchRow } from "./controls";
import { TextField, useFieldSave } from "./fields";

/** The debug app never installs a login item (ADR 0036). */
const RELEASE = import.meta.env.VITE_DEV_HOOKS !== "1";

interface AppFieldSpec {
  setting: AppSetting;
  label: string;
  description?: string;
  placeholder: string;
  mono?: boolean;
}

const APPS: AppFieldSpec[] = [
  { setting: "defaultEditor", label: "Default editor", description: "Edit Code opens projects with it, unless a project names its own.", placeholder: "Visual Studio Code or code" },
  { setting: "gitClient", label: "Git client", description: "Open Git Client opens the project folder with it.", placeholder: "Sublime Merge or smerge" },
  { setting: "terminalApp", label: "Terminal", description: "Terminal actions and Open Terminal Here use it.", placeholder: "Terminal" },
  { setting: "configEditor", label: "Project file editor", description: "Edit Project File… opens .project-launcher.json with it; empty uses the editor.", placeholder: "TextEdit" },
];

/** An app setting's row; saved when left or on ↩, Rust's refusal shown below the field. */
function AppField({ spec, value }: { spec: AppFieldSpec; value: string }) {
  const [error, commit] = useFieldSave((next) => api.setAppSetting(spec.setting, next), `Couldn’t change the ${spec.label.toLowerCase()}`);
  return (
    <TextField
      label={spec.label}
      description={spec.description}
      anchor={spec.setting}
      placeholder={spec.placeholder}
      mono={spec.mono}
      wide
      error={error}
      value={value}
      onCommit={commit}
    />
  );
}

const MANAGERS: { value: DockerManager; label: string }[] = [
  { value: "docker-desktop", label: "Docker Desktop" },
  { value: "orbstack", label: "OrbStack" },
  { value: "colima", label: "Colima" },
  { value: "custom", label: "Custom" },
];

const DOCKER_COMMANDS: AppFieldSpec[] = [
  { setting: "dockerStartCommand", label: "Start command", description: "Runs in a login shell.", placeholder: "colima start --cpu 4", mono: true },
  { setting: "dockerStopCommand", label: "Stop command", placeholder: "colima stop", mono: true },
];

function DockerGroup() {
  const { data: settings } = useSettings();
  const id = useId();
  const manager = settings?.dockerManager ?? "docker-desktop";
  return (
    <Group title="Dev Environment" anchor="docker">
      <Row label="Docker app" description="Starts and stops Docker for projects with a dev environment." htmlFor={id}>
        <PopUpButton
          id={id}
          value={manager}
          options={MANAGERS}
          disabled={!settings}
          onChange={(next) => void attempt(api.setDockerManager(next), "Couldn’t change the Docker app")}
        />
      </Row>
      {manager === "custom" && DOCKER_COMMANDS.map((spec) => <AppField key={spec.setting} spec={spec} value={settings?.[spec.setting] ?? ""} />)}
    </Group>
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
    <SwitchRow
      label="Open at login"
      description={RELEASE ? "Starts in the menu bar, without a window." : "Only the installed app opens at login."}
      anchor="login"
      checked={on}
      disabled={!RELEASE}
      onChange={change}
    />
  );
}

export function GeneralPage() {
  const { data: settings } = useSettings();
  return (
    <>
      <Group title="Apps">
        {APPS.map((spec) => (
          <AppField key={spec.setting} spec={spec} value={settings?.[spec.setting] ?? ""} />
        ))}
      </Group>
      <DockerGroup />
      <Group title="Startup">
        <OpenAtLogin />
      </Group>
    </>
  );
}
