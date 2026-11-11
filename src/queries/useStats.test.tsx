import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { EMPTY_FILE_STATS, EMPTY_QUICK_STATS } from "../test/apiMock";
import { fileStats, quickStats } from "../test/stats";
import { createQueryClient } from "./queryClient";
import { useFileStats, useQuickStats, useRefreshStats } from "./useStats";

const mocked = vi.mocked(api, true);

let client = createQueryClient();

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  vi.clearAllMocks();
  client = createQueryClient();
  mocked.quickStats.mockResolvedValue(EMPTY_QUICK_STATS);
  mocked.fileStats.mockResolvedValue(EMPTY_FILE_STATS);
});

describe("useQuickStats", () => {
  it("returns the stats api.quickStats reads for the project, from Rust's cache", async () => {
    mocked.quickStats.mockResolvedValue(quickStats({ kind: "Node project" }));

    const { result } = renderHook(() => useQuickStats("demo-1"), { wrapper });

    await waitFor(() => expect(result.current.data?.kind).toBe("Node project"));
    expect(mocked.quickStats).toHaveBeenCalledWith("demo-1", false);
  });

  it("reads nothing while no project is given", async () => {
    const { result } = renderHook(() => useQuickStats(null), { wrapper });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(result.current.data).toBeUndefined();
    expect(mocked.quickStats).not.toHaveBeenCalled();
  });
});

describe("useFileStats", () => {
  it("returns the stats api.fileStats reads for the project, from Rust's cache", async () => {
    mocked.fileStats.mockResolvedValue(fileStats({ files: 12 }));

    const { result } = renderHook(() => useFileStats("demo-1"), { wrapper });

    await waitFor(() => expect(result.current.data?.files).toBe(12));
    expect(mocked.fileStats).toHaveBeenCalledWith("demo-1", false);
  });

  it("reads nothing while no project is given", async () => {
    const { result } = renderHook(() => useFileStats(null), { wrapper });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(result.current.data).toBeUndefined();
    expect(mocked.fileStats).not.toHaveBeenCalled();
  });
});

describe("useRefreshStats", () => {
  it("reads both stats past the cache and updates what the hooks return", async () => {
    mocked.quickStats.mockResolvedValueOnce(quickStats({ kind: "Old" }));
    mocked.fileStats.mockResolvedValueOnce(fileStats({ files: 1 }));
    const { result } = renderHook(
      () => ({ quick: useQuickStats("demo-1").data, files: useFileStats("demo-1").data, refresh: useRefreshStats("demo-1") }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.files?.files).toBe(1));
    mocked.quickStats.mockResolvedValue(quickStats({ kind: "New" }));
    mocked.fileStats.mockResolvedValue(fileStats({ files: 2 }));

    await act(() => result.current.refresh());

    expect(mocked.quickStats).toHaveBeenCalledWith("demo-1", true);
    expect(mocked.fileStats).toHaveBeenCalledWith("demo-1", true);
    await waitFor(() => expect(result.current.quick?.kind).toBe("New"));
    expect(result.current.files?.files).toBe(2);
  });

  it("reads nothing while no project is given", async () => {
    const { result } = renderHook(() => useRefreshStats(null), { wrapper });

    await act(() => result.current());

    expect(mocked.quickStats).not.toHaveBeenCalled();
    expect(mocked.fileStats).not.toHaveBeenCalled();
  });

  it("rejects with the error when a read fails", async () => {
    mocked.fileStats.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const { result } = renderHook(() => useRefreshStats("demo-1"), { wrapper });

    await expect(result.current()).rejects.toEqual({ kind: "io", message: "Disk full", code: null });
  });
});
