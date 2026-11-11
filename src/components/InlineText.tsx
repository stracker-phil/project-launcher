import { useEffect, useRef, useState, type ChangeEvent, type KeyboardEvent } from "react";

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
  const cancelled = useRef(false);

  const setEditing = (on: boolean) => {
    setOwnEditing(on);
    onEditingChange?.(on);
  };

  useEffect(() => {
    if (!isEditing) return;
    cancelled.current = false;
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
    if (!cancelled.current && draft.trimEnd() !== value) onSave(draft.trimEnd());
    setEditing(false);
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      cancelled.current = true;
      field.current?.blur();
    } else if (e.key === "Enter" && (!multiline || e.metaKey)) {
      e.preventDefault();
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
