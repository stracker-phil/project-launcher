import { RotateCcw } from "lucide-react";
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

function ShortcutRowView({ row, rows }: { row: ShortcutRow; rows: ShortcutRow[] }) {
  const hidden = !row.overridden;
  return (
    <Row label={row.title} anchor={rowAnchor(row.id)}>
      <div className="shortcut-control">
        {/* Keeps its slot while hidden, so the recorder never moves. */}
        <button
          type="button"
          className={hidden ? "icon-btn tiny slot-hidden" : "icon-btn tiny"}
          aria-label={`Reset ${row.title}`}
          aria-hidden={hidden || undefined}
          tabIndex={hidden ? -1 : undefined}
          title="Reset to default"
          onClick={() => void reset(rows, row)}
        >
          <RotateCcw size={11} aria-hidden="true" />
        </button>
        <ShortcutRecorder
          value={row.key}
          name={row.title}
          onRecord={(key) => void assign(rows, row, key)}
          onUnbind={() => void attempt(api.setShortcut(row.id, null), SAVE_FAILED)}
        />
      </div>
    </Row>
  );
}

/** Each shortcut, grouped by menu or window, changeable or resettable (ADR 0032). */
export function ShortcutsPage() {
  const { data: settings } = useSettings();
  const { data: menu, error } = useMenuShortcuts();
  if (error && !menu) {
    return (
      <p className="settings-empty" role="alert">
        Could not read the shortcuts: {toAppError(error).message}
      </p>
    );
  }
  if (!menu || !settings) return null;
  const rows = shortcutRows(menu, settings.shortcuts);
  return groupRows(rows).map((group) => (
    <Group key={group.title} title={group.title}>
      {group.rows.map((row) => (
        <ShortcutRowView key={row.id} row={row} rows={rows} />
      ))}
    </Group>
  ));
}
