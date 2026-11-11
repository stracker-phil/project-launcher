import { vi } from "vitest";
import type { Api, EventName, EventPayloads, Library, ProjectConfig, ProjectStatus, Settings } from "../api";

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

/** The config of a project whose file sets nothing up. */
export const EMPTY_PROJECT_CONFIG: ProjectConfig = {
  apps: [],
  scripts: [],
  builtins: [],
  env: [],
  stateProviders: [],
  states: [],
  dev: "none",
  database: "detect",
  claude: null,
  locked: false,
  isGitRepo: false,
};

/** The status of a project without states, hidden actions or running terminals. */
export const EMPTY_STATUS: ProjectStatus = { states: [], hidden: [], running: [] };

/** The library of an empty install under the "All Projects" filter. */
export const EMPTY_LIBRARY: Library = {
  title: "All Projects",
  countLabel: "0 projects",
  sidebar: { all: 0, starred: 0, untagged: null, archived: 0, tags: [] },
  sections: [],
  emptyText: "No projects here",
};

type Handler =(payload: unknown) => void;

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
    menuShortcuts: vi.fn().mockResolvedValue([]),
    pauseMenuKeys: vi.fn().mockResolvedValue(undefined),
    getSettings: vi.fn().mockResolvedValue(DEFAULT_SETTINGS),
    setAppearance: vi.fn().mockResolvedValue(undefined),
    setShortcut: vi.fn().mockResolvedValue(undefined),
    reassignShortcut: vi.fn().mockResolvedValue(undefined),
    resetShortcut: vi.fn().mockResolvedValue(undefined),
    addTag: vi.fn().mockResolvedValue(undefined),
    renameTag: vi.fn().mockResolvedValue(undefined),
    deleteTag: vi.fn().mockResolvedValue(undefined),
    setTagOrder: vi.fn().mockResolvedValue(undefined),
    listProjects: vi.fn().mockResolvedValue([]),
    reloadProjects: vi.fn().mockResolvedValue(undefined),
    addProject: vi.fn().mockRejectedValue({ kind: "invalid_input", message: "No folder at this path" }),
    addProjectFolders: vi.fn().mockResolvedValue({ added: [], existing: [] }),
    library: vi.fn().mockResolvedValue(EMPTY_LIBRARY),
    removeProject: vi.fn().mockResolvedValue(undefined),
    setProjectName: vi.fn().mockResolvedValue(undefined),
    setProjectMeta: vi.fn().mockResolvedValue(undefined),
    projectConfig: vi.fn().mockResolvedValue(EMPTY_PROJECT_CONFIG),
    saveAction: vi.fn().mockResolvedValue(undefined),
    removeAction: vi.fn().mockResolvedValue(undefined),
    moveAction: vi.fn().mockResolvedValue(undefined),
    setBuiltinEnabled: vi.fn().mockResolvedValue(undefined),
    setProjectLocked: vi.fn().mockResolvedValue(undefined),
    setDevMode: vi.fn().mockResolvedValue(undefined),
    duplicateAction: vi.fn().mockResolvedValue(undefined),
    launchAction: vi.fn().mockResolvedValue({ kind: "opened", how: "launched" }),
    openTerminal: vi.fn().mockResolvedValue(undefined),
    revealProject: vi.fn().mockResolvedValue(undefined),
    copyProjectPath: vi.fn().mockResolvedValue(undefined),
    editProjectFile: vi.fn().mockResolvedValue(undefined),
    projectStatus: vi.fn().mockResolvedValue(EMPTY_STATUS),
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
