import { useEffect, useState, type KeyboardEvent } from "react";
import { api } from "../../api";
import { bindingFromEvent, bindingLabel } from "../../commands/registry";
import { attempt } from "../../state/attempt";

interface Props {
  value: string | null;
  /** The command's title, for assistive tech. */
  name: string;
  onRecord: (binding: string) => void;
  onUnbind: () => void;
}

/** A click records the next key press: ⌫ leaves the command without a key, ⎋ cancels. The menu's keys are off
 *  meanwhile, so they reach the recorder. */
export function ShortcutRecorder({ value, name, onRecord, onUnbind }: Props) {
  const [recording, setRecording] = useState(false);
  useEffect(() => {
    if (!recording) return;
    void attempt(api.pauseMenuKeys(true), "Could not pause the menu's shortcuts");
    return () => void attempt(api.pauseMenuKeys(false), "Could not restore the menu's shortcuts");
  }, [recording]);
  /** Stops recording, then hands on what was pressed, if anything. */
  const end = (outcome?: () => void) => {
    setRecording(false);
    outcome?.();
  };
  const onRecordingKey = (e: KeyboardEvent<HTMLButtonElement>) => {
    // Tab moves the focus on, which ends the recording.
    if (!recording || e.key === "Tab") return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      end();
    } else if (e.key === "Backspace" || e.key === "Delete") {
      end(onUnbind);
    } else {
      const binding = bindingFromEvent(e);
      if (binding) end(() => onRecord(binding));
    }
  };
  const className = ["shortcut-field", recording && "recording", !value && !recording && "unbound"].filter(Boolean).join(" ");
  return (
    <button
      type="button"
      className={className}
      data-recording={recording || undefined}
      aria-label={`${recording ? "Press the new shortcut" : "Record shortcut"} for ${name}`}
      onClick={(e) => {
        e.currentTarget.focus();
        setRecording(true);
      }}
      onBlur={() => end()}
      onKeyDown={onRecordingKey}
    >
      {recording ? "Press keys…" : value ? bindingLabel(value) : "None"}
    </button>
  );
}
