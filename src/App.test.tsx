import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";
import { resetStore } from "./state/store";

const mocked = vi.mocked(api, true);

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
});
