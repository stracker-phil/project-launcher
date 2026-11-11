import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const MENU_SHORTCUTS_KEY = ["menuShortcuts"] as const;

/** The menu items' keys for Settings → Shortcuts; read again on `settings_changed`. */
export function useMenuShortcuts() {
  return useQuery({ queryKey: MENU_SHORTCUTS_KEY, queryFn: api.menuShortcuts });
}
