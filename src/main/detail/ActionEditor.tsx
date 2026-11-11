import { useState, type CSSProperties, type FormEvent, type RefObject } from "react";
import { Check } from "lucide-react";
import { toAppError, type ActionDraft, type ActionSource, type Builtin, type Project, type RunsAs } from "../../api";
import { Popover } from "../../components/Popover";
import { SegmentedControl } from "../../components/SegmentedControl";
import { colorVar, iconFor, PROJECT_COLORS, PROJECT_ICONS } from "../../lib/icons";
import { ShortcutRecorder } from "../../settings/shortcuts/ShortcutRecorder";
import { saveAction } from "./actionEdits";

/** What the editor works on: an action at `source`, or a new one (`source` null). */
export interface EditorTarget {
  source: ActionSource | null;
  draft: ActionDraft;
  preset: Builtin | null;
  /** A built-in's values; its empty fields fall back to these. */
  defaults: ActionDraft | null;
}

const RUNS_AS: readonly RunsAs[] = ["app", "url", "terminal", "script"];
const RUNS_AS_LABELS: Record<RunsAs, string> = { app: "App", url: "Link", terminal: "Terminal", script: "Script" };
const HINTS: Record<RunsAs, string> = {
  app: "Opens the project folder, or the arguments, with an app or a command-line tool.",
  url: "Opens a link in its default app.",
  terminal: "Opens a terminal in the project folder that stays open.",
  script: "Runs in the background; the output shows under Scripts.",
};
const FALLBACK_ICON: Record<RunsAs, string> = { app: "Window", url: "Globe", terminal: "Terminal", script: "Terminal" };
const COMMAND_PLACEHOLDER: Partial<Record<RunsAs, string>> = { terminal: "Optional: empty opens a shell", script: "npm run build" };

type TextField = "label" | "app" | "args" | "url" | "command";
type TextProps = (field: TextField, fallback: string) => {
  value: string;
  placeholder: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
};

/** A built-in with any field of its own set. */
const isOverridden = (draft: ActionDraft) => Object.entries(draft).some(([key, value]) => key !== "runsAs" && !!value);

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="action-editor__row">
      <span className="action-editor__label">{label}</span>
      {children}
    </label>
  );
}

function LookPicker({ icon, color, onChange }: { icon: string; color: string; onChange: (patch: Partial<ActionDraft>) => void }) {
  return (
    <div className="action-editor__look">
      <div className="icon-picker__colors">
        {PROJECT_COLORS.map((name) => (
          <button key={name} type="button" className="icon-picker__swatch" style={{ background: colorVar(name) }} aria-label={name} aria-pressed={name === color} onClick={() => onChange({ color: name })}>
            {name === color && <Check size={14} aria-hidden />}
          </button>
        ))}
      </div>
      <div className="icon-picker__icons" style={{ "--tile-color": colorVar(color) } as CSSProperties}>
        {PROJECT_ICONS.map((name) => {
          const Icon = iconFor(name);
          return (
            <button key={name} type="button" className="icon-picker__icon" aria-label={name} title={name} aria-pressed={name === icon} onClick={() => onChange({ icon: name })}>
              <Icon size={18} aria-hidden />
            </button>
          );
        })}
      </div>
    </div>
  );
}

/** The fields of the action's kind. */
function KindFields({ runsAs, text }: { runsAs: RunsAs; text: TextProps }) {
  if (runsAs === "app") {
    return (
      <>
        <Field label="App"><input className="field" {...text("app", "Zed, zed or code")} /></Field>
        <Field label="Arguments"><input className="field" {...text("args", "${dir}")} /></Field>
      </>
    );
  }
  if (runsAs === "url") return <Field label="URL"><input className="field" {...text("url", "https://… or ${url}")} /></Field>;
  return <Field label="Command"><input className="field mono" {...text("command", COMMAND_PLACEHOLDER[runsAs] ?? "")} /></Field>;
}

function EditorForm({ project, target, onClose }: { project: Project; target: EditorTarget; onClose: () => void }) {
  const [draft, setDraft] = useState(target.draft);
  const [lookOpen, setLookOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const defaults = target.defaults;
  const set = (patch: Partial<ActionDraft>) => setDraft((d) => ({ ...d, ...patch }));
  const text: TextProps = (field, fallback) => ({
    value: draft[field] ?? "",
    placeholder: defaults?.[field] || fallback,
    onChange: (e: React.ChangeEvent<HTMLInputElement>) => set({ [field]: e.target.value }),
  });
  const icon = draft.icon || defaults?.icon || FALLBACK_ICON[draft.runsAs];
  const color = draft.color || defaults?.color || "Blue";
  const Icon = iconFor(icon);
  const overridden = target.preset !== null && isOverridden(draft);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    try {
      await saveAction(project, target.source, draft, target.preset);
      onClose();
    } catch (err) {
      setError(toAppError(err).message);
    }
  };

  return (
    <form className="action-editor" onSubmit={(e) => void submit(e)}>
      <div className="action-editor__head">
        <button type="button" className="action-editor__tile" style={{ "--tile-color": colorVar(color) } as CSSProperties} aria-label="Icon and Color" aria-expanded={lookOpen} onClick={() => setLookOpen((o) => !o)}>
          <Icon size={16} aria-hidden />
        </button>
        <input autoFocus className="field" aria-label="Title" {...text("label", "Title")} />
      </div>
      {lookOpen && <LookPicker icon={icon} color={color} onChange={set} />}
      {target.preset === null ? (
        <div className="action-editor__kind">
          <SegmentedControl label="Runs as" options={RUNS_AS} labels={RUNS_AS_LABELS} value={draft.runsAs} onChange={(runsAs) => set({ runsAs })} />
          <p className="action-editor__hint">{HINTS[draft.runsAs]}</p>
        </div>
      ) : (
        <p className="action-editor__hint">A built-in: empty fields use the default shown in gray.</p>
      )}
      <KindFields runsAs={draft.runsAs} text={text} />
      <div className="action-editor__row">
        <span className="action-editor__label">Shortcut</span>
        <ShortcutRecorder
          value={draft.shortcut ?? defaults?.shortcut ?? null}
          name={draft.label || defaults?.label || "the action"}
          onRecord={(shortcut) => set({ shortcut })}
          onUnbind={() => set({ shortcut: undefined })}
        />
      </div>
      <p className="action-editor__hint">{"${dir} is the project folder, ${url} its website."}</p>
      {error && <p className="action-editor__error" role="alert">{error}</p>}
      <div className="action-editor__buttons">
        {overridden && (
          <button type="button" className="btn" onClick={() => setDraft({ runsAs: draft.runsAs, label: "" })}>
            Reset to Default
          </button>
        )}
        <span className="action-editor__spacer" />
        <button type="button" className="btn" onClick={onClose}>Cancel</button>
        <button type="submit" className="btn btn-primary">{target.source ? "Save" : "Add"}</button>
      </div>
    </form>
  );
}

/** The editor under `anchor`; it starts from the target's values each time it opens. */
export function ActionEditor({ project, target, anchor, onClose }: {
  project: Project;
  target: EditorTarget | null;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}) {
  return (
    <Popover open={target !== null} anchor={anchor} label={target?.source ? "Edit Action" : "New Action"} onClose={onClose}>
      {target && <EditorForm key={target.source ? `${target.source.list}:${target.source.index}` : "new"} project={project} target={target} onClose={onClose} />}
    </Popover>
  );
}
