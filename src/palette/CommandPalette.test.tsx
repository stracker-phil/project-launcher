import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuCommand } from "../api";
import { useMainCommands } from "../commands";
import { clearCommands, registerCommands, runPageCommand } from "../commands/registry";
import { MAIN_COMMANDS } from "../commands/mainCommands";
import { resetStore, useStore } from "../state/store";
import type { createApiMock } from "../test/apiMock";
import { CommandPalette } from "./CommandPalette";
import { clearRecentSearches } from "./recent";

const mocked = vi.mocked(api, true);
const events = api as unknown as ReturnType<typeof createApiMock>;

const ROWS: MenuCommand[] = [
  { menu: "File", label: "New Project", command: "app:new", key: "⌘N" },
  { menu: "File", label: "Open Folder", command: "app:open", key: null },
  { menu: "View", label: "Week View", command: "app:week", key: "⌘2" },
];

function Host() {
  useMainCommands();
  return (
    <>
      <button>Opener</button>
      <CommandPalette />
    </>
  );
}

function renderHost() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <Host />
    </QueryClientProvider>,
  );
}

const togglePalette = () => act(() => void runPageCommand("palette.open"));

async function openPalette() {
  renderHost();
  togglePalette();
  await screen.findByRole("option", { name: /New Project/ });
}

const selectedOption = () => screen.getByRole("option", { selected: true });
const field = () => screen.getByRole("combobox");

describe("CommandPalette", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
    clearCommands();
    clearRecentSearches();
    registerCommands(MAIN_COMMANDS);
    mocked.menuCommands.mockResolvedValue(ROWS);
    mocked.runCommand.mockResolvedValue(undefined);
  });

  describe("opening and closing", () => {
    it("opens with the palette command and closes again with the same command", async () => {
      await openPalette();
      expect(screen.getByRole("dialog", { name: "Command palette" })).toBeInTheDocument();

      togglePalette();

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("opens when Rust reports the palette's page command", async () => {
      renderHost();
      await act(async () => {});

      act(() => events.emit("page_command", "palette.open"));

      expect(await screen.findByRole("dialog", { name: "Command palette" })).toBeInTheDocument();
    });

    it("closes with Escape", async () => {
      await openPalette();

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("closes when the dimmed area around it is pressed, not when the panel is", async () => {
      await openPalette();

      fireEvent.mouseDown(screen.getByRole("dialog"));
      expect(screen.getByRole("dialog")).toBeInTheDocument();

      fireEvent.mouseDown(screen.getByRole("dialog").parentElement!);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("gives the focus back to the element that had it before", async () => {
      renderHost();
      screen.getByRole("button", { name: "Opener" }).focus();
      togglePalette();
      await screen.findByRole("option", { name: /New Project/ });
      expect(field()).toHaveFocus();

      await userEvent.keyboard("{Escape}");

      expect(screen.getByRole("button", { name: "Opener" })).toHaveFocus();
    });
  });

  describe("results", () => {
    it("lists the menus' commands under their menu and the page's commands under Window", async () => {
      await openPalette();

      expect(screen.getByRole("region", { name: "File" })).toHaveTextContent("Open Folder");
      expect(screen.getByRole("region", { name: "View" })).toHaveTextContent("Week View");
      expect(screen.getByRole("region", { name: "Window" })).toHaveTextContent("Toggle Sidebar");
    });

    it("does not list the palette command itself", async () => {
      await openPalette();

      expect(screen.queryByText("Command Palette")).not.toBeInTheDocument();
    });

    it("shows the key of a command as a chip", async () => {
      await openPalette();

      expect(screen.getByText("⌘N")).toBeInTheDocument();
      expect(screen.getByText("⌘2")).toBeInTheDocument();
    });

    it("narrows the list to the best matches while typing", async () => {
      await openPalette();

      await userEvent.type(field(), "week");

      expect(screen.getAllByRole("option")).toHaveLength(1);
      expect(screen.getByRole("region", { name: "Commands" })).toHaveTextContent("Week View");
    });

    it("says there are no results when nothing matches", async () => {
      await openPalette();

      await userEvent.type(field(), "zzzz");

      expect(screen.getByText("No results")).toBeInTheDocument();
      expect(screen.queryByRole("option")).not.toBeInTheDocument();
    });

    it("finds nothing for a prefix that has no source yet", async () => {
      await openPalette();

      await userEvent.type(field(), ":");

      expect(screen.getByText("No results")).toBeInTheDocument();
    });
  });

  describe("keyboard", () => {
    it("starts on the first row and moves down and up", async () => {
      await openPalette();
      expect(selectedOption()).toHaveTextContent("New Project");

      await userEvent.keyboard("{ArrowDown}");
      expect(selectedOption()).toHaveTextContent("Open Folder");

      await userEvent.keyboard("{ArrowUp}");
      expect(selectedOption()).toHaveTextContent("New Project");
    });

    it("wraps from the last row to the first and from the first to the last", async () => {
      await openPalette();

      await userEvent.keyboard("{ArrowUp}");
      expect(selectedOption()).toHaveTextContent("Toggle Inspector");

      await userEvent.keyboard("{ArrowDown}");
      expect(selectedOption()).toHaveTextContent("New Project");
    });

    it("runs the selected command with Enter and closes", async () => {
      await openPalette();

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(mocked.runCommand).toHaveBeenCalledWith("app:open");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("runs a page command of the window with Enter", async () => {
      await openPalette();
      await userEvent.type(field(), "toggle sidebar");

      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().sidebarHidden).toBe(true);
    });

    it("runs a command that is clicked", async () => {
      await openPalette();

      await userEvent.click(screen.getByRole("option", { name: /Week View/ }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:week");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("does nothing on Enter when there is no result", async () => {
      await openPalette();
      await userEvent.type(field(), "zzzz");

      await userEvent.keyboard("{Enter}");

      expect(mocked.runCommand).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });
  });

  describe("the commands prefix button", () => {
    it("switches to the commands prefix and back", async () => {
      await openPalette();
      const button = screen.getByRole("button", { name: /Commands/ });
      expect(button).toHaveAttribute("aria-pressed", "false");

      await userEvent.click(button);
      expect(field()).toHaveValue(">");
      expect(field()).toHaveAttribute("placeholder", "Run a menu command");
      expect(button).toHaveAttribute("aria-pressed", "true");

      await userEvent.click(button);
      expect(field()).toHaveValue("");
      expect(button).toHaveAttribute("aria-pressed", "false");
    });

    it("lists the commands for the prefix alone and applies a query after it", async () => {
      await openPalette();

      await userEvent.type(field(), ">");
      expect(screen.getAllByRole("option").length).toBeGreaterThan(1);

      await userEvent.type(field(), "open");
      expect(screen.getAllByRole("option")).toHaveLength(1);
      expect(screen.getByRole("option")).toHaveTextContent("Open Folder");
    });
  });

  describe("failures", () => {
    it("shows an error toast when the chosen command fails", async () => {
      mocked.runCommand.mockRejectedValue({ kind: "not_found", message: "No such command", code: null });
      await openPalette();

      await userEvent.keyboard("{Enter}");

      await waitFor(() =>
        expect(useStore.getState().toasts).toMatchObject([{ kind: "error", message: "Could not run “New Project”: No such command" }]),
      );
    });
  });

  describe("recent searches", () => {
    async function searchAndRun(text: string) {
      await openPalette();
      await userEvent.type(field(), text);
      await userEvent.keyboard("{Enter}");
    }

    it("lists a search that led to a command the next time the palette opens", async () => {
      await searchAndRun("week");

      togglePalette();

      const recent = await screen.findByRole("region", { name: "Recent Searches" });
      expect(recent).toHaveTextContent("week");
      expect(screen.getByText("↩ search")).toBeInTheDocument();
    });

    it("puts the recent search into the field on Enter and keeps the palette open", async () => {
      await searchAndRun("week");
      togglePalette();
      await screen.findByRole("region", { name: "Recent Searches" });

      await userEvent.keyboard("{Enter}");

      expect(field()).toHaveValue("week");
      expect(screen.getByRole("option")).toHaveTextContent("Week View");
      expect(screen.queryByRole("region", { name: "Recent Searches" })).not.toBeInTheDocument();
    });

    it("does not remember a command chosen without a query", async () => {
      await openPalette();
      await userEvent.keyboard("{Enter}");

      togglePalette();
      await screen.findByRole("option", { name: /New Project/ });

      expect(screen.queryByRole("region", { name: "Recent Searches" })).not.toBeInTheDocument();
    });
  });
});
