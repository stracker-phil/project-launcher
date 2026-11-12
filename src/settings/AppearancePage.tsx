import { api, type Appearance } from "../api";
import { useSettings } from "../queries/useSettings";
import { attempt } from "../state/attempt";
import { AccentSwatches } from "./AccentSwatches";
import { Group, Row } from "./controls";

const MODES: { id: Appearance; label: string }[] = [
  { id: "system", label: "Automatic" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

/** A miniature window in one palette: the dark sidebar beside the list and the detail. */
function MiniWindow({ scheme, className = "" }: { scheme: "light" | "dark"; className?: string }) {
  return (
    <span className={`scheme-look scheme-tile-window ${className}`} data-scheme={scheme}>
      <span className="scheme-tile-sidebar">
        <span className="scheme-tile-entry" />
      </span>
      <span className="scheme-tile-content">
        <span className="scheme-tile-block" />
      </span>
    </span>
  );
}

/** One tile per mode, ringed when chosen; Automatic is split into its light and dark halves. */
function ModePicker({ value, onChange }: { value: Appearance | null; onChange: (mode: Appearance) => void }) {
  return (
    <div className="scheme-picker" role="group" aria-label="Mode">
      {MODES.map(({ id, label }) => (
        <button key={id} type="button" className="scheme-option" aria-pressed={value === id} onClick={() => onChange(id)}>
          <span className="scheme-tile" aria-hidden="true">
            {id === "dark" ? <MiniWindow scheme="dark" /> : <MiniWindow scheme="light" />}
            {id === "system" && <MiniWindow scheme="dark" className="scheme-tile-half" />}
          </span>
          {label}
        </button>
      ))}
    </div>
  );
}

/** Light, dark or following macOS (ADR 0026), and the accent (ADR 0042), for every Launcher window. */
export function AppearancePage() {
  const { data: settings } = useSettings();
  return (
    <Group>
      <Row label="Mode" description="Light, dark, or Automatic, which follows macOS." stacked>
        <ModePicker value={settings?.appearance ?? null} onChange={(appearance) => void attempt(api.setAppearance(appearance), "Could not change the appearance")} />
      </Row>
      <Row label="Accent color" description="Selection, focus, switches and buttons. System follows macOS." anchor="accent">
        <AccentSwatches value={settings?.accent ?? null} onChange={(accent) => void attempt(api.setAccent(accent), "Could not change the accent color")} />
      </Row>
    </Group>
  );
}
