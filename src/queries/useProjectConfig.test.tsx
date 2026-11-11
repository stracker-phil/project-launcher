import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { EMPTY_PROJECT_CONFIG, type createApiMock } from "../test/apiMock";
import { createQueryClient } from "./queryClient";
import { useEvents } from "./useEvents";
import { useProjectConfig } from "./useProjectConfig";

const mocked = vi.mocked(api, true);
const emit = (api as ReturnType<typeof createApiMock>).emit;

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe("useProjectConfig", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.projectConfig.mockResolvedValue(EMPTY_PROJECT_CONFIG);
  });

  it("returns the config that api.projectConfig reads for the project", async () => {
    mocked.projectConfig.mockResolvedValue({ ...EMPTY_PROJECT_CONFIG, dev: "ddev", isGitRepo: true });

    const { result } = renderHook(() => useProjectConfig("demo-1"), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual({ ...EMPTY_PROJECT_CONFIG, dev: "ddev", isGitRepo: true }));
    expect(mocked.projectConfig).toHaveBeenCalledWith("demo-1");
  });

  it("reads nothing while no project is given", async () => {
    const { result } = renderHook(() => useProjectConfig(null), { wrapper });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(result.current.data).toBeUndefined();
    expect(mocked.projectConfig).not.toHaveBeenCalled();
  });

  it("reads the config once a project is given", async () => {
    const { result, rerender } = renderHook(({ id }: { id: string | null }) => useProjectConfig(id), {
      wrapper,
      initialProps: { id: null as string | null },
    });
    expect(result.current.data).toBeUndefined();

    rerender({ id: "demo-1" });

    await waitFor(() => expect(result.current.data).toEqual(EMPTY_PROJECT_CONFIG));
  });

  it("reports the error when api.projectConfig rejects", async () => {
    mocked.projectConfig.mockRejectedValue({ kind: "not_found", message: "Project file is missing", code: null });

    const { result } = renderHook(() => useProjectConfig("demo-1"), { wrapper });

    await waitFor(() => expect(result.current.error).toEqual({ kind: "not_found", message: "Project file is missing", code: null }));
  });

  it.each(["projects_changed", "settings_changed"] as const)("reads the config again on %s", async (event) => {
    const { result } = renderHook(
      () => {
        useEvents();
        return useProjectConfig("demo-1").data;
      },
      { wrapper },
    );
    await waitFor(() => expect(result.current?.dev).toBe("none"));
    mocked.projectConfig.mockResolvedValue({ ...EMPTY_PROJECT_CONFIG, dev: "docker" });

    act(() => emit(event, null));

    await waitFor(() => expect(result.current?.dev).toBe("docker"));
  });
});
