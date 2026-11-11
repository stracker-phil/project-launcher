import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Project } from "../api";
import { createQueryClient } from "./queryClient";
import { useProjects } from "./useProjects";

const mocked = vi.mocked(api, true);

const project: Project = {
  id: "demo-1",
  path: "~/Code/demo-shop",
  name: "Demo Shop",
  meta: { icon: "", color: "", tag: null, notes: [], starred: false, archived: false, url: null, repoUrl: null, editor: null },
  addedAt: 1,
  lastOpenedAt: null,
  available: true,
  locked: false,
  fileError: null,
};

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe("useProjects", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.listProjects.mockResolvedValue([]);
  });

  it("returns the projects that api.listProjects reads", async () => {
    mocked.listProjects.mockResolvedValue([project]);

    const { result } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual([project]));
  });

  it("returns an empty list when there are no projects", async () => {
    const { result } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => expect(result.current.data).toEqual([]));
  });

  it("reports the error when api.listProjects rejects", async () => {
    mocked.listProjects.mockRejectedValue({ kind: "io", message: "Cannot read projects", code: null });

    const { result } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => expect(result.current.error).toEqual({ kind: "io", message: "Cannot read projects", code: null }));
  });
});
