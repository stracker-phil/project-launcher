import { Search } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { CommandRow, PrefixButtons, RecentSearchRow } from "./PaletteRows";
import { parsePrefix } from "./prefixes";
import { recentSearches, rememberSearch } from "./recent";
import { paletteSections, sourcedPrefixes, type PaletteItem } from "./sections";
import type { PaletteCommand } from "./sources";

const optionId = (index: number) => `palette-option-${index}`;
const itemKey = (item: PaletteItem) => (item.kind === "recent" ? `recent:${item.text}` : `command:${item.command.id}`);

interface PaletteOptionProps {
  item: PaletteItem;
  index: number;
  active: boolean;
  searching: boolean;
  onHover: (index: number) => void;
  onRun: (item: PaletteItem) => void;
}

function PaletteOption({ item, index, active, searching, onHover, onRun }: PaletteOptionProps) {
  return (
    <div
      id={optionId(index)}
      role="option"
      aria-selected={active}
      className={active ? "palette__option palette__option--active" : "palette__option"}
      onMouseMove={() => onHover(index)}
      onClick={() => onRun(item)}
    >
      {item.kind === "recent" ? <RecentSearchRow text={item.text} /> : <CommandRow command={item.command} searching={searching} />}
    </div>
  );
}

/** The palette's search field, sections and footer. ⎋ and ⌘K close it; a chosen row closes it, then runs. Mounted
 *  fresh for each opening. */
export function PalettePanel({ commands, onClose }: { commands: PaletteCommand[]; onClose: () => void }) {
  const [text, setText] = useState("");
  const [active, setActive] = useState(0);
  const [recent] = useState(recentSearches);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const { prefix, query } = parsePrefix(text);
  const sections = paletteSections(commands, recent, text);
  const all = sections.flatMap((section) => section.items);
  // Where each section's rows start in `all`.
  const starts = sections.map((_, i) => sections.slice(0, i).reduce((sum, section) => sum + section.items.length, 0));

  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const changeText = (next: string) => {
    setText(next);
    setActive(0);
  };
  const narrow = (next: string) => {
    changeText(next);
    inputRef.current?.focus();
  };

  const run = (item: PaletteItem | undefined) => {
    if (!item) return;
    if (item.kind === "recent") return narrow(item.text);
    if (query) rememberSearch(text);
    onClose();
    item.command.run();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const handled = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (e.key === "Escape" || (e.metaKey && e.key.toLowerCase() === "k")) {
      handled();
      onClose();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      handled();
      if (all.length > 0) setActive((at) => (at + (e.key === "ArrowDown" ? 1 : -1) + all.length) % all.length);
    } else if (e.key === "Enter") {
      handled();
      run(all[active]);
    }
  };

  const placeholder = prefix?.placeholder ?? "Search projects, library, actions and commands";
  const activeItem = all[active];

  return (
    <div className="palette" role="dialog" aria-label="Command palette" onKeyDown={onKeyDown}>
      <label className="palette__input">
        <Search size={18} strokeWidth={2} aria-hidden="true" />
        <input
          ref={inputRef}
          autoFocus
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-results"
          aria-activedescendant={activeItem ? optionId(active) : undefined}
          aria-label={placeholder}
          placeholder={placeholder}
          value={text}
          onChange={(e) => changeText(e.target.value)}
        />
      </label>
      <div ref={listRef} id="palette-results" className="palette__results scroller" role="listbox" aria-label="Results">
        {sections.map((section, i) => (
          <section key={section.heading} aria-label={section.heading}>
            {i > 0 && <hr className="palette__separator" />}
            <h2 className="palette__caps">{section.heading}</h2>
            {section.items.map((item, at) => {
              const index = starts[i] + at;
              return (
                <PaletteOption
                  key={itemKey(item)}
                  item={item}
                  index={index}
                  active={index === active}
                  searching={query !== ""}
                  onHover={setActive}
                  onRun={run}
                />
              );
            })}
          </section>
        ))}
        {all.length === 0 && text.trim() !== "" && <p className="palette__empty">No results</p>}
      </div>
      <footer className="palette__footer">
        <PrefixButtons prefixes={sourcedPrefixes(commands)} active={prefix} onPick={narrow} />
        <span className="palette__spacer" />
        <span>{activeItem?.kind === "recent" ? "↩ search" : "↩ run"}</span>
        <span>esc close</span>
      </footer>
    </div>
  );
}
