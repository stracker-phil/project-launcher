import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const MENU_COMMANDS_KEY = ["menuCommands"] as const;

/** The enabled menu items for the palette, the Project menu's fitted to `currentId`; read again on
 *  `settings_changed` (keys) and `projects_changed` (star, lock, tags). */
export function useMenuCommands(currentId: string | null) {
  return useQuery({ queryKey: [...MENU_COMMANDS_KEY, currentId], queryFn: () => api.menuCommands(currentId) });
}
