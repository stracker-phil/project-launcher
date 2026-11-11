import { Check, History, Zap } from "lucide-react";
import type { Prefix } from "./prefixes";
import type { PaletteCommand } from "./sources";

/** The hint at the right, else while searching the command's section (results aren't grouped by section). */
export function CommandRow({ command, searching }: { command: PaletteCommand; searching: boolean }) {
  const meta = command.hint ?? (searching && command.mode === "commands" ? command.section : undefined);
  const Icon = command.checked ? Check : Zap;
  return (
    <>
      {command.icon ?? (
        <Icon size={16} strokeWidth={command.checked ? 2.2 : 1.8} aria-label={command.checked ? "On" : undefined} aria-hidden={command.checked ? undefined : true} />
      )}
      <span className="palette__summary">{command.label}</span>
      {meta && <span className="palette__meta">{meta}</span>}
      {command.shortcut && <kbd className="palette__shortcut">{command.shortcut}</kbd>}
    </>
  );
}

/** A search typed earlier that led to a command. */
export function RecentSearchRow({ text }: { text: string }) {
  return (
    <>
      <History size={16} strokeWidth={1.8} aria-hidden="true" />
      <span className="palette__summary">{text}</span>
    </>
  );
}

/** One button per prefix that has something to find; the active one clears it again. */
export function PrefixButtons({ prefixes, active, onPick }: { prefixes: Prefix[]; active: Prefix | null; onPick: (text: string) => void }) {
  return prefixes.map((prefix) => {
    const on = active?.char === prefix.char;
    return (
      <button
        key={prefix.char}
        type="button"
        tabIndex={-1}
        className={on ? "palette__prefix palette__prefix--on" : "palette__prefix"}
        aria-pressed={on}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => onPick(on ? "" : prefix.char)}
      >
        <kbd className="palette__prefix-key">{prefix.char}</kbd>
        {prefix.label}
      </button>
    );
  });
}
