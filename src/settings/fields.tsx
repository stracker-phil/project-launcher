/** Text fields for settings rows, each showing the error Rust gave for its field below it. */
import { useEffect, useId, useState, type InputHTMLAttributes, type ReactNode } from "react";
import { toAppError } from "../api";
import { useStore } from "../state/store";
import { Row } from "./controls";

/** Sends a field's value; a refusal naming a field (`InvalidField`) becomes the field's error, anything else a
 *  toast with `failure` first. The error clears once a value goes through. */
export function useFieldSave(save: (value: string) => Promise<unknown>, failure: string): [string | null, (value: string) => void] {
  const [error, setError] = useState<string | null>(null);
  const commit = (value: string) => {
    save(value).then(
      () => setError(null),
      (e: unknown) => {
        const refused = toAppError(e);
        if (refused.field) setError(refused.field.message);
        else useStore.getState().toast("error", `${failure}: ${refused.message}`);
      },
    );
  };
  return [error, commit];
}

type BufferedInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value"> & {
  value: string;
  onCommit: (value: string) => void;
};

/** An input that commits on blur or ↩, so half-typed values never reach Rust. */
function BufferedInput({ value, onCommit, ...input }: BufferedInputProps) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    if (text !== value) onCommit(text);
  };
  return (
    <input
      {...input}
      type="text"
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && commit()}
    />
  );
}

function FieldError({ id, message }: { id: string; message?: string | null }) {
  if (!message) return null;
  return (
    <span id={id} className="settings-field-error">
      {message}
    </span>
  );
}

const inputClass = (wide?: boolean, mono?: boolean) => ["input", wide && "wide", mono && "mono"].filter(Boolean).join(" ");

interface TextFieldProps {
  label: string;
  description?: ReactNode;
  anchor?: string;
  placeholder?: string;
  /** Room for a command. */
  wide?: boolean;
  /** Machine text, e.g. a command. */
  mono?: boolean;
  /** Rust's message for this field. */
  error?: string | null;
  value: string;
  onCommit: (value: string) => void;
}

/** A row with an input, saved when left or on ↩, and Rust's error below it. */
export function TextField({ label, description, anchor, placeholder, wide, mono, error, value, onCommit }: TextFieldProps) {
  const id = useId();
  const errorId = `${id}-error`;
  return (
    <Row label={label} description={description} anchor={anchor} htmlFor={id}>
      <div className="settings-field">
        <div className="settings-input">
          <BufferedInput
            id={id}
            className={inputClass(wide, mono)}
            placeholder={placeholder}
            spellCheck={false}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            value={value}
            onCommit={onCommit}
          />
        </div>
        <FieldError id={errorId} message={error} />
      </div>
    </Row>
  );
}
