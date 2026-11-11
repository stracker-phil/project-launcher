import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { DEFAULT_SETTINGS } from "../test/apiMock";
import { createQueryClient } from "./queryClient";
import { useSettings } from "./useSettings";

const mocked = vi.mocked(api, true);

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe("useSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
  });

  it("returns the settings that api.getSettings reads", async () => {
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: "dark", tags: ["client", "internal"] });

    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual(expect.objectContaining({ appearance: "dark", tags: ["client", "internal"] })));
  });

  it("reports the error when api.getSettings rejects", async () => {
    mocked.getSettings.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    const { result } = renderHook(() => useSettings(), { wrapper });

    await waitFor(() => expect(result.current.error).toEqual({ kind: "io", message: "Disk full", code: null }));
    expect(result.current.data).toBeUndefined();
  });
});
