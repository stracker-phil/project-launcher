import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError, ErrorKind, LaunchEvent } from "./types.gen";

export type { AppError, ErrorKind, LaunchEvent } from "./types.gen";

/** Every event Rust emits to the page (`src-tauri/src/events.rs`), with its payload. */
export type EventPayloads = {
  launch: null;
  window_opened: null;
  /** A menu's `page:<id>` item, without the prefix. */
  page_command: string;
};
export type EventName = keyof EventPayloads;

/** The page's whole contract with Rust; UI tests replace it with `createApiMock()`. */
export const api = {
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  logError: (message: string) => invoke<void>("log_frontend_error", { message }),
  takeLaunchEvents: () => invoke<LaunchEvent[]>("take_launch_events"),
  /** Runs a command of the Rust bus by id (`app:show`, `page:view.toggleSidebar`, …). */
  runCommand: (id: string) => invoke<void>("run_command", { id }),
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
