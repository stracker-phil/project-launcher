import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type MenuShortcut } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { Toasts } from "../../components/Toasts";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { DEFAULT_SETTINGS } from "../../test/apiMock";
import { ShortcutsPage } from "./ShortcutsPage";

const mocked = vi.mocked(api, true);

const item = (menu: string, label: string, command: string, key: string | null, defaultKey: string | null = key): MenuShortcut => ({
  menu,
  label,
  command,
  key,
  defaultKey,
});

const MENU: MenuShortcut[] = [
  item("File", "Settings…", "app:settings", "Mod+,"),
  item("File", "Print", "app:print", "Mod+P"),
  item("Edit", "Undo", "edit:undo", "Mod+Z"),
  item("View", "Toggle Sidebar", "page:view.toggleSidebar", "Mod+B"),
];

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ShortcutsPage />
      <Toasts />
      <ConfirmHost />
    </QueryClientProvider>,
  );
}

const group = (name: string) => within(screen.getByRole("region", { name }));
const recorder = (title: string) => screen.findByRole("button", { name: `Record shortcut for ${title}` });
const searchField = () => screen.getByRole("searchbox", { name: "Search commands" });

/** The elements before a row's recorder: the reset and remove buttons, or the placeholders keeping their places. */
const slotsOf = (recorderButton: HTMLElement) => Array.from(recorderButton.parentElement?.children ?? []).filter((el) => el !== recorderButton);

async function record(title: string, keys: string) {
  await userEvent.click(await recorder(title));
  await userEvent.keyboard(keys);
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.menuShortcuts.mockResolvedValue(MENU);
  mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
  mocked.setShortcut.mockResolvedValue(undefined);
  mocked.resetShortcut.mockResolvedValue(undefined);
  mocked.reassignShortcut.mockResolvedValue(undefined);
  mocked.pauseMenuKeys.mockResolvedValue(undefined);
});

describe("ShortcutsPage", () => {
  describe("rows", () => {
    it("groups the rows by where the key works, under no menu name", async () => {
      renderPage();

      await recorder("Print");

      expect(screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent)).toEqual([
        "In every window",
        "In the main window",
        "In the Settings window",
      ]);
      expect(screen.queryByRole("region", { name: /^(Launcher|File|Edit|View|Main Window|Settings Window)$/ })).not.toBeInTheDocument();
    });

    it("lists every menu item in menu order under In every window, with its key", async () => {
      renderPage();

      await recorder("Print");

      const buttons = group("In every window").getAllByRole("button", { name: /^Record shortcut for / });
      expect(buttons.map((b) => b.getAttribute("aria-label"))).toEqual([
        "Record shortcut for Settings",
        "Record shortcut for Print",
        "Record shortcut for Undo",
        "Record shortcut for Toggle Sidebar",
      ]);
      expect(buttons.map((b) => b.textContent)).toEqual(["⌘,", "⌘P", "⌘Z", "⌘B"]);
    });

    it.each([
      ["Print", "File menu"],
      ["Undo", "Edit menu"],
      ["Toggle Sidebar", "View menu"],
    ])("names the menu of %s in its description", async (title, menu) => {
      renderPage();

      const row = (await recorder(title)).closest<HTMLElement>(".settings-row");

      expect(within(row!).getByText(menu)).toBeInTheDocument();
    });

    it("adds the page-only commands of the two windows and leaves out those a menu item runs", async () => {
      renderPage();

      await recorder("Toggle Inspector");

      expect(group("In the main window").getByText("Toggle Inspector")).toBeInTheDocument();
      expect(group("In the main window").queryByText("Toggle Sidebar")).not.toBeInTheDocument();
      expect(group("In every window").getByText("Toggle Sidebar")).toBeInTheDocument();
      expect(group("In the Settings window").getByRole("button", { name: "Record shortcut for Close Settings" })).toHaveTextContent("⎋");
      expect(group("In the Settings window").getByRole("button", { name: "Record shortcut for Search Settings" })).toHaveTextContent("⌘F");
    });

    it.each([
      ["In the main window", "Close Window", "⎋"],
      ["In the main window", "Hide Window", "⌘Q"],
      ["In the Settings window", "Close Settings", "⎋"],
      ["In the Settings window", "Hide Settings", "⌘Q"],
    ])("lists %s with %s on %s", async (region, title, key) => {
      renderPage();

      await recorder(title);

      expect(group(region).getByRole("button", { name: `Record shortcut for ${title}` })).toHaveTextContent(key);
    });

    it("says None for a command without a key", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("Edit", "Redo", "edit:redo", null, "Mod+Shift+Z")]);
      renderPage();

      expect(await recorder("Redo")).toHaveTextContent("None");
      expect(await recorder("Toggle Inspector")).toHaveTextContent("None");
    });

    it.each(["Toggle Inspector", "Close Settings"])("gives the page command %s no description", async (title) => {
      renderPage();

      const row = within((await recorder(title)).closest<HTMLElement>(".settings-row")!);

      expect(row.getByText(title)).toBeInTheDocument();
      expect(row.queryByText(/ menu$/)).not.toBeInTheDocument();
      expect(row.queryByText(/In the main window|In the Settings window/)).not.toBeInTheDocument();
    });

    it("shows an alert when the menu's keys cannot be read", async () => {
      mocked.menuShortcuts.mockRejectedValue({ kind: "io", message: "Permission denied", code: null });
      renderPage();

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not read the shortcuts: Permission denied");
    });
  });

  describe("reset", () => {
    it("is hidden but keeps its slot for a command on its default key", async () => {
      renderPage();
      const quit = await recorder("Print");

      expect(screen.queryByRole("button", { name: "Reset Print" })).not.toBeInTheDocument();
      expect(slotsOf(quit)).toHaveLength(2);
      expect(slotsOf(quit)[0]).toHaveAttribute("aria-hidden", "true");
    });

    it("is hidden for a menu command whose stored override equals its default", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+P", "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+P" } });
      renderPage();
      const quit = await recorder("Print");

      expect(screen.queryByRole("button", { name: "Reset Print" })).not.toBeInTheDocument();
      expect(slotsOf(quit)[0]).toHaveAttribute("aria-hidden", "true");
    });

    it("is hidden for a page command whose stored override equals its default", async () => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "settings.search": "Mod+F" } });
      renderPage();

      await recorder("Search Settings");

      expect(screen.queryByRole("button", { name: "Reset Search Settings" })).not.toBeInTheDocument();
    });

    it.each([["Mod+G"], [null]])("is shown for a page command stored on %s instead of its default", async (stored) => {
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "settings.search": stored } });
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Reset Search Settings" }));

      expect(mocked.resetShortcut).toHaveBeenCalledWith("settings.search");
    });

    it("restores the default of a command with the owner's key", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J" } });
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Reset Print" }));

      expect(mocked.resetShortcut).toHaveBeenCalledWith("app:print");
    });

    describe("when another command now holds the default key", () => {
      beforeEach(() => {
        mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P"), item("Edit", "Undo", "edit:undo", "Mod+P", "Mod+Z")]);
        mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J", "edit:undo": "Mod+P" } });
      });

      it("asks, then takes the key from that command", async () => {
        renderPage();

        await userEvent.click(await screen.findByRole("button", { name: "Reset Print" }));

        expect(await screen.findByRole("alertdialog", { name: "⌘P is used by “Undo”" })).toBeInTheDocument();
        await userEvent.click(screen.getByRole("button", { name: "Reassign" }));
        await waitFor(() => expect(mocked.resetShortcut).toHaveBeenCalledWith("app:print", "edit:undo"));
      });

      it("changes nothing when the question is cancelled", async () => {
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Reset Print" }));

        await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

        await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
        expect(mocked.resetShortcut).not.toHaveBeenCalled();
      });
    });

    it("shows an error toast when the reset fails", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J" } });
      mocked.resetShortcut.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Reset Print" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not reset the shortcut: Disk full");
    });
  });

  describe("remove", () => {
    it("leaves the command without a key", async () => {
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Remove shortcut for Print" }));

      expect(mocked.setShortcut).toHaveBeenCalledWith("app:print", null);
    });

    it("is hidden but keeps its slot for a command without a key", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("Edit", "Redo", "edit:redo", null, "Mod+Shift+Z")]);
      renderPage();
      const redo = await recorder("Redo");

      expect(screen.queryByRole("button", { name: "Remove shortcut for Redo" })).not.toBeInTheDocument();
      expect(slotsOf(redo)).toHaveLength(2);
      expect(slotsOf(redo)[1]).toHaveAttribute("aria-hidden", "true");
    });

    it("sits beside the reset button, both shown for a command on another key", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J" } });
      renderPage();
      const quit = await recorder("Print");

      expect(slotsOf(quit).map((slot) => slot.getAttribute("aria-label"))).toEqual(["Reset Print", "Remove shortcut for Print"]);
    });

    it("shows an error toast when it fails", async () => {
      mocked.setShortcut.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Remove shortcut for Print" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the shortcut: Disk full");
    });
  });

  describe("search", () => {
    it("narrows the rows to the commands whose title contains the text", async () => {
      renderPage();
      await recorder("Print");

      await userEvent.type(searchField(), "UNDO");

      expect(screen.getByRole("button", { name: "Record shortcut for Undo" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Record shortcut for Print" })).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "In the main window" })).not.toBeInTheDocument();
    });

    it("also matches the menu named in a row's description", async () => {
      renderPage();
      await recorder("Print");

      await userEvent.type(searchField(), "file menu");

      expect(screen.getByRole("button", { name: "Record shortcut for Print" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Record shortcut for Settings" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Record shortcut for Undo" })).not.toBeInTheDocument();
    });

    it("keeps all the rows of a group whose name matches", async () => {
      renderPage();
      await recorder("Print");

      await userEvent.type(searchField(), "settings window");

      expect(group("In the Settings window").getByRole("button", { name: "Record shortcut for Close Settings" })).toBeInTheDocument();
      expect(group("In the Settings window").getByRole("button", { name: "Record shortcut for Search Settings" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Record shortcut for Print" })).not.toBeInTheDocument();
    });

    it("says No matching commands. when nothing matches", async () => {
      renderPage();
      await recorder("Print");

      await userEvent.type(searchField(), "zzz");

      expect(screen.getByText("No matching commands.")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Record shortcut for/ })).not.toBeInTheDocument();
    });

    it("is cleared by the first Escape, which brings every row back", async () => {
      renderPage();
      await recorder("Print");
      await userEvent.type(searchField(), "zzz");

      await userEvent.keyboard("{Escape}");

      expect(searchField()).toHaveValue("");
      expect(screen.queryByText("No matching commands.")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Record shortcut for Print" })).toBeInTheDocument();
    });
  });

  describe("recording", () => {
    it("pauses the menu's keys while recording and restores them afterwards", async () => {
      renderPage();

      await userEvent.click(await recorder("Print"));

      expect(await screen.findByRole("button", { name: "Press the new shortcut for Print" })).toHaveTextContent("Press keys…");
      expect(mocked.pauseMenuKeys).toHaveBeenLastCalledWith(true);
      await userEvent.keyboard("{Meta>}j{/Meta}");
      await waitFor(() => expect(mocked.pauseMenuKeys).toHaveBeenLastCalledWith(false));
    });

    it("saves the recorded key", async () => {
      renderPage();

      await record("Print", "{Meta>}{Shift>}j{/Shift}{/Meta}");

      expect(mocked.setShortcut).toHaveBeenCalledWith("app:print", "Mod+Shift+J");
    });

    it("cancels on Escape without saving", async () => {
      renderPage();

      await record("Print", "{Escape}");

      expect(await recorder("Print")).toHaveTextContent("⌘P");
      expect(mocked.setShortcut).not.toHaveBeenCalled();
      expect(mocked.reassignShortcut).not.toHaveBeenCalled();
    });

    it.each(["{Backspace}", "{Delete}"])("leaves the command without a key on %s", async (keys) => {
      renderPage();

      await record("Print", keys);

      expect(mocked.setShortcut).toHaveBeenCalledWith("app:print", null);
    });

    it("does nothing when the command already has the key", async () => {
      renderPage();

      await record("Print", "{Meta>}p{/Meta}");

      expect(mocked.setShortcut).not.toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });

    it("resets a command to its default when its default key is recorded", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J" } });
      renderPage();

      await record("Print", "{Meta>}p{/Meta}");

      await waitFor(() => expect(mocked.resetShortcut).toHaveBeenCalledWith("app:print"));
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("resets an unbound command when its default key is recorded", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", null, "Mod+P")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": null } });
      renderPage();

      await record("Print", "{Meta>}p{/Meta}");

      await waitFor(() => expect(mocked.resetShortcut).toHaveBeenCalledWith("app:print"));
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("asks before resetting to a default key another command holds, then takes it", async () => {
      mocked.menuShortcuts.mockResolvedValue([item("File", "Print", "app:print", "Mod+J", "Mod+P"), item("Edit", "Undo", "edit:undo", "Mod+P", "Mod+Z")]);
      mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, shortcuts: { "app:print": "Mod+J", "edit:undo": "Mod+P" } });
      renderPage();

      await record("Print", "{Meta>}p{/Meta}");

      expect(await screen.findByRole("alertdialog", { name: "⌘P is used by “Undo”" })).toBeInTheDocument();
      await userEvent.click(screen.getByRole("button", { name: "Reassign" }));
      await waitFor(() => expect(mocked.resetShortcut).toHaveBeenCalledWith("app:print", "edit:undo"));
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("shows an error toast when saving fails", async () => {
      mocked.setShortcut.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();

      await record("Print", "{Meta>}j{/Meta}");

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the shortcut: Disk full");
    });

    it("refuses a menu key without ⌘ or ⌃ and says why", async () => {
      renderPage();

      await record("Print", "j");

      expect(await screen.findByRole("alert")).toHaveTextContent("J can't be a menu shortcut: it needs ⌘ or ⌃.");
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("accepts a letter alone for a page command", async () => {
      renderPage();

      await record("Toggle Inspector", "j");

      expect(mocked.setShortcut).toHaveBeenCalledWith("view.toggleInspector", "J");
    });
  });

  describe("conflicts", () => {
    it("asks before giving a key another command has, then reassigns it", async () => {
      renderPage();

      await record("Print", "{Meta>}z{/Meta}");

      expect(await screen.findByRole("alertdialog", { name: "⌘Z is used by “Undo”" })).toHaveAccessibleDescription(
        "Give it to “Print” instead? “Undo” is left without a shortcut.",
      );
      await userEvent.click(screen.getByRole("button", { name: "Reassign" }));
      await waitFor(() => expect(mocked.reassignShortcut).toHaveBeenCalledWith("app:print", "Mod+Z", "edit:undo"));
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("changes nothing when the question is cancelled", async () => {
      renderPage();
      await record("Print", "{Meta>}z{/Meta}");

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
      expect(mocked.reassignShortcut).not.toHaveBeenCalled();
      expect(mocked.setShortcut).not.toHaveBeenCalled();
    });

    it("asks when a page key is taken by a command of the same window", async () => {
      renderPage();

      await record("Close Settings", "{Meta>}f{/Meta}");

      expect(await screen.findByRole("alertdialog", { name: "⌘F is used by “Search Settings”" })).toBeInTheDocument();
    });

    it("does not ask when a page key is taken by a command of the other window", async () => {
      renderPage();

      await record("Toggle Inspector", "{Meta>}f{/Meta}");

      await waitFor(() => expect(mocked.setShortcut).toHaveBeenCalledWith("view.toggleInspector", "Mod+F"));
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });

    it("asks when a page key is taken by a menu item", async () => {
      renderPage();

      await record("Toggle Inspector", "{Meta>}z{/Meta}");

      expect(await screen.findByRole("alertdialog", { name: "⌘Z is used by “Undo”" })).toBeInTheDocument();
    });
  });
});
