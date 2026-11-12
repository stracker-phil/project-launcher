import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { registerCoreSettingsPages } from "../../settings/pages";
import { resetStore, useStore } from "../../state/store";
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
  it("lists General, Appearance, Shortcuts, Status, Privacy & Data then About and opens on General", async () => {
    await renderSettings();

    expect(within(nav()).getAllByRole("button").map((b) => b.textContent?.trim())).toEqual([
      "General",
      "Appearance",
      "Shortcuts",
      "Status",
      "Privacy & Data",
      "About",
    ]);
    expect(screen.getByRole("heading", { level: 2, name: "General" })).toBeInTheDocument();
    expect(within(nav()).getByRole("button", { name: "General" })).toHaveAttribute("aria-current", "page");
  });

  it("groups the pages under the headings App and System", async () => {
    await renderSettings();

    const pagesOf = (group: string) => within(within(nav()).getByRole("group", { name: group })).getAllByRole("button").map((b) => b.textContent?.trim());
    expect(pagesOf("App")).toEqual(["General", "Appearance", "Shortcuts"]);
    expect(pagesOf("System")).toEqual(["Status", "Privacy & Data", "About"]);
    expect(within(nav()).getByText("App")).toBeInTheDocument();
    expect(within(nav()).getByText("System")).toBeInTheDocument();
  });

  it("shows the page's intro under its title", async () => {
    await renderSettings();
    expect(screen.getByText("The apps projects open with, Docker, and how Launcher starts.")).toBeInTheDocument();

    await userEvent.click(within(nav()).getByRole("button", { name: "Appearance" }));

    expect(screen.getByText("How every Launcher window looks.")).toBeInTheDocument();
    expect(screen.queryByText("The apps projects open with, Docker, and how Launcher starts.")).not.toBeInTheDocument();
  });

  it("shows Privacy & Data when it is chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(within(nav()).getByRole("button", { name: "Privacy & Data" }));

    expect(screen.getByRole("heading", { level: 2, name: "Privacy & Data" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Show Data Folder" })).toBeInTheDocument();
  });

  it("opens the Docker app menu in the Settings window", async () => {
    await renderSettings();
    const dockerApp = await screen.findByRole("button", { name: "Docker app" });
    await waitFor(() => expect(dockerApp).toBeEnabled());

    await userEvent.click(dockerApp);

    expect(within(screen.getByRole("menu", { name: "Docker app" })).getByRole("menuitemradio", { name: "Docker Desktop" })).toHaveAttribute("aria-checked", "true");
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

    const accent = () => document.documentElement.dataset.accent;

    beforeEach(() => {
      delete document.documentElement.dataset.scheme;
      delete document.documentElement.dataset.accent;
    });

    it.each(["teal", "graphite"] as const)("applies the stored %s accent", async (hue) => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, accent: hue });

      await renderSettings();

      await waitFor(() => expect(accent()).toBe(hue));
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
    const hit = (name: string) => within(nav()).getByRole("button", { name });
    const row = (anchor: string) => document.querySelector(`[data-anchor="${anchor}"]`);

    it("replaces the page list with one Results group while there is text", async () => {
      await renderSettings();

      await userEvent.type(search(), "log");

      expect(within(nav()).getByRole("group", { name: "Results" })).toBeInTheDocument();
      expect(within(nav()).queryByRole("group", { name: "App" })).not.toBeInTheDocument();
      expect(within(nav()).queryByRole("group", { name: "System" })).not.toBeInTheDocument();
    });

    it("lists a row hit with its page title first and the row title below, named with both", async () => {
      await renderSettings();

      await userEvent.type(search(), "log");

      expect(hit("Log folder, Status")).toHaveTextContent("StatusLog folder");
      expect(within(hit("Log folder, Status")).getByText("Status")).toBeInTheDocument();
      expect(within(hit("Log folder, Status")).getByText("Log folder")).toBeInTheDocument();
      expect(within(nav()).queryByRole("button", { name: "General" })).not.toBeInTheDocument();
    });

    it("lists the rows of different pages that share a title", async () => {
      await renderSettings();

      await userEvent.type(search(), "version");

      expect(hit("Version, Status")).toBeInTheDocument();
      expect(hit("Version, About")).toBeInTheDocument();
    });

    it("finds a row by one of its keywords", async () => {
      await renderSettings();

      await userEvent.type(search(), "colima");

      expect(hit("Docker app, General")).toBeInTheDocument();
    });

    it("lists a page hit by the page title alone", async () => {
      await renderSettings();

      await userEvent.type(search(), "appearance");

      expect(hit("Appearance")).toBeInTheDocument();
    });

    it("brings the page list back when the search is cleared", async () => {
      await renderSettings();
      await userEvent.type(search(), "log");

      await userEvent.clear(search());

      expect(within(nav()).getByRole("group", { name: "App" })).toBeInTheDocument();
      expect(within(nav()).queryByRole("group", { name: "Results" })).not.toBeInTheDocument();
    });

    describe("opening a hit", () => {
      afterEach(() => {
        mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
      });

      it("opens the hit's page and highlights its row", async () => {
        await renderSettings();
        await userEvent.type(search(), "log folder");

        await userEvent.click(hit("Log folder, Status"));

        expect(screen.getByRole("heading", { level: 2, name: "Status" })).toBeInTheDocument();
        await waitFor(() => expect(row("logs")).toHaveClass("found"));
      });

      it("highlights the row again when the same hit is chosen again", async () => {
        await renderSettings();
        await userEvent.type(search(), "log folder");
        await userEvent.click(hit("Log folder, Status"));
        await waitFor(() => expect(row("logs")).toHaveClass("found"));
        row("logs")?.classList.remove("found");

        await userEvent.click(hit("Log folder, Status"));

        await waitFor(() => expect(row("logs")).toHaveClass("found"));
      });

      it("highlights a group when its hit is chosen", async () => {
        await renderSettings();
        await userEvent.type(search(), "network");

        await userEvent.click(hit("Network, Privacy & Data"));

        expect(screen.getByRole("heading", { level: 2, name: "Privacy & Data" })).toBeInTheDocument();
        await waitFor(() => expect(row("network")).toHaveClass("found"));
      });

      it("highlights the Docker app row for a start command that only shows with a custom Docker app", async () => {
        await renderSettings();
        await userEvent.type(search(), "start command");

        await userEvent.click(hit("Start command, General"));

        await waitFor(() => expect(row("docker")).toHaveClass("found"));
        expect(row("dockerStartCommand")).toBeNull();
      });

      it("highlights the start command itself when the Docker app is custom", async () => {
        mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, dockerManager: "custom" });
        await renderSettings();
        await userEvent.type(search(), "start command");

        await userEvent.click(hit("Start command, General"));

        await waitFor(() => expect(row("dockerStartCommand")).toHaveClass("found"));
        await act(async () => {
          await new Promise((resolve) => setTimeout(resolve, 20));
        });
        expect(row("dockerStartCommand")).toHaveClass("found");
        expect(row("docker")).not.toHaveClass("found");
      });

      it("moves the highlight from the Docker app row to the start command once the Docker app becomes custom", async () => {
        await renderSettings();
        await userEvent.type(search(), "start command");
        await userEvent.click(hit("Start command, General"));
        await waitFor(() => expect(row("docker")).toHaveClass("found"));
        mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, dockerManager: "custom" });

        act(() => (api as ReturnType<typeof createApiMock>).emit("settings_changed", null));

        await waitFor(() => expect(row("dockerStartCommand")).toHaveClass("found"));
        expect(row("docker")).not.toHaveClass("found");
      });

      it("opens the page without highlighting anything when a page hit is chosen", async () => {
        await renderSettings();
        await userEvent.type(search(), "appearance");

        await userEvent.click(hit("Appearance"));

        expect(screen.getByRole("heading", { level: 2, name: "Appearance" })).toBeInTheDocument();
        expect(document.querySelector(".found")).toBeNull();
      });
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
        await screen.findByRole("button", { name: "Quit, Shortcuts" });
        await userEvent.click(within(nav()).getByRole("button", { name: "Quit, Shortcuts" }));

        expect(screen.getByRole("heading", { level: 2, name: "Shortcuts" })).toBeInTheDocument();
      });
    });

    it("finds the pages of a group by its name", async () => {
      await renderSettings();

      await userEvent.type(search(), "system");

      const names = within(nav()).getAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent?.trim());
      expect(names).toEqual(expect.arrayContaining(["Status", "Privacy & Data", "About"]));
      expect(names).not.toContain("General");
      expect(names).not.toContain("Appearance");
    });

    it("finds Privacy & Data by its Network row", async () => {
      await renderSettings();

      await userEvent.type(search(), "network");

      expect(hit("Network, Privacy & Data")).toBeInTheDocument();
    });

    it("finds General by its Open at login row", async () => {
      await renderSettings();

      await userEvent.type(search(), "open at");

      expect(hit("Open at login, General")).toBeInTheDocument();
    });

    it("finds General by its Docker app row", async () => {
      await renderSettings();

      await userEvent.type(search(), "docker app");

      expect(hit("Docker app, General")).toBeInTheDocument();
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

    it("opens the first hit's row on Enter", async () => {
      await renderSettings();

      await userEvent.type(search(), "log folder{Enter}");

      expect(screen.getByRole("heading", { level: 2, name: "Status" })).toBeInTheDocument();
      await waitFor(() => expect(row("logs")).toHaveClass("found"));
    });

    it("does nothing on Enter while the search is empty", async () => {
      await renderSettings();
      await userEvent.click(within(nav()).getByRole("button", { name: "About" }));
      await userEvent.click(search());

      await userEvent.keyboard("{Enter}");

      expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeInTheDocument();
    });

    it("does nothing on Enter when nothing matches", async () => {
      await renderSettings();

      await userEvent.type(search(), "zzz{Enter}");

      expect(screen.getByRole("heading", { level: 2, name: "General" })).toBeInTheDocument();
    });
  });

  describe("opening at a row", () => {
    const row = (anchor: string) => document.querySelector(`[data-anchor="${anchor}"]`);

    it("highlights the row named in the open_settings event", async () => {
      await renderSettings();

      emit("logs");

      expect(screen.getByRole("heading", { level: 2, name: "Status" })).toBeInTheDocument();
      await waitFor(() => expect(row("logs")).toHaveClass("found"));
    });

    it("highlights the row again when the event repeats", async () => {
      await renderSettings();
      emit("logs");
      await waitFor(() => expect(row("logs")).toHaveClass("found"));
      row("logs")?.classList.remove("found");

      emit("logs");

      await waitFor(() => expect(row("logs")).toHaveClass("found"));
    });

    it("highlights the Docker app row, which holds the Docker app menu", async () => {
      await renderSettings();

      emit("docker");

      await waitFor(() => expect(row("docker")).toHaveClass("found"));
      expect(within(row("docker") as HTMLElement).getByRole("button", { name: "Docker app" })).toBeInTheDocument();
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

    it("keeps the window open on Escape while a confirmation is open", async () => {
      await renderSettings();
      act(() => void useStore.getState().confirm({ title: "Remove it?", text: "This removes it.", action: "Remove" }));
      await screen.findByRole("alertdialog");

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).not.toHaveBeenCalledWith("window:close");
    });

    it("focuses the search field on ⌘F", async () => {
      await renderSettings();

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(search()).toHaveFocus();
    });

    it("selects the search text on ⌘F", async () => {
      await renderSettings();
      await userEvent.type(search(), "log");
      await userEvent.click(document.body);

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(search()).toHaveFocus();
      const field = search() as HTMLInputElement;
      expect(field.selectionStart).toBe(0);
      expect(field.selectionEnd).toBe(3);
    });

    it("clears the search on the first Escape inside the search field and keeps Settings open", async () => {
      await renderSettings();
      await userEvent.type(search(), "log");

      await userEvent.keyboard("{Escape}");

      expect(search()).toHaveValue("");
      expect(mocked.runCommand).not.toHaveBeenCalledWith("window:close");
    });
  });
});
