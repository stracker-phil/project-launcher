import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const MENU_COMMANDS_KEY = ["menuCommands"] as const;

/** The enabled menu items for the palette; read again on `settings_changed` (keys) and `projects_changed`. */
export function useMenuCommands() {
  return useQuery({ queryKey: MENU_COMMANDS_KEY, queryFn: api.menuCommands });
}
