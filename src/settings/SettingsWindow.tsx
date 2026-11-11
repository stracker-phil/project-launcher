import { useState } from "react";
import { hasRow, resolveSettingsPage, searchSettingsPages, type SettingsMatch } from "../registry/settingsPages";
import { ScrollTarget } from "./controls";

const targetOf = (match: SettingsMatch) => match.row?.id ?? match.page.id;

/** The page list, narrowed by the search field; a page found by a row's title opens at that row. */
function SettingsNav({ current, onPage }: { current: string; onPage: (id: string) => void }) {
  const [query, setQuery] = useState("");
  const matches = searchSettingsPages(query);
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
          if (e.key === "Enter" && matches[0]) onPage(targetOf(matches[0]));
        }}
      />
      <div className="settings-nav-list scroller">
        {matches.length === 0 && <p className="settings-nav-empty">No Results</p>}
        {matches.map((match) => {
          const { page: { id, title, icon: Icon }, row } = match;
          return (
            <button
              key={id}
              type="button"
              aria-label={row ? `${title} ${row.title}` : undefined}
              aria-current={current === id ? "page" : undefined}
              onClick={() => onPage(targetOf(match))}
            >
              <Icon size={14} aria-hidden="true" />
              <span className="settings-nav-title">{title}</span>
              {row && <span className="settings-nav-row">{row.title}</span>}
            </button>
          );
        })}
      </div>
    </nav>
  );
}

/** `target` is a page id or a row id; a row's page opens scrolled to it. */
export function SettingsWindow({ target, onPage }: { target: string | undefined; onPage: (id: string) => void }) {
  const page = resolveSettingsPage(target);
  if (!page) return null;
  const row = hasRow(page, target) ? target : undefined;
  const { component: Body } = page;
  return (
    <div className="settings-window">
      <SettingsNav current={page.id} onPage={onPage} />
      <div className="settings-pane">
        <header data-tauri-drag-region>
          <h2 data-tauri-drag-region>{page.title}</h2>
        </header>
        <div className="settings-body scroller" key={page.id}>
          <ScrollTarget.Provider value={row}>
            <Body />
          </ScrollTarget.Provider>
        </div>
      </div>
    </div>
  );
}
