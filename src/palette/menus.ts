import { type RefObject, useEffect, useRef } from "react";
import type { MenuEntry } from "../state/menuTypes";

/** A menu of the page's own the palette lists while it is on screen, e.g. a project's context menu. */
export interface PaletteMenu {
  /** The palette's section. */
  section: string;
  /** Before each item's text: "Project" → "Project: Rename". */
  title: string;
  entries: () => MenuEntry[];
}

const menus = new Map<string, RefObject<PaletteMenu | null>>();

/** Offers `menu` under `id` while mounted; the latest registration wins, null offers nothing. */
export function usePaletteMenu(id: string, menu: PaletteMenu | null) {
  const latest = useRef(menu);
  useEffect(() => {
    latest.current = menu;
  });
  const offered = menu !== null;
  useEffect(() => {
    if (!offered) return;
    menus.set(id, latest);
    return () => {
      if (menus.get(id) === latest) menus.delete(id);
    };
  }, [id, offered]);
}

/** What the page offers now, in registration order. */
export function paletteMenus(): PaletteMenu[] {
  return [...menus.values()].flatMap((menu) => (menu.current ? [menu.current] : []));
}
