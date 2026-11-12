import { useMemo, useState } from "react";
import { findRow, resolveSettingsPage, searchSettings, settingsGroups, useSettingsPagesVersion, type SettingsMatch, type SettingsPage } from "../registry/settingsPages";
import { ScrollTarget, type RevealTarget } from "./controls";

/** What Settings is asked to show: a page or row id. A new object per request, so asking again reveals again. */
export interface SettingsRequest {
  id?: string;
}

const targetOf = (match: SettingsMatch) => match.row?.id ?? match.page.id;

function NavButton({ page, row, current, onOpen }: SettingsMatch & { current: string; onOpen: (id: string) => void }) {
  const { icon: Icon } = page;
  return (
    <button
      type="button"
      aria-label={row ? `${row.title}, ${page.title}` : undefined}
      aria-current={!row && current === page.id ? "page" : undefined}
      onClick={() => onOpen(row?.id ?? page.id)}
    >
      <Icon size={14} aria-hidden="true" />
      <span className="settings-nav-text">
        <span className="settings-nav-title">{page.title}</span>
        {row && <span className="settings-nav-setting">{row.title}</span>}
      </span>
    </button>
  );
}

/** The page list under its group headings; a search replaces it with every matching page and row, best first. */
function SettingsNav({ current, onOpen }: { current: string; onOpen: (id: string) => void }) {
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const results = searching ? searchSettings(query) : [];
  return (
    <nav className="settings-nav" aria-label="Settings pages">
      <div className="settings-titlebar" data-tauri-drag-region />
      <input
        className="settings-nav-search"
        type="search"
        aria-label="Search Settings"
        placeholder="Search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && results[0]) onOpen(targetOf(results[0]));
          // A first Escape clears the search; with nothing to clear it closes Settings.
          if (e.key === "Escape" && query) {
            e.stopPropagation();
            setQuery("");
          } else if (e.key === "Escape") {
            e.currentTarget.blur();
          }
        }}
      />
      <div className="settings-nav-list scroller">
        {searching && results.length === 0 && <p className="settings-nav-empty">No Results</p>}
        {results.length > 0 && (
          <div className="settings-nav-group" role="group" aria-label="Results">
            {results.map((match) => (
              <NavButton key={targetOf(match)} {...match} current={current} onOpen={onOpen} />
            ))}
          </div>
        )}
        {!searching &&
          settingsGroups().map(({ group, pages }) => (
            <div key={group} className="settings-nav-group" role="group" aria-label={group}>
              <div className="settings-nav-heading" aria-hidden="true">
                {group}
              </div>
              {pages.map((page) => (
                <NavButton key={page.id} page={page} current={current} onOpen={onOpen} />
              ))}
            </div>
          ))}
      </div>
    </nav>
  );
}

/** The row to reveal: kept while the request and the page stay the same, so a re-render doesn't reveal it again. */
function useRevealTarget(request: SettingsRequest, page: SettingsPage | undefined): RevealTarget | undefined {
  return useMemo(() => {
    const row = page && findRow(page, request.id);
    return row ? { id: row.id, under: row.under } : undefined;
  }, [request, page]);
}

/** `request.id` may name a row: its page opens scrolled to it, highlighted. */
export function SettingsWindow({ request, onOpen }: { request: SettingsRequest; onOpen: (id: string) => void }) {
  useSettingsPagesVersion();
  const page = resolveSettingsPage(request.id);
  const target = useRevealTarget(request, page);
  if (!page) return null;
  const { component: Body } = page;
  return (
    <div className="settings-window">
      <SettingsNav current={page.id} onOpen={onOpen} />
      <div className="settings-pane">
        <header data-tauri-drag-region>
          <h2 data-tauri-drag-region>{page.title}</h2>
        </header>
        <div className="settings-body scroller" key={page.id}>
          {page.intro && <p className="settings-intro">{page.intro}</p>}
          <ScrollTarget.Provider value={target}>
            <Body />
          </ScrollTarget.Provider>
        </div>
      </div>
    </div>
  );
}
