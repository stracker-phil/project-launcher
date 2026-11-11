import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError, AppStatus, Appearance, ErrorKind, LaunchEvent, MenuCommand, MetaEdit, Project, Settings } from "./types.gen";

export type {
  AppError,
  AppStatus,
  Appearance,
  CliLink,
  CommandLine,
  ErrorKind,
  LaunchEvent,
  MenuCommand,
  MetaEdit,
  Project,
  ProjectMeta,
  Settings,
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
  getSettings: () => invoke<Settings>("get_settings"),
  setAppearance: (appearance: Appearance) => invoke<void>("set_appearance", { appearance }),
  /** `key: null` leaves the command without a key. */
  setShortcut: (command: string, key: string | null) => invoke<void>("set_shortcut", { command, key }),
  /** Back to the command's default key. */
  resetShortcut: (command: string) => invoke<void>("reset_shortcut", { command }),
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
  removeProject: (id: string) => invoke<void>("remove_project", { id }),
  /** An empty name falls back to the folder's name. */
  setProjectName: (id: string, name: string) => invoke<void>("set_project_name", { id, name }),
  setProjectMeta: (id: string, edit: MetaEdit) => invoke<void>("set_project_meta", { id, edit }),
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
