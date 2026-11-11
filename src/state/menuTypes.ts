import type { ReactNode } from "react";

/** A row of a menu: runs `run` when chosen, or opens `items` as a submenu. */
export interface MenuItem {
  text: string;
  /** False grays the row out; it can't be chosen and the keys skip it. */
  enabled?: boolean;
  /** Makes the row a choice; true shows the check. */
  checked?: boolean;
  /** With `checked`: an on/off switch rather than one choice of a group. */
  toggle?: boolean;
  /** Before the text: a line icon or a tile. */
  icon?: ReactNode;
  /** At the right, muted: a short hint. */
  hint?: string;
  /** At the right, muted: the key that runs it too ("⌘K"). */
  shortcut?: string;
  /** A destructive action, in red. */
  danger?: boolean;
  run?: () => void;
  items?: MenuEntry[];
}

/** A row, a separator line, or a caps caption that starts a group of rows. */
export type MenuEntry = MenuItem | "separator" | { caption: string };

/** Where a menu opens: at a point (a right-click) or under an element (a button, which also gets the focus back). */
export type MenuAt = { x: number; y: number } | HTMLElement;

export interface OpenMenu {
  /** Tells menus apart, so a new one starts fresh. */
  id: number;
  entries: MenuEntry[];
  at: MenuAt;
  /** Its accessible name. */
  label: string;
  /** Where the focus goes back to when it closes. */
  opener: HTMLElement | null;
  /** A search field's placeholder: the menu opens with the field focused and narrows its rows as you type. */
  search?: string;
}

export interface MenuOptions {
  search?: string;
}

export const isItem = (entry: MenuEntry): entry is MenuItem => entry !== "separator" && !("caption" in entry);
