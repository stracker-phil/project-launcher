import { useEffect, useRef, useState, type ChangeEvent, type KeyboardEvent } from "react";
import { restoreFocus } from "../lib/focus";

interface Props {
  value: string;
  /** Shown while the value is empty. */
  placeholder: string;
  label: string;
  onSave: (text: string) => void;
  /** Enter adds a line; ⌘↩ saves. */
  multiline?: boolean;
  /** Plain text that can't be edited; an empty value shows a dash. */
  readOnly?: boolean;
  /** Edit mode, when a menu or command asks for it. */
  editing?: boolean;
  onEditingChange?: (editing: boolean) => void;
  className?: string;
}

/** Text that turns into a field on click: Enter (⌘↩ when multiline) or leaving saves, Esc keeps the old value. */
export function InlineText({ value, placeholder, label, onSave, multiline, readOnly, editing, onEditingChange, className = "" }: Props) {
  const [ownEditing, setOwnEditing] = useState(false);
  const isEditing = !readOnly && (editing ?? ownEditing);
  const [draft, setDraft] = useState(value);
  const field = useRef<HTMLInputElement & HTMLTextAreaElement>(null); // one of the two, whichever renders
  /** How Enter or Escape ended the edit; `null` while typing, or when the field lost the keys. */
  const exit = useRef<"save" | "cancel" | null>(null);

  const setEditing = (on: boolean) => {
    setOwnEditing(on);
    onEditingChange?.(on);
  };

  useEffect(() => {
    if (!isEditing) return;
    exit.current = null;
    setDraft(value);
    requestAnimationFrame(() => {
      field.current?.focus();
      field.current?.select();
    });
    // Only entering edit mode takes the current value.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isEditing]);

  if (readOnly) {
    return <span className={`inline-text inline-text--read-only ${className}`}>{value || "—"}</span>;
  }

  if (!isEditing) {
    return (
      <button type="button" className={`inline-text ${value ? "" : "inline-text--empty"} ${className}`} aria-label={label} onClick={() => setEditing(true)}>
        {value || placeholder}
      </button>
    );
  }

  const finish = () => {
    // Switching apps or opening the palette blurs the field too; it stays open for the return.
    if (exit.current === null && !document.hasFocus()) return;
    if (exit.current !== "cancel" && draft.trimEnd() !== value) onSave(draft.trimEnd());
    setEditing(false);
    restoreFocus();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.nativeEvent.isComposing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      exit.current = "cancel";
      field.current?.blur();
    } else if (e.key === "Enter" && (!multiline || e.metaKey)) {
      e.preventDefault();
      exit.current = "save";
      field.current?.blur();
    }
  };
  const props = {
    ref: field,
    className: `inline-text inline-text--editing ${className}`,
    "aria-label": label,
    placeholder,
    value: draft,
    onChange: (e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => setDraft(e.target.value),
    onBlur: finish,
    onKeyDown,
  };
  return multiline ? <textarea rows={Math.max(2, draft.split("\n").length)} {...props} /> : <input {...props} />;
}
