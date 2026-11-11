/** The window's one menu: context menus at the mouse, and the menus of pop-up buttons under them (ADR 0030). */
import type { MouseEvent } from "react";
import { useStore } from "../../state/store";
import type { MenuAt, MenuEntry, MenuOptions } from "../../state/menuTypes";

export { isItem, type MenuAt, type MenuEntry, type MenuItem, type MenuOptions } from "../../state/menuTypes";
export { MenuHost } from "./MenuHost";

/** Opens a menu at a point or under an element; `label` is its accessible name. `options.search` gives it a search
 *  field, focused, that narrows the rows. */
export function openMenu(entries: MenuEntry[], at: MenuAt, label?: string, options?: MenuOptions) {
  useStore.getState().openMenu(entries, at, label, options);
}

/** A right-click's menu, at the mouse; the WebView's own menu stays shut. */
export function openContextMenu(e: MouseEvent, entries: MenuEntry[], label?: string) {
  e.preventDefault();
  e.stopPropagation();
  openMenu(entries, { x: e.clientX, y: e.clientY }, label);
}
