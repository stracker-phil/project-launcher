import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type LaunchEvent } from "../api";
import type { createApiMock } from "../test/apiMock";
import { useLaunchEvents } from "./useLaunchEvents";

const mocked = vi.mocked(api, true) as ReturnType<typeof createApiMock> & ReturnType<typeof vi.mocked<typeof api>>;

const error = (message: string): LaunchEvent => ({ type: "error", message });
const activate: LaunchEvent = { type: "activate" };

const flush = () => act(async () => {});

describe("useLaunchEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.takeLaunchEvents.mockResolvedValue([]);
  });

  it("hands the events taken on mount to the callback in order", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([error("first"), activate, error("second")]);
    const onEvent = vi.fn();

    renderHook(() => useLaunchEvents(onEvent));

    await waitFor(() => expect(onEvent).toHaveBeenCalledTimes(3));
    expect(onEvent.mock.calls.map(([event]) => event)).toEqual([error("first"), activate, error("second")]);
  });

  it("takes again and hands over the new events when the launch event fires", async () => {
    mocked.takeLaunchEvents.mockResolvedValueOnce([]).mockResolvedValueOnce([error("later")]);
    const onEvent = vi.fn();
    renderHook(() => useLaunchEvents(onEvent));
    await waitFor(() => expect(mocked.on).toHaveBeenCalledWith("launch", expect.any(Function)));
    await flush();
    expect(onEvent).not.toHaveBeenCalled();

    act(() => mocked.emit("launch", null));

    await waitFor(() => expect(onEvent).toHaveBeenCalledWith(error("later")));
  });

  it("logs a failing take through api.logError", async () => {
    mocked.takeLaunchEvents.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    renderHook(() => useLaunchEvents(vi.fn()));

    await waitFor(() => expect(mocked.logError).toHaveBeenCalledWith("Taking launch events failed: Disk full"));
  });

  it("hands nothing over when the take resolves after unmount", async () => {
    let resolve!: (events: LaunchEvent[]) => void;
    mocked.takeLaunchEvents.mockReturnValue(new Promise((r) => (resolve = r)));
    const onEvent = vi.fn();
    const { unmount } = renderHook(() => useLaunchEvents(onEvent));

    unmount();
    resolve([error("too late")]);
    await flush();

    expect(onEvent).not.toHaveBeenCalled();
  });
});
