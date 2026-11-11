import { bindingKeys } from "../commands/registry";

/** A shortcut as separate keycaps; nothing without one. */
export function Keycaps({ binding, className = "" }: { binding: string | null; className?: string }) {
  const keys = bindingKeys(binding);
  if (keys.length === 0) return null;
  return (
    <span className={`keycaps ${className}`} aria-label={keys.join("")}>
      {keys.map((key, i) => (
        <kbd key={i} className="keycap" aria-hidden>
          {key}
        </kbd>
      ))}
    </span>
  );
}
