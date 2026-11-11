export interface SegmentedControlProps<T extends string> {
  label: string;
  options: readonly T[];
  /** `null` presses none. */
  value: T | null;
  onChange: (value: T) => void;
  /** What a button reads; the capitalized option without one. */
  labels?: Partial<Record<T, string>>;
}

const capitalize = (s: string) => s[0].toUpperCase() + s.slice(1);

/** A row of mutually exclusive buttons; the pressed one is a raised tile (docs/ux.md → Settings). */
export function SegmentedControl<T extends string>({ label, options, value, onChange, labels }: SegmentedControlProps<T>) {
  return (
    <div className="segmented" role="group" aria-label={label}>
      {options.map((option) => (
        <button key={option} type="button" aria-pressed={value === option} onClick={() => onChange(option)}>
          {labels?.[option] ?? capitalize(option)}
        </button>
      ))}
    </div>
  );
}
