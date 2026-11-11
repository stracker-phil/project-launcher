import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PalettePanel } from "./PalettePanel";
import type { PaletteMode } from "./prefixes";
import { clearRecentSearches } from "./recent";
import type { PaletteCommand } from "./sources";

let calls: string[] = [];

const command = (mode: PaletteMode, label: string, section: string, patch: Partial<PaletteCommand> = {}): PaletteCommand => ({
  id: `${section}:${label}`,
  mode,
  label,
  section,
  run: () => void calls.push(`run ${label}`),
  ...patch,
});

const COMMANDS = [
  command("projects", "Alpha", "Projects", { hint: "Current" }),
  command("projects", "Beta Shop", "Projects", { hint: "Client" }),
  command("library", "Starred", "Library", { hint: "1", icon: <svg aria-label="Star icon" /> }),
  command("actions", "Open in Editor", "Alpha", { shortcut: "⌘O" }),
  command("commands", "New Project", "File", { shortcut: "⌘N" }),
  command("commands", "Week View", "View"),
];

function renderPanel(commands = COMMANDS) {
  const onClose = vi.fn(() => void calls.push("close"));
  const view = render(<PalettePanel commands={commands} onClose={onClose} />);
  return { onClose, ...view };
}

const field = () => screen.getByRole("combobox");
const selected = () => screen.getByRole("option", { selected: true });

beforeEach(() => {
  calls = [];
  clearRecentSearches();
});

describe("PalettePanel", () => {
  describe("results", () => {
    it("lists projects, library, the project's actions and the menus under their headings", () => {
      renderPanel();

      expect(screen.getByRole("region", { name: "Projects" })).toHaveTextContent("Beta Shop");
      expect(screen.getByRole("region", { name: "Library" })).toHaveTextContent("Starred");
      expect(screen.getByRole("region", { name: "Alpha" })).toHaveTextContent("Open in Editor");
      expect(screen.getByRole("region", { name: "File" })).toHaveTextContent("New Project");
      expect(screen.getByRole("region", { name: "View" })).toHaveTextContent("Week View");
    });

    it("shows a row's hint, shortcut and icon", () => {
      renderPanel();

      expect(screen.getByRole("option", { name: /Alpha\s*Current/ })).toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Starred/ })).toContainElement(screen.getByLabelText("Star icon"));
      expect(screen.getByText("⌘N")).toBeInTheDocument();
    });

    it("narrows the list to the best matches while typing", async () => {
      renderPanel();

      await userEvent.type(field(), "week");

      expect(screen.getAllByRole("option")).toHaveLength(1);
      expect(screen.getByRole("region", { name: "Commands" })).toHaveTextContent("Week View");
    });

    it("says there are no results when nothing matches", async () => {
      renderPanel();

      await userEvent.type(field(), "zzzz");

      expect(screen.getByText("No results")).toBeInTheDocument();
      expect(screen.queryByRole("option")).not.toBeInTheDocument();
    });
  });

  describe("prefix buttons", () => {
    it("offers a button for each kind that has commands", () => {
      renderPanel([command("commands", "Week View", "View"), command("projects", "Alpha", "Projects")]);

      expect(screen.getByRole("button", { name: /Commands/ })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Projects/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Library/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Actions/ })).not.toBeInTheDocument();
    });

    it("narrows to one kind and back", async () => {
      renderPanel();
      const button = screen.getByRole("button", { name: /Projects/ });

      await userEvent.click(button);
      expect(field()).toHaveValue(":");
      expect(field()).toHaveAttribute("placeholder", "Find a project");
      expect(button).toHaveAttribute("aria-pressed", "true");
      expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["AlphaCurrent", "Beta ShopClient"]);

      await userEvent.click(button);
      expect(field()).toHaveValue("");
      expect(button).toHaveAttribute("aria-pressed", "false");
    });

    it("applies a query typed after a typed prefix", async () => {
      renderPanel();

      await userEvent.type(field(), ">open");

      expect(screen.queryByRole("option")).not.toBeInTheDocument();
      await userEvent.clear(field());
      await userEvent.type(field(), "/open");
      expect(screen.getByRole("option")).toHaveTextContent("Open in Editor");
    });
  });

  describe("keyboard", () => {
    it("starts on the first row and moves down and up", async () => {
      renderPanel();
      expect(selected()).toHaveTextContent("Alpha");

      await userEvent.keyboard("{ArrowDown}");
      expect(selected()).toHaveTextContent("Beta Shop");

      await userEvent.keyboard("{ArrowUp}");
      expect(selected()).toHaveTextContent("Alpha");
    });

    it("wraps from the first row to the last and back", async () => {
      renderPanel();

      await userEvent.keyboard("{ArrowUp}");
      expect(selected()).toHaveTextContent("Week View");

      await userEvent.keyboard("{ArrowDown}");
      expect(selected()).toHaveTextContent("Alpha");
    });

    it("closes, then runs the selected command with Enter", async () => {
      const { onClose } = renderPanel();

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(calls).toEqual(["close", "run Beta Shop"]);
      expect(onClose).toHaveBeenCalled();
    });

    it("closes, then runs a command that is clicked", async () => {
      renderPanel();

      await userEvent.click(screen.getByRole("option", { name: /Week View/ }));

      expect(calls).toEqual(["close", "run Week View"]);
    });

    it("does nothing on Enter when there is no result", async () => {
      renderPanel();
      await userEvent.type(field(), "zzzz");

      await userEvent.keyboard("{Enter}");

      expect(calls).toEqual([]);
    });

    it.each([["Escape", "{Escape}"], ["⌘K", "{Meta>}k{/Meta}"]])("closes without running anything on %s", async (_name, keys) => {
      renderPanel();

      await userEvent.keyboard(keys);

      expect(calls).toEqual(["close"]);
    });
  });

  describe("mouse", () => {
    it("keeps the focus in the search field when a press lands elsewhere in the panel", () => {
      renderPanel();

      const notPrevented = fireEvent.mouseDown(screen.getByRole("heading", { name: "Projects" }));

      expect(notPrevented).toBe(false);
    });

    it("leaves a press on the search field itself alone", () => {
      renderPanel();

      expect(fireEvent.mouseDown(field())).toBe(true);
    });
  });

  describe("recent searches", () => {
    async function searchAndRun(text: string) {
      const { unmount } = renderPanel();
      await userEvent.type(field(), text);
      await userEvent.keyboard("{Enter}");
      unmount();
    }

    it("lists a search that led to a command the next time the panel opens", async () => {
      await searchAndRun("week");

      renderPanel();

      expect(screen.getByRole("region", { name: "Recent Searches" })).toHaveTextContent("week");
      expect(screen.getByText("↩ search")).toBeInTheDocument();
    });

    it("puts the recent search into the field on Enter and keeps the panel open", async () => {
      await searchAndRun("week");
      const { onClose } = renderPanel();
      calls = [];

      await userEvent.keyboard("{Enter}");

      expect(field()).toHaveValue("week");
      expect(screen.getByRole("option")).toHaveTextContent("Week View");
      expect(calls).toEqual([]);
      expect(onClose).not.toHaveBeenCalled();
    });

    it("does not remember a command chosen without a query", async () => {
      const { unmount } = renderPanel();
      await userEvent.keyboard("{Enter}");
      unmount();

      renderPanel();

      expect(screen.queryByRole("region", { name: "Recent Searches" })).not.toBeInTheDocument();
    });
  });
});
