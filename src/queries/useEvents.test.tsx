import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Project } from "../api";
import { DEFAULT_SETTINGS, type createApiMock } from "../test/apiMock";
import { createQueryClient } from "./queryClient";
import { useEvents } from "./useEvents";
import { useProjects } from "./useProjects";
import { useSettings } from "./useSettings";

const mocked = vi.mocked(api, true);
const emit = (api as ReturnType<typeof createApiMock>).emit;

const project = (name: string): Project => ({
  id: "demo-1",
  path: "~/Code/demo-shop",
  name,
  meta: { icon: "", color: "", tag: null, notes: [], starred: false, archived: false, url: null, repoUrl: null, editor: null },
  addedAt: 1,
  lastOpenedAt: null,
  available: true,
  fileError: null,
});

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

function renderReads() {
  return renderHook(
    () => {
      useEvents();
      return { projects: useProjects().data, settings: useSettings().data };
    },
    { wrapper },
  );
}

describe("useEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.listProjects.mockResolvedValue([project("Demo Shop")]);
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, tags: ["client"] });
  });

  it("reads the projects again on projects_changed and leaves the settings alone", async () => {
    const { result } = renderReads();
    await waitFor(() => expect(result.current.projects?.[0].name).toBe("Demo Shop"));
    await waitFor(() => expect(result.current.settings?.tags).toEqual(["client"]));
    mocked.listProjects.mockResolvedValue([project("Demo Shop Renamed")]);
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, tags: ["client", "internal"] });

    act(() => emit("projects_changed", null));

    await waitFor(() => expect(result.current.projects?.[0].name).toBe("Demo Shop Renamed"));
    expect(result.current.settings?.tags).toEqual(["client"]);
  });

  it("reads the settings again on settings_changed and leaves the projects alone", async () => {
    const { result } = renderReads();
    await waitFor(() => expect(result.current.projects?.[0].name).toBe("Demo Shop"));
    await waitFor(() => expect(result.current.settings?.tags).toEqual(["client"]));
    mocked.listProjects.mockResolvedValue([project("Demo Shop Renamed")]);
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, tags: ["client", "internal"] });

    act(() => emit("settings_changed", null));

    await waitFor(() => expect(result.current.settings?.tags).toEqual(["client", "internal"]));
    expect(result.current.projects?.[0].name).toBe("Demo Shop");
  });
});
