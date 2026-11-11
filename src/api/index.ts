import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActionDraft,
  ActionSource,
  AddedProjects,
  AppError,
  AppStatus,
  Appearance,
  Builtin,
  DevMode,
  ErrorKind,
  Filter,
  LaunchEvent,
  Library,
  MenuCommand,
  MenuShortcut,
  MetaEdit,
  Project,
  ProjectConfig,
  Settings,
} from "./types.gen";

export type {
  AddedProjects,
  Action,
  ActionDraft,
  ActionEdit,
  ActionList,
  ActionSource,
  AppError,
  AppStatus,
  Appearance,
  Builtin,
  BuiltinApp,
  ClaudeSetup,
  CliLink,
  CommandLine,
  DatabaseSetting,
  DevMode,
  EnvVar,
  ErrorKind,
  Filter,
  LaunchEvent,
  Library,
  MenuCommand,
  MenuShortcut,
  MetaEdit,
  Project,
  ProjectConfig,
  ProjectMeta,
  Row,
  Section,
  Sidebar,
  TagCount,
  RunsAs,
  Settings,
  StateDef,
  StateProvider,
  StateValue,
} from "./types.gen";

/** Every event Rust emits to the page (`src-tauri/src/events.rs`), with its payload. */
export type EventPayloads = {
  launch: null;
  window_opened: null;
  /** A menu's `page:<id>` item, without the prefix. */
  page_command: string;
  /** A settings page id for the Settings window to show. */
  open_settings: string;
  /** The project list or a project's details changed. */
  projects_changed: null;
  /** The settings or the tag list changed. */
  settings_changed: null;
  /** The Edit menu's Undo or Redo, for this window to route (text field or data). */
  edit_request: "undo" | "redo";
};
export type EventName = keyof EventPayloads;

/** The page's whole contract with Rust; UI tests replace it with `createApiMock()`. */
export const api = {
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  logError: (message: string) => invoke<void>("log_frontend_error", { message }),
  takeLaunchEvents: () => invoke<LaunchEvent[]>("take_launch_events"),
  /** Runs a command of the Rust bus by id (`app:show`, `page:view.toggleSidebar`, …). */
  runCommand: (id: string) => invoke<void>("run_command", { id }),
  /** Settings → Status: version, build, folders and the command line link. */
  appStatus: () => invoke<AppStatus>("app_status"),
  /** The menu bar's enabled items, in menu order, for the command palette. */
  menuCommands: () => invoke<MenuCommand[]>("menu_commands"),
  /** Every menu item of a known command with its default and current key, for Settings → Shortcuts. */
  menuShortcuts: () => invoke<MenuShortcut[]>("menu_shortcuts"),
  /** Takes the menu items' keys off while a shortcut is being recorded. */
  pauseMenuKeys: (paused: boolean) => invoke<void>("pause_menu_keys", { paused }),
  getSettings: () => invoke<Settings>("get_settings"),
  setAppearance: (appearance: Appearance) => invoke<void>("set_appearance", { appearance }),
  /** `key: null` leaves the command without a key. */
  setShortcut: (command: string, key: string | null) => invoke<void>("set_shortcut", { command, key }),
  /** Gives `command` the key `from` has and leaves `from` without one, as one change. */
  reassignShortcut: (command: string, key: string, from: string) => invoke<void>("reassign_shortcut", { command, key, from }),
  /** Back to the command's default key; `from`, which has that key now, is left without one, as one change. */
  resetShortcut: (command: string, from?: string) => invoke<void>("reset_shortcut", { command, from }),
  addTag: (name: string) => invoke<void>("add_tag", { name }),
  /** Also renames the tag in every project file that carries it. */
  renameTag: (from: string, to: string) => invoke<void>("rename_tag", { from, to }),
  /** Also removes the tag from every project file that carries it. */
  deleteTag: (name: string) => invoke<void>("delete_tag", { name }),
  setTagOrder: (tags: string[]) => invoke<void>("set_tag_order", { tags }),
  listProjects: () => invoke<Project[]>("list_projects"),
  /** Reads every project file from disk again. */
  reloadProjects: () => invoke<void>("reload_projects"),
  addProject: (path: string) => invoke<Project>("add_project", { path }),
  /** Shows the folder panel and adds the chosen folders in one step; empty when cancelled. */
  addProjectFolders: () => invoke<AddedProjects>("add_project_folders"),
  /** The sidebar, title and sections the main window shows for a filter and a search. */
  library: (filter: Filter, query: string) => invoke<Library>("library", { filter, query }),
  removeProject: (id: string) => invoke<void>("remove_project", { id }),
  /** An empty name falls back to the folder's name. */
  setProjectName: (id: string, name: string) => invoke<void>("set_project_name", { id, name }),
  setProjectMeta: (id: string, edit: MetaEdit) => invoke<void>("set_project_meta", { id, edit }),
  /** The project's apps, scripts, env, states and dev setup, resolved; fails while its file is missing or broken. */
  projectConfig: (id: string) => invoke<ProjectConfig>("project_config", { id }),
  /** `source: null` creates the action; `preset` names the built-in the draft overrides. */
  saveAction: (id: string, source: ActionSource | null, draft: ActionDraft, preset: Builtin | null = null) =>
    invoke<void>("save_action", { id, source, draft, preset }),
  removeAction: (id: string, source: ActionSource) => invoke<void>("remove_action", { id, source }),
  moveAction: (id: string, source: ActionSource, to: number) => invoke<void>("move_action", { id, source, to }),
  setBuiltinEnabled: (id: string, preset: Builtin, enabled: boolean) => invoke<void>("set_builtin_enabled", { id, preset, enabled }),
  /** A locked project's details can't be edited, except starring, archiving and unlocking. */
  setProjectLocked: (id: string, locked: boolean) => invoke<void>("set_project_locked", { id, locked }),
  setDevMode: (id: string, mode: DevMode) => invoke<void>("set_dev_mode", { id, mode }),
  /** Takes back the last change; resolves to its label, or null when there was none. */
  undo: () => invoke<string | null>("undo_change"),
  redo: () => invoke<string | null>("redo_change"),
  on: <E extends EventName>(event: E, handler: (payload: EventPayloads[E]) => void): Promise<UnlistenFn> =>
    listen<EventPayloads[E]>(event, (e) => handler(e.payload)),
};

export type Api = typeof api;

const ERROR_KINDS: Record<ErrorKind, true> = {
  io: true,
  invalid_input: true,
  not_found: true,
  internal: true,
};

/** Any thrown value as an `AppError`; unknown kinds become `io`. */
export function toAppError(e: unknown): AppError {
  if (typeof e !== "object" || e === null || !("message" in e)) return { kind: "io", message: String(e) };
  if ("kind" in e && Object.hasOwn(ERROR_KINDS, String(e.kind))) return e as AppError;
  return { kind: "io", message: String(e.message) };
}
