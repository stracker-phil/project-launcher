import { act, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";
import { clearCommands } from "./commands/registry";
import { createQueryClient } from "./queries/queryClient";
import { DEFAULT_SETTINGS, EMPTY_LIBRARY, type createApiMock } from "./test/apiMock";
import { library, project, section } from "./test/library";
import { resetLibrary } from "./state/library";
import { resetStore } from "./state/store";

const mocked = vi.mocked(api, true) as ReturnType<typeof createApiMock> & ReturnType<typeof vi.mocked<typeof api>>;

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
    resetLibrary();
    mocked.takeLaunchEvents.mockResolvedValue([]);
    mocked.library.mockResolvedValue(EMPTY_LIBRARY);
  });

  it("renders the shell with the project detail", () => {
    render(<App />);

    expect(screen.getByRole("main", { name: "Project" })).toBeInTheDocument();
  });

  it("shows the library's title and project rows, and the first project in the detail", async () => {
    mocked.library.mockResolvedValue(library([section(null, project({ name: "Alpha" }))], { title: "All Projects", countLabel: "1 project" }));

    render(<App client={createQueryClient()} />);

    expect(await screen.findByRole("heading", { level: 1, name: "All Projects" })).toBeInTheDocument();
    expect(screen.getByText("1 project")).toBeInTheDocument();
    const row = await screen.findByRole("option", { name: /^Alpha/ });
    await waitFor(() => expect(row).toHaveAttribute("aria-selected", "true"));
    expect(within(screen.getByRole("main", { name: "Project" })).getByRole("button", { name: "Project name" })).toHaveTextContent("Alpha");
  });

  it("shows an error launch event as an error toast and logs it through api.logError", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Could not open the demo project" }]);

    render(<App />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Something went wrong");
    expect(alert).toHaveTextContent("Could not open the demo project");
    expect(mocked.logError).toHaveBeenCalledWith("Could not open the demo project");
  });

  it("shows no toast and logs nothing for an activate launch event", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "activate" }]);

    render(<App />);
    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
    await act(async () => {});

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(mocked.logError).not.toHaveBeenCalled();
  });

  describe("appearance", () => {
    const scheme = () => document.documentElement.dataset.scheme;

    beforeEach(() => {
      delete document.documentElement.dataset.scheme;
    });

    afterEach(() => {
      mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
    });

    it.each(["light", "dark"] as const)("applies the stored %s appearance", async (appearance) => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance });

      render(<App client={createQueryClient()} />);

      await waitFor(() => expect(scheme()).toBe(appearance));
    });

    it("applies the new appearance after settings_changed", async () => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: "dark" });
      render(<App client={createQueryClient()} />);
      await waitFor(() => expect(scheme()).toBe("dark"));
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: "light" });

      act(() => mocked.emit("settings_changed", null));

      await waitFor(() => expect(scheme()).toBe("light"));
    });
  });

  describe("page commands from the menus", () => {
    const sidebar = () => screen.queryByRole("navigation", { name: "Sidebar" });
    const inspector = () => screen.queryByRole("complementary", { name: "Inspector" });

    beforeEach(() => {
      clearCommands();
    });

    it("hides the sidebar when the View menu sends view.toggleSidebar", () => {
      render(<App />);
      expect(sidebar()).toBeInTheDocument();

      act(() => mocked.emit("page_command", "view.toggleSidebar"));

      expect(sidebar()).not.toBeInTheDocument();
    });

    it("shows the inspector when the View menu sends view.toggleInspector", () => {
      render(<App />);
      expect(inspector()).not.toBeInTheDocument();

      act(() => mocked.emit("page_command", "view.toggleInspector"));

      expect(inspector()).toBeInTheDocument();
    });

    it("changes nothing for an unknown command id", () => {
      render(<App />);

      act(() => mocked.emit("page_command", "view.unknown"));

      expect(sidebar()).toBeInTheDocument();
      expect(inspector()).not.toBeInTheDocument();
    });
  });
});
