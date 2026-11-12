import type { Accent } from "../api";

/** The named hues, in the order macOS lists its accent colors; each is `--accent-hue-<name>` in the palettes. */
const HUES = ["orange", "gold", "red", "magenta", "purple", "indigo", "blue", "teal", "graphite"] as const satisfies readonly Accent[];

const capitalize = (s: string) => s[0].toUpperCase() + s.slice(1);

/** One swatch per accent, after System (the macOS accent color); the chosen one is ringed. */
export function AccentSwatches({ value, onChange }: { value: Accent | null; onChange: (accent: Accent) => void }) {
  return (
    <div className="accent-swatches" role="group" aria-label="Accent color">
      <button type="button" className="accent-system" aria-label="System" aria-pressed={value === "system"} title="System" onClick={() => onChange("system")} />
      {HUES.map((hue) => (
        <button
          key={hue}
          type="button"
          aria-label={capitalize(hue)}
          aria-pressed={value === hue}
          title={capitalize(hue)}
          style={{ background: `var(--accent-hue-${hue})` }}
          onClick={() => onChange(hue)}
        />
      ))}
    </div>
  );
}
