import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuShortcut, type Project } from "../api";
import { DEFAULT_SETTINGS, EMPTY_STATUS, type createApiMock } from "../test/apiMock";
import { createQueryClient } from "./queryClient";
import { useEvents } from "./useEvents";
import { useMenuShortcuts } from "./useMenuShortcuts";
import { useProjectStatus } from "./useProjectStatus";
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
  locked: false,
  fileError: null,
});

const menuItem = (key: string): MenuShortcut => ({ menu: "File", label: "Quit", command: "app:quit", defaultKey: "Mod+Q", key });

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

function renderReads() {
  return renderHook(
    () => {
      useEvents();
      return { projects: useProjects().data, settings: useSettings().data, menu: useMenuShortcuts().data };
    },
    { wrapper },
  );
}

describe("useEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.listProjects.mockResolvedValue([project("Demo Shop")]);
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, tags: ["client"] });
    mocked.menuShortcuts.mockResolvedValue([menuItem("Mod+Q")]);
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

  it("reads the menu's shortcuts again on settings_changed", async () => {
    const { result } = renderReads();
    await waitFor(() => expect(result.current.menu?.[0].key).toBe("Mod+Q"));
    mocked.menuShortcuts.mockResolvedValue([menuItem("Mod+J")]);

    act(() => emit("settings_changed", null));

    await waitFor(() => expect(result.current.menu?.[0].key).toBe("Mod+J"));
  });

  it("reads a project's status again on projects_changed", async () => {
    mocked.projectStatus.mockResolvedValue({ ...EMPTY_STATUS, states: [{ label: "Branch", value: "main" }] });
    const { result } = renderHook(
      () => {
        useEvents();
        return useProjectStatus("demo-1").data;
      },
      { wrapper },
    );
    await waitFor(() => expect(result.current?.states[0].value).toBe("main"));
    mocked.projectStatus.mockResolvedValue({ ...EMPTY_STATUS, states: [{ label: "Branch", value: "develop" }] });

    act(() => emit("projects_changed", null));

    await waitFor(() => expect(result.current?.states[0].value).toBe("develop"));
  });

  it("leaves the menu's shortcuts alone on projects_changed", async () => {
    const { result } = renderReads();
    await waitFor(() => expect(result.current.menu?.[0].key).toBe("Mod+Q"));
    await waitFor(() => expect(result.current.projects?.[0].name).toBe("Demo Shop"));
    mocked.menuShortcuts.mockResolvedValue([menuItem("Mod+J")]);
    mocked.listProjects.mockResolvedValue([project("Demo Shop Renamed")]);

    act(() => emit("projects_changed", null));

    await waitFor(() => expect(result.current.projects?.[0].name).toBe("Demo Shop Renamed"));
    expect(result.current.menu?.[0].key).toBe("Mod+Q");
  });
});
