import { act, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";

const mocked = vi.mocked(api, true);

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.takeLaunchEvents.mockResolvedValue([]);
  });

  it("logs the message of an error launch event through api.logError", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Could not open the demo project" }]);

    render(<App />);

    await waitFor(() => expect(mocked.logError).toHaveBeenCalledWith("Could not open the demo project"));
  });

  it("logs nothing for an activate launch event", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "activate" }]);

    render(<App />);
    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
    await act(async () => {});

    expect(mocked.logError).not.toHaveBeenCalled();
  });
});
