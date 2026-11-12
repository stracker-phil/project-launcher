import { useState } from "react";
import { hasRow, resolveSettingsPage, settingsGroups, useSettingsPagesVersion, type SettingsMatch } from "../registry/settingsPages";
import { ScrollTarget } from "./controls";

const targetOf = (match: SettingsMatch) => match.row?.id ?? match.page.id;

/** The page list under its group headings, narrowed by the search field; a page found by a row's title opens at
 *  that row. */
function SettingsNav({ current, onPage }: { current: string; onPage: (id: string) => void }) {
  const [query, setQuery] = useState("");
  const groups = settingsGroups(query);
  const first = groups[0]?.matches[0];
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
          if (e.key === "Enter" && first) onPage(targetOf(first));
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
        {groups.length === 0 && <p className="settings-nav-empty">No Results</p>}
        {groups.map(({ group, matches }) => (
          <div key={group} className="settings-nav-group" role="group" aria-label={group}>
            <div className="settings-nav-heading" aria-hidden="true">
              {group}
            </div>
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
        ))}
      </div>
    </nav>
  );
}

/** `target` is a page id or a row id; a row's page opens scrolled to it. */
export function SettingsWindow({ target, onPage }: { target: string | undefined; onPage: (id: string) => void }) {
  useSettingsPagesVersion();
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
          {page.intro && <p className="settings-intro">{page.intro}</p>}
          <ScrollTarget.Provider value={row}>
            <Body />
          </ScrollTarget.Provider>
        </div>
      </div>
    </div>
  );
}
