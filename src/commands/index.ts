import { useEffect } from "react";
import { api } from "../api";
import { registerCommands } from "./registry";
import { useEditRequests } from "./useEditRequests";
import { useShortcutOverrides } from "./useShortcutOverrides";
import { useShortcuts } from "./useShortcuts";
import { LIBRARY_COMMANDS } from "../main/libraryCommands";
import { runPageRequest } from "../main/pageRequests";
import { MAIN_COMMANDS } from "./mainCommands";
import { SETTINGS_COMMANDS } from "./settingsCommands";

/** Installs the main window's page commands: registers them, routes their keys (with the owner's overrides) and runs the menus'
 *  `page:<id>` items (`page_command`) and the Edit menu's Undo and Redo (`edit_request`). */
export function useMainCommands() {
  useShortcuts();
  useShortcutOverrides();
  useEditRequests();
  useEffect(() => {
    registerCommands([...MAIN_COMMANDS, ...LIBRARY_COMMANDS]);
    const stopListening = api.on("page_command", (id) => void runPageRequest(id));
    return () => void stopListening.then((stop) => stop());
  }, []);
}

/** Installs the Settings window's page commands and routes their keys. */
export function useSettingsCommands() {
  useShortcuts();
  useShortcutOverrides();
  useEditRequests();
  useEffect(() => registerCommands(SETTINGS_COMMANDS), []);
}
