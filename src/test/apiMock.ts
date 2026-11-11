import { vi } from "vitest";
import type { Api, EventName, EventPayloads } from "../api";

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
    on: vi.fn(async (event: string, handler: Handler) => {
      const handlers = byEvent.get(event) ?? new Set<Handler>();
      handlers.add(handler);
      byEvent.set(event, handlers);
      return () => handlers.delete(handler);
    }) as unknown as Api["on"],
    emit: (event, payload) => byEvent.get(event)?.forEach((handler) => handler(payload)),
  };
}
