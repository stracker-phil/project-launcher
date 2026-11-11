import { vi } from "vitest";
import type { Api, EventName, EventPayloads, Settings } from "../api";

/** The settings a fresh install starts with. */
export const DEFAULT_SETTINGS: Settings = {
  appearance: "system",
  tags: [],
  shortcuts: {},
  defaultEditor: "",
  gitClient: "",
  terminalApp: "Terminal",
  configEditor: "",
};

type Handler = (payload: unknown) => void;

/** A fake `api` for UI tests; `emit` plays an event from Rust to the listeners. */
export function createApiMock(): Api & { emit: <E extends EventName>(event: E, payload: EventPayloads[E]) => void } {
  const byEvent = new Map<string, Set<Handler>>();
  return {
    openUrl: vi.fn().mockResolvedValue(undefined),
    logError: vi.fn().mockResolvedValue(undefined),
    takeLaunchEvents: vi.fn().mockResolvedValue([]),
    runCommand: vi.fn().mockResolvedValue(undefined),
    appStatus: vi.fn().mockResolvedValue({
      name: "Launcher",
      version: "1.2.3",
      debug: false,
      dataFolder: "~/Library/Application Support/com.example.launcher",
      logFolder: "~/Library/Logs/com.example.launcher",
      commandLine: { path: "~/.local/bin/launcher", link: "installed" },
    }),
    menuCommands: vi.fn().mockResolvedValue([]),
    getSettings: vi.fn().mockResolvedValue(DEFAULT_SETTINGS),
    setAppearance: vi.fn().mockResolvedValue(undefined),
    setShortcut: vi.fn().mockResolvedValue(undefined),
    resetShortcut: vi.fn().mockResolvedValue(undefined),
    addTag: vi.fn().mockResolvedValue(undefined),
    renameTag: vi.fn().mockResolvedValue(undefined),
    deleteTag: vi.fn().mockResolvedValue(undefined),
    setTagOrder: vi.fn().mockResolvedValue(undefined),
    listProjects: vi.fn().mockResolvedValue([]),
    reloadProjects: vi.fn().mockResolvedValue(undefined),
    addProject: vi.fn().mockRejectedValue({ kind: "invalid_input", message: "No folder at this path" }),
    removeProject: vi.fn().mockResolvedValue(undefined),
    setProjectName: vi.fn().mockResolvedValue(undefined),
    setProjectMeta: vi.fn().mockResolvedValue(undefined),
    undo: vi.fn().mockResolvedValue(null),
    redo: vi.fn().mockResolvedValue(null),
    on: vi.fn(async (event: string, handler: Handler) => {
      const handlers = byEvent.get(event) ?? new Set<Handler>();
      handlers.add(handler);
      byEvent.set(event, handlers);
      return () => handlers.delete(handler);
    }) as unknown as Api["on"],
    emit: (event, payload) => byEvent.get(event)?.forEach((handler) => handler(payload)),
  };
}
