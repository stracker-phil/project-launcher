import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";
import { clearCommands } from "./commands/registry";
import type { createApiMock } from "./test/apiMock";
import { resetStore } from "./state/store";

const mocked = vi.mocked(api, true) as ReturnType<typeof createApiMock> & ReturnType<typeof vi.mocked<typeof api>>;

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
    mocked.takeLaunchEvents.mockResolvedValue([]);
  });

  it("renders the shell with the project detail", () => {
    render(<App />);

    expect(screen.getByRole("main", { name: "Project" })).toBeInTheDocument();
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
