/** An on/off switch: a checkbox drawn as a track with a knob. Named by `label`, or by a `<label htmlFor={id}>`. */
export function Switch({ id, label, checked, disabled, onChange }: { id?: string; label?: string; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void }) {
  return (
    <input
      id={id}
      className="switch"
      type="checkbox"
      role="switch"
      aria-label={label}
      checked={checked}
      disabled={disabled}
      onChange={(e) => onChange(e.target.checked)}
    />
  );
}
