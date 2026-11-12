import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
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
  mocked.setAccent.mockResolvedValue(undefined);
});

describe("AppearancePage", () => {
  it.each([
    ["system", "Automatic"],
    ["light", "Light"],
    ["dark", "Dark"],
  ] as const)("presses only the %s tile when the setting is %s", async (appearance, label) => {
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance });
    renderPage();

    const tiles = within(await screen.findByRole("group", { name: "Appearance" }));
    expect(await tiles.findByRole("button", { name: label, pressed: true })).toBeInTheDocument();
    expect(tiles.getAllByRole("button")).toHaveLength(3);
    expect(tiles.getAllByRole("button", { pressed: true })).toHaveLength(1);
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

  describe("accent color", () => {
    const swatches = async () => within(await screen.findByRole("group", { name: "Accent color" }));

    it("offers System and the nine hues", async () => {
      renderPage();

      const group = await swatches();

      expect(group.getAllByRole("button").map((b) => b.getAttribute("aria-label"))).toEqual([
        "System",
        "Orange",
        "Gold",
        "Red",
        "Magenta",
        "Purple",
        "Indigo",
        "Blue",
        "Teal",
        "Graphite",
      ]);
    });

    it.each([
      ["system", "System"],
      ["teal", "Teal"],
    ] as const)("presses only the %s swatch when the accent is %s", async (accent, label) => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, accent });
      renderPage();

      const group = await swatches();

      await waitFor(() => expect(group.getByRole("button", { name: label })).toHaveAttribute("aria-pressed", "true"));
      expect(group.getAllByRole("button", { pressed: true })).toHaveLength(1);
    });

    it.each([
      ["Orange", "orange"],
      ["Graphite", "graphite"],
    ] as const)("saves the %s swatch as the accent %s", async (label, hue) => {
      renderPage();

      await userEvent.click((await swatches()).getByRole("button", { name: label }));

      expect(mocked.setAccent).toHaveBeenCalledWith(hue);
    });

    it("saves System to follow macOS again", async () => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, accent: "red" });
      renderPage();
      const group = await swatches();
      await waitFor(() => expect(group.getByRole("button", { name: "Red" })).toHaveAttribute("aria-pressed", "true"));

      await userEvent.click(group.getByRole("button", { name: "System" }));

      expect(mocked.setAccent).toHaveBeenCalledWith("system");
    });

    it("shows an error toast when saving fails", async () => {
      mocked.setAccent.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();

      await userEvent.click((await swatches()).getByRole("button", { name: "Blue" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the accent color: Disk full");
    });
  });
});
