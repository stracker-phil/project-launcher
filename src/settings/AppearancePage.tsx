import { api, type Appearance } from "../api";
import { useSettings } from "../queries/useSettings";
import { attempt } from "../state/attempt";
import { Group, Segmented } from "./controls";

const OPTIONS = ["system", "light", "dark"] as const satisfies readonly Appearance[];

/** Light, dark or following macOS, for every Launcher window (ADR 0026). */
export function AppearancePage() {
  const { data: settings } = useSettings();
  return (
    <Group title="Appearance">
      <Segmented
        label="Appearance"
        description="Automatic follows macOS."
        options={OPTIONS}
        labels={{ system: "Automatic" }}
        value={settings?.appearance ?? null}
        onChange={(appearance) => void attempt(api.setAppearance(appearance), "Could not change the appearance")}
      />
    </Group>
  );
}
