import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetLibrary } from "../state/library";
import { resetStore, useStore } from "../state/store";
import { EMPTY_LIBRARY } from "../test/apiMock";
import { library, project, section } from "../test/library";
import { quickStats } from "../test/stats";
import { Shell } from "./Shell";

const mocked = vi.mocked(api, true);

const sidebar = () => screen.queryByRole("navigation", { name: "Sidebar" });
const list = () => screen.queryByRole("region", { name: "Projects" });
const detail = () => screen.queryByRole("main", { name: "Project" });
const inspector = () => screen.queryByRole("complementary", { name: "Inspector" });

async function renderShell() {
  await act(async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <Shell />
      </QueryClientProvider>,
    );
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.library.mockResolvedValue(EMPTY_LIBRARY);
  resetStore();
  resetLibrary();
});

describe("Shell", () => {
  it("shows the sidebar, the project list and the detail, without the inspector, at first", async () => {
    await renderShell();

    expect(sidebar()).toBeInTheDocument();
    expect(list()).toBeInTheDocument();
    expect(detail()).toBeInTheDocument();
    expect(inspector()).not.toBeInTheDocument();
  });

  it("shows the inspector once it is toggled open, and hides it when toggled again", async () => {
    await renderShell();

    act(() => useStore.getState().toggleInspector());
    expect(inspector()).toBeInTheDocument();

    act(() => useStore.getState().toggleInspector());
    expect(inspector()).not.toBeInTheDocument();
  });

  it("says in the inspector that no project is selected when the library is empty", async () => {
    resetStore({ inspectorOpen: { withSidebar: true, withoutSidebar: true } });
    await renderShell();

    expect(within(inspector()!).getByText("No Project Selected")).toBeInTheDocument();
  });

  it("shows the selected project's details in the inspector", async () => {
    mocked.library.mockResolvedValue(library([section(null, project({ name: "Alpha" }))]));
    mocked.quickStats.mockResolvedValue(quickStats({ kind: "Node project" }));
    resetStore({ inspectorOpen: { withSidebar: true, withoutSidebar: true } });
    await renderShell();

    expect(await within(inspector()!).findByText("Node project")).toBeInTheDocument();
    expect(within(inspector()!).getByRole("button", { name: "Refresh" })).toBeInTheDocument();
  });

  it("shows only the detail and the inspector when the sidebar is hidden", async () => {
    await renderShell();

    act(() => useStore.getState().toggleSidebar());

    expect(sidebar()).not.toBeInTheDocument();
    expect(detail()).toBeInTheDocument();
    expect(inspector()).toBeInTheDocument();
  });

  it("keeps the selected project in the detail while the sidebar is hidden", async () => {
    mocked.library.mockResolvedValue(library([section(null, project({ name: "Alpha" }))]));
    await renderShell();
    expect(await within(detail()!).findByRole("button", { name: "Project name" })).toHaveTextContent("Alpha");

    act(() => useStore.getState().toggleSidebar());

    expect(within(detail()!).getByRole("button", { name: "Project name" })).toHaveTextContent("Alpha");
  });

  it("brings the sidebar and the list back when the sidebar is shown again", async () => {
    resetStore({ sidebarHidden: true });
    await renderShell();

    act(() => useStore.getState().toggleSidebar());

    expect(sidebar()).toBeInTheDocument();
    expect(list()).toBeInTheDocument();
  });
});
