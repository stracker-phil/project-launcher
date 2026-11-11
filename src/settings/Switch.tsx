/** An on/off switch. */
export function Switch({ label, checked, disabled, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void }) {
  return (
    <button type="button" role="switch" className="switch" aria-label={label} aria-checked={checked} disabled={disabled} onClick={() => onChange(!checked)}>
      <span className="switch__knob" aria-hidden />
    </button>
  );
}
