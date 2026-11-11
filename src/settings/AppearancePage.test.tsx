import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { Toasts } from "../components/Toasts";
import { resetStore } from "../state/store";
import { DEFAULT_SETTINGS } from "../test/apiMock";
import { AppearancePage } from "./AppearancePage";

const mocked = vi.mocked(api, true);

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AppearancePage />
      <Toasts />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
  mocked.setAppearance.mockResolvedValue(undefined);
});

describe("AppearancePage", () => {
  it.each([
    ["system", "Automatic"],
    ["light", "Light"],
    ["dark", "Dark"],
  ] as const)("presses only the %s segment when the setting is %s", async (appearance, label) => {
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance });
    renderPage();

    expect(await screen.findByRole("button", { name: label, pressed: true })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { pressed: true })).toHaveLength(1);
  });

  it("saves the clicked appearance", async () => {
    renderPage();

    await userEvent.click(await screen.findByRole("button", { name: "Dark" }));

    expect(mocked.setAppearance).toHaveBeenCalledWith("dark");
  });

  it("shows an error toast when saving fails", async () => {
    mocked.setAppearance.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderPage();

    await userEvent.click(await screen.findByRole("button", { name: "Light" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the appearance: Disk full");
  });
});
