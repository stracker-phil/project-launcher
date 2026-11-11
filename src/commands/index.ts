import { useEffect } from "react";
import { api } from "../api";
import { registerCommands, runPageCommand } from "./registry";
import { useShortcuts } from "./useShortcuts";
import { MAIN_COMMANDS } from "./mainCommands";
import { SETTINGS_COMMANDS } from "./settingsCommands";

/** Installs the main window's page commands: registers them, routes their keys and runs the menus'
 *  `page:<id>` items (`page_command`). */
export function useMainCommands() {
  useShortcuts();
  useEffect(() => {
    registerCommands(MAIN_COMMANDS);
    const stopListening = api.on("page_command", (id) => void runPageCommand(id));
    return () => void stopListening.then((stop) => stop());
  }, []);
}

/** Installs the Settings window's page commands and routes their keys. */
export function useSettingsCommands() {
  useShortcuts();
  useEffect(() => registerCommands(SETTINGS_COMMANDS), []);
}
