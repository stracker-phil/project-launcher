import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { registerCoreSettingsPages } from "../../settings/pages";
import { resetStore } from "../../state/store";
import { DEFAULT_SETTINGS, type createApiMock } from "../../test/apiMock";
import { SettingsApp } from "./SettingsApp";

const mocked = vi.mocked(api, true);
const emit = (page: string) => act(() => (api as ReturnType<typeof createApiMock>).emit("open_settings", page));

async function renderSettings() {
  render(<SettingsApp client={createQueryClient()} />);
  await waitFor(() => expect(mocked.on).toHaveBeenCalledWith("open_settings", expect.any(Function)));
}

const nav = () => screen.getByRole("navigation", { name: "Settings pages" });
const search = () => screen.getByRole("searchbox", { name: "Search Settings" });

beforeAll(() => {
  registerCoreSettingsPages();
});

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

afterEach(() => {
  window.history.replaceState({}, "", "/");
});

describe("SettingsApp", () => {
  it("lists General, Appearance, Shortcuts, Status then About and opens on General", async () => {
    await renderSettings();

    expect(within(nav()).getAllByRole("button").map((b) => b.textContent?.trim())).toEqual(["General", "Appearance", "Shortcuts", "Status", "About"]);
    expect(screen.getByRole("heading", { level: 2, name: "General" })).toBeInTheDocument();
    expect(within(nav()).getByRole("button", { name: "General" })).toHaveAttribute("aria-current", "page");
  });

  it("shows the app fields and the login switch on General", async () => {
    await renderSettings();

    expect(await screen.findByRole("textbox", { name: "Default editor" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Open at login" })).toBeInTheDocument();
  });

  it("shows Appearance when it is chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(within(nav()).getByRole("button", { name: "Appearance" }));

    expect(screen.getByRole("heading", { level: 2, name: "Appearance" })).toBeInTheDocument();
    expect(within(nav()).getByRole("button", { name: "Appearance" })).toHaveAttribute("aria-current", "page");
  });

  it("shows Shortcuts when it is chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(within(nav()).getByRole("button", { name: "Shortcuts" }));

    expect(screen.getByRole("heading", { level: 2, name: "Shortcuts" })).toBeInTheDocument();
    expect(within(nav()).getByRole("button", { name: "Shortcuts" })).toHaveAttribute("aria-current", "page");
  });

  it("shows About when it is chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(within(nav()).getByRole("button", { name: "About" }));

    expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeInTheDocument();
    expect(within(nav()).getByRole("button", { name: "About" })).toHaveAttribute("aria-current", "page");
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

      await renderSettings();

      await waitFor(() => expect(scheme()).toBe(appearance));
    });

    it("applies the new appearance after settings_changed", async () => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: "dark" });
      await renderSettings();
      await waitFor(() => expect(scheme()).toBe("dark"));
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: "light" });

      act(() => (api as ReturnType<typeof createApiMock>).emit("settings_changed", null));

      await waitFor(() => expect(scheme()).toBe("light"));
    });
  });

  describe("opening at a page", () => {
    it("opens at the page named in the window address", async () => {
      window.history.replaceState({}, "", "/?page=about");

      await renderSettings();

      expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeInTheDocument();
    });

    it("opens the page holding the row named in the window address", async () => {
      window.history.replaceState({}, "", "/?page=cli");

      await renderSettings();

      expect(screen.getByRole("heading", { level: 2, name: "Status" })).toBeInTheDocument();
    });

    it("switches pages on the open_settings event", async () => {
      await renderSettings();

      emit("about");

      expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeInTheDocument();
    });
  });

  describe("search", () => {
    it("finds Status by its Log folder row", async () => {
      await renderSettings();

      await userEvent.type(search(), "log");

      expect(within(nav()).getByRole("button", { name: "Status Log folder" })).toBeInTheDocument();
      expect(within(nav()).queryByRole("button", { name: "About" })).not.toBeInTheDocument();
    });

    describe("with a menu command", () => {
      afterEach(() => {
        mocked.menuShortcuts.mockResolvedValue([]);
        registerCoreSettingsPages();
      });

      it("finds a menu command's row that arrives after the window rendered", async () => {
        await renderSettings();
        mocked.menuShortcuts.mockResolvedValue([{ menu: "File", label: "Quit", command: "app:quit", defaultKey: "Mod+Q", key: "Mod+Q" }]);
        act(() => registerCoreSettingsPages());

        await userEvent.type(search(), "quit");
        await screen.findByRole("button", { name: "Shortcuts Quit" });
        await userEvent.click(within(nav()).getByRole("button", { name: "Shortcuts Quit" }));

        expect(screen.getByRole("heading", { level: 2, name: "Shortcuts" })).toBeInTheDocument();
      });
    });

    it("finds General by its Open at login row", async () => {
      await renderSettings();

      await userEvent.type(search(), "open at");

      expect(within(nav()).getByRole("button", { name: "General Open at login" })).toBeInTheDocument();
    });

    it("finds General by its Docker app row", async () => {
      await renderSettings();

      await userEvent.type(search(), "docker app");

      expect(within(nav()).getByRole("button", { name: "General Docker app" })).toBeInTheDocument();
    });

    it("shows No Results when nothing matches", async () => {
      await renderSettings();

      await userEvent.type(search(), "zzz");

      expect(screen.getByText("No Results")).toBeInTheDocument();
      expect(within(nav()).queryByRole("button")).not.toBeInTheDocument();
    });

    it("opens the first hit on Enter", async () => {
      await renderSettings();

      await userEvent.type(search(), "about{Enter}");

      expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeInTheDocument();
    });
  });

  describe("keys", () => {
    it("closes the window on Escape outside the search field", async () => {
      await renderSettings();

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
    });

    it("keeps the window open on Escape inside the search field", async () => {
      await renderSettings();
      await userEvent.click(search());

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).not.toHaveBeenCalledWith("window:close");
    });

    it("focuses the search field on ⌘F", async () => {
      await renderSettings();

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(search()).toHaveFocus();
    });
  });
});
