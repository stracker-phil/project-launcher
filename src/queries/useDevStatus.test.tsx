import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "./queryClient";
import { useDevStatus } from "./useDevStatus";

const mocked = vi.mocked(api, true);

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

beforeEach(() => vi.clearAllMocks());

describe("useDevStatus", () => {
  it("reads the dev status of the given project", async () => {
    mocked.devStatus.mockResolvedValue({ docker: { running: true, activity: null, manager: "OrbStack" }, server: null, database: null });

    const { result } = renderHook(() => useDevStatus("alpha"), { wrapper });

    await waitFor(() => expect(result.current.data?.docker?.manager).toBe("OrbStack"));
    expect(mocked.devStatus).toHaveBeenCalledWith("alpha");
  });

  it("reads nothing without a project", () => {
    const { result } = renderHook(() => useDevStatus(null), { wrapper });

    expect(result.current.data).toBeUndefined();
    expect(mocked.devStatus).not.toHaveBeenCalled();
  });
});
