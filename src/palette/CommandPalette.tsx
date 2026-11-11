import { useQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { api } from "../api";
import { useStore } from "../state/store";
import { CommandRow, PrefixButtons, RecentSearchRow } from "./PaletteRows";
import { parsePrefix } from "./prefixes";
import { recentSearches, rememberSearch } from "./recent";
import { paletteSections, SOURCED_PREFIXES, type PaletteItem } from "./sections";
import { collectCommands, snapshotPage } from "./sources";

const MENU_COMMANDS_KEY = ["menuCommands"];

/** ⌘K over the dimmed window: searches and runs the menus' commands, the page's own and the Settings pages. */
export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  return open ? <Palette /> : null;
}

const optionId = (index: number) => `palette-option-${index}`;
const itemKey = (item: PaletteItem) => (item.kind === "recent" ? `recent:${item.text}` : `command:${item.command.id}`);

/** On unmount, gives the focus back to `opener` unless a command has moved it elsewhere. */
function useRestoreFocus(opener: HTMLElement | null, panelRef: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const panel = panelRef.current;
    return () => {
      const focused = document.activeElement;
      const lost = !focused || focused === document.body || !!panel?.contains(focused);
      if (lost && opener?.isConnected) opener.focus({ preventScroll: true });
    };
  }, [opener, panelRef]);
}

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

function Palette() {
  const close = () => useStore.getState().setPaletteOpen(false);
  const [text, setText] = useState("");
  const [active, setActive] = useState(0);
  // Read before the field takes the focus: the page as it was, and where the focus goes back to.
  const [page] = useState(snapshotPage);
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  const [recent] = useState(recentSearches);
  const { data: menu } = useQuery({ queryKey: MENU_COMMANDS_KEY, queryFn: api.menuCommands, refetchOnMount: "always" });
  const commands = useMemo(() => collectCommands(menu ?? [], page), [menu, page]);
  const panelRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const { prefix, query } = parsePrefix(text);
  const sections = paletteSections(commands, recent, text);
  const all = sections.flatMap((section) => section.items);
  const indexOf = new Map(all.map((item, at) => [item, at]));

  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: "nearest" });
  }, [active]);
  useRestoreFocus(opener, panelRef);

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
    close();
    item.command.run();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const handled = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (e.key === "Escape") {
      handled();
      close();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      handled();
      if (all.length > 0) setActive((at) => (at + (e.key === "ArrowDown" ? 1 : -1) + all.length) % all.length);
    } else if (e.key === "Enter") {
      handled();
      run(all[active]);
    }
  };

  const placeholder = prefix?.placeholder ?? "Search commands";
  const activeItem = all[active];

  return (
    <div className="palette-scrim" onMouseDown={(e) => e.target === e.currentTarget && close()}>
      <div ref={panelRef} className="palette" role="dialog" aria-label="Command palette" onKeyDown={onKeyDown}>
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
              {section.items.map((item) => {
                const index = indexOf.get(item) ?? -1;
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
          <PrefixButtons prefixes={SOURCED_PREFIXES} active={prefix} onPick={narrow} />
          <span className="palette__spacer" />
          <span>{activeItem?.kind === "recent" ? "↩ search" : "↩ run"}</span>
          <span>esc close</span>
        </footer>
      </div>
    </div>
  );
}
