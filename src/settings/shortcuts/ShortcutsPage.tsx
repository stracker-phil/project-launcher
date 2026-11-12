import { RotateCcw, Search, X } from "lucide-react";
import { useState, type ReactNode } from "react";
import { api, toAppError } from "../../api";
import { bindingLabel, hasCommandOrControl } from "../../commands/registry";
import { useMenuShortcuts } from "../../queries/useMenuShortcuts";
import { useSettings } from "../../queries/useSettings";
import { attempt } from "../../state/attempt";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { conflictFor, groupRows, rowAnchor, shortcutRows, type ShortcutRow } from "./rows";
import { ShortcutRecorder } from "./ShortcutRecorder";

const SAVE_FAILED = "Could not change the shortcut";
const RESET_FAILED = "Could not reset the shortcut";

/** Asks whether `row` may take `key` from `other`, which leaves `other` without a shortcut. */
function confirmTakeover(row: ShortcutRow, other: ShortcutRow, key: string): Promise<boolean> {
  return useStore.getState().confirm({
    title: `${bindingLabel(key)} is used by “${other.title}”`,
    text: `Give it to “${row.title}” instead? “${other.title}” is left without a shortcut.`,
    action: "Reassign",
  });
}

/** Gives the row `key`; a key another command has is reassigned after asking. */
async function assign(rows: ShortcutRow[], row: ShortcutRow, key: string) {
  if (row.scope === "menu" && !hasCommandOrControl(key)) {
    useStore.getState().toast("error", `${bindingLabel(key)} can't be a menu shortcut: it needs ⌘ or ⌃.`);
    return;
  }
  if (key === row.key) return;
  // The default key resets the row, so it has nothing left to reset.
  if (key === row.defaultKey) {
    await reset(rows, row);
    return;
  }
  const other = conflictFor(rows, row, key);
  if (!other) {
    await attempt(api.setShortcut(row.id, key), SAVE_FAILED);
    return;
  }
  if (await confirmTakeover(row, other, key)) await attempt(api.reassignShortcut(row.id, key, other.id), SAVE_FAILED);
}

/** Back to the row's default key; a default another command has now is taken from it after asking. */
async function reset(rows: ShortcutRow[], row: ShortcutRow) {
  const key = row.defaultKey;
  const other = key ? conflictFor(rows, row, key) : undefined;
  if (!key || !other) {
    await attempt(api.resetShortcut(row.id), RESET_FAILED);
    return;
  }
  if (await confirmTakeover(row, other, key)) await attempt(api.resetShortcut(row.id, other.id), RESET_FAILED);
}

/** Keeps its slot when hidden, so its neighbours never move. */
function SlotButton({ shown, label, title, onClick, children }: { shown: boolean; label: string; title: string; onClick: () => void; children: ReactNode }) {
  if (!shown) return <span className="icon-btn tiny slot-hidden" aria-hidden="true" />;
  return (
    <button type="button" className="icon-btn tiny" aria-label={label} title={title} onClick={onClick}>
      {children}
    </button>
  );
}

function ShortcutRowView({ row, rows }: { row: ShortcutRow; rows: ShortcutRow[] }) {
  const unbind = () => void attempt(api.setShortcut(row.id, null), SAVE_FAILED);
  return (
    <Row label={row.title} description={row.detail} anchor={rowAnchor(row.id)}>
      <div className="shortcut-control">
        <SlotButton shown={row.overridden} label={`Reset ${row.title}`} title="Reset to default" onClick={() => void reset(rows, row)}>
          <RotateCcw size={11} aria-hidden="true" />
        </SlotButton>
        <SlotButton shown={row.key !== null} label={`Remove shortcut for ${row.title}`} title="Remove shortcut" onClick={unbind}>
          <X size={11} aria-hidden="true" />
        </SlotButton>
        <ShortcutRecorder value={row.key} name={row.title} onRecord={(key) => void assign(rows, row, key)} onUnbind={unbind} />
      </div>
    </Row>
  );
}

const matches = (row: ShortcutRow, needle: string) =>
  !needle || [row.title, row.group, row.detail ?? ""].some((text) => text.toLowerCase().includes(needle));

/** Each shortcut, grouped by where it works, changeable, removable or resettable (ADR 0032); a search narrows
 *  the rows. */
export function ShortcutsPage() {
  const { data: settings } = useSettings();
  const { data: menu, error } = useMenuShortcuts();
  const [search, setSearch] = useState("");
  if (error && !menu) {
    return (
      <p className="settings-empty" role="alert">
        Could not read the shortcuts: {toAppError(error).message}
      </p>
    );
  }
  if (!menu || !settings) return null;
  const rows = shortcutRows(menu, settings.shortcuts);
  const needle = search.trim().toLowerCase();
  const groups = groupRows(rows.filter((row) => matches(row, needle)));
  return (
    <>
      <label className="settings-search">
        <Search size={14} aria-hidden="true" />
        <input type="search" aria-label="Search commands" placeholder="Search commands"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            // A first Escape clears the search; with nothing to clear it closes Settings.
            if (e.key !== "Escape" || !search) return;
            e.stopPropagation();
            setSearch("");
          }}
        />
      </label>
      {groups.length === 0 && (
        <Group>
          <div className="settings-row settings-empty">No matching commands.</div>
        </Group>
      )}
      {groups.map((group) => (
        <Group key={group.title} title={group.title}>
          {group.rows.map((row) => (
            <ShortcutRowView key={row.id} row={row} rows={rows} />
          ))}
        </Group>
      ))}
    </>
  );
}
