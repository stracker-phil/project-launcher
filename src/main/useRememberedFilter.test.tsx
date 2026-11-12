import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Filter } from "../api";
import { resetLibrary, useLibrary } from "../state/library";
import { useRememberedFilter } from "./useRememberedFilter";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  resetLibrary();
  mocked.viewFilter.mockResolvedValue({ kind: "all" });
  mocked.setViewFilter.mockResolvedValue(undefined);
});

describe("useRememberedFilter", () => {
  it.each<[string, Filter]>([
    ["starred", { kind: "starred" }],
    ["archived", { kind: "archived" }],
    ["tag", { kind: "tag", tag: "Client" }],
  ])("starts on the %s filter Rust remembered", async (_name, filter) => {
    mocked.viewFilter.mockResolvedValue(filter);

    renderHook(() => useRememberedFilter());

    await waitFor(() => expect(useLibrary.getState().filter).toEqual(filter));
  });

  it("tells Rust the new filter when the owner picks another one", async () => {
    renderHook(() => useRememberedFilter());

    act(() => useLibrary.getState().setFilter({ kind: "tag", tag: "Client" }));

    await waitFor(() => expect(mocked.setViewFilter).toHaveBeenLastCalledWith({ kind: "tag", tag: "Client" }));
  });

  it("keeps a filter chosen before Rust answers", async () => {
    let answer: (filter: Filter) => void = () => {};
    mocked.viewFilter.mockReturnValue(new Promise<Filter>((resolve) => (answer = resolve)));
    renderHook(() => useRememberedFilter());

    act(() => useLibrary.getState().setFilter({ kind: "archived" }));
    await act(async () => answer({ kind: "starred" }));

    expect(useLibrary.getState().filter).toEqual({ kind: "archived" });
  });

  it("stays on All when Rust cannot answer", async () => {
    mocked.viewFilter.mockRejectedValue({ kind: "io", message: "Unreadable", code: null });

    renderHook(() => useRememberedFilter());
    await act(async () => {});

    expect(useLibrary.getState().filter).toEqual({ kind: "all" });
  });

  it("keeps the filter change when Rust rejects the report", async () => {
    mocked.setViewFilter.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderHook(() => useRememberedFilter());

    act(() => useLibrary.getState().setFilter({ kind: "starred" }));
    await act(async () => {});

    expect(useLibrary.getState().filter).toEqual({ kind: "starred" });
  });

  it("stops reporting when unmounted", async () => {
    const { unmount } = renderHook(() => useRememberedFilter());
    await act(async () => {});
    unmount();

    act(() => useLibrary.getState().setFilter({ kind: "archived" }));

    expect(mocked.setViewFilter).not.toHaveBeenCalledWith({ kind: "archived" });
  });
});
