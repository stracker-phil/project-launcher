import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Library } from "../api";
import { ConfirmHost } from "../components/ConfirmSheet";
import { MenuHost } from "../components/menu";
import { resetLibrary, useLibrary } from "../state/library";
import { resetStore, useStore } from "../state/store";
import { libraryWithTags } from "../test/library";
import { Sidebar } from "./Sidebar";

const mocked = vi.mocked(api, true);

function renderSidebar(library: Library = libraryWithTags()) {
  render(
    <>
      <Sidebar library={library} />
      <MenuHost />
      <ConfirmHost />
    </>,
  );
}

const filter = () => useLibrary.getState().filter;
const toastMessages = () => useStore.getState().toasts.map((t) => t.message);

async function chooseFromTagMenu(row: string, item: string) {
  fireEvent.contextMenu(screen.getByRole("button", { name: row }));
  await userEvent.click(await screen.findByRole("menuitem", { name: item }));
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.addTag.mockResolvedValue(undefined);
  mocked.renameTag.mockResolvedValue(undefined);
  mocked.deleteTag.mockResolvedValue(undefined);
  resetStore();
  resetLibrary();
});

describe("Sidebar", () => {
  describe("header and groups", () => {
    it("shows the library's title and count label", () => {
      renderSidebar();

      expect(screen.getByRole("heading", { level: 1, name: "All Projects" })).toBeInTheDocument();
      expect(screen.getByText("5 projects")).toBeInTheDocument();
    });

    it("renders without a library", () => {
      render(<Sidebar library={undefined} />);

      expect(screen.getByRole("button", { name: "All Projects" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Archived" })).toBeInTheDocument();
    });

    it("lists the library filters and the tags with their counts, and no number at zero", () => {
      renderSidebar();

      const library = screen.getByRole("group", { name: "Library" });
      expect(within(library).getByRole("button", { name: "All Projects 5" })).toBeInTheDocument();
      expect(within(library).getByRole("button", { name: "Starred" })).toBeInTheDocument();
      const tags = screen.getByRole("group", { name: "Tags" });
      expect(within(tags).getByRole("button", { name: "Client 2" })).toBeInTheDocument();
      expect(within(tags).getByRole("button", { name: "Internal 1" })).toBeInTheDocument();
      expect(within(tags).getByRole("button", { name: "Ideas" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Archived 1" })).toBeInTheDocument();
    });

    it("lists the tags in the library's order", () => {
      renderSidebar();

      const rows = within(screen.getByRole("group", { name: "Tags" })).getAllByRole("button", { name: /^(Client|Internal|Ideas)/ });
      expect(rows.map((row) => row.textContent)).toEqual(["Client2", "Internal1", "Ideas"]);
    });

    it("shows Untagged with its count when the library reports untagged projects", () => {
      renderSidebar();

      expect(screen.getByRole("button", { name: "Untagged 2" })).toBeInTheDocument();
    });

    it("leaves Untagged out when the library reports none", () => {
      renderSidebar(libraryWithTags({ untagged: null }));

      expect(screen.queryByRole("button", { name: /^Untagged/ })).not.toBeInTheDocument();
    });
  });

  describe("settings", () => {
    it("runs the settings command from the cog button", async () => {
      renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: "Settings" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
    });
  });

  describe("filters", () => {
    it("starts on All Projects as the current row", () => {
      renderSidebar();

      expect(screen.getByRole("button", { name: "All Projects 5" })).toHaveAttribute("aria-current", "true");
    });

    it.each([
      ["Starred", { kind: "starred" }],
      ["Client 2", { kind: "tag", tag: "Client" }],
      ["Untagged 2", { kind: "untagged" }],
      ["Archived 1", { kind: "archived" }],
    ])("sets the %s row as the filter when clicked", async (row, expected) => {
      renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: row }));

      expect(filter()).toEqual(expected);
      expect(screen.getByRole("button", { name: row })).toHaveAttribute("aria-current", "true");
      expect(screen.getByRole("button", { name: "All Projects 5" })).not.toHaveAttribute("aria-current");
    });

    it("goes back to All Projects when its row is clicked", async () => {
      resetLibrary({ filter: { kind: "archived" } });
      renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: "All Projects 5" }));

      expect(filter()).toEqual({ kind: "all" });
    });
  });

  describe("new tag", () => {
    it("shows a Tag name field when New Tag is clicked", async () => {
      renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      expect(screen.getByRole("textbox", { name: "Tag name" })).toHaveValue("");
    });

    it("adds the trimmed name on Enter and closes the field", async () => {
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "  Docs  {Enter}");

      expect(mocked.addTag).toHaveBeenCalledWith("Docs");
      expect(screen.queryByRole("textbox", { name: "Tag name" })).not.toBeInTheDocument();
    });

    it("adds the name when the field loses focus", async () => {
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Docs");
      const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(true);
      await userEvent.tab();
      hasFocus.mockRestore();

      expect(mocked.addTag).toHaveBeenCalledWith("Docs");
    });

    it("shows Rust's rejection as a toast when the tag exists", async () => {
      mocked.addTag.mockRejectedValue({ kind: "invalid_input", message: "“Client” already exists" });
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Client{Enter}");

      expect(mocked.addTag).toHaveBeenCalledWith("Client");
      await waitFor(() => expect(toastMessages()).toContain("Couldn’t create tag: “Client” already exists"));
    });

    it("adds nothing on Escape", async () => {
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Docs{Escape}");

      expect(mocked.addTag).not.toHaveBeenCalled();
      expect(screen.queryByRole("textbox", { name: "Tag name" })).not.toBeInTheDocument();
    });

    it("adds nothing for a blank name", async () => {
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "   {Enter}");

      expect(mocked.addTag).not.toHaveBeenCalled();
    });

    it.each(["Docs{Enter}", "Docs{Escape}"])("hands the keys to the project list after %j", async (keys) => {
      render(<div role="listbox" aria-label="Projects" tabIndex={0} />);
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), keys);
      await act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));

      expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
    });

    it("keeps the field open and adds nothing when it blurs while the document has no focus", async () => {
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));
      const field = screen.getByRole("textbox", { name: "Tag name" });
      await userEvent.type(field, "Docs");
      const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(false);

      fireEvent.blur(field);

      expect(screen.getByRole("textbox", { name: "Tag name" })).toHaveValue("Docs");
      expect(mocked.addTag).not.toHaveBeenCalled();
      hasFocus.mockRestore();
    });

    it("toasts the reason when adding fails", async () => {
      mocked.addTag.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSidebar();
      await userEvent.click(screen.getByRole("button", { name: "New Tag" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Docs{Enter}");

      await waitFor(() => expect(toastMessages()).toContain("Couldn’t create tag: Disk full"));
    });
  });

  describe("rename tag", () => {
    it("shows the tag's name selected in a field on double-click", async () => {
      renderSidebar();

      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));

      const field = screen.getByRole<HTMLInputElement>("textbox", { name: "Tag name" });
      expect(field).toHaveValue("Client");
      await waitFor(() => expect(field.selectionEnd).toBe("Client".length));
    });

    it("renames on Enter and moves the filter to the new tag when it was selected", async () => {
      renderSidebar();
      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));
      expect(filter()).toEqual({ kind: "tag", tag: "Client" });

      const field = screen.getByRole("textbox", { name: "Tag name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Customers{Enter}");

      expect(mocked.renameTag).toHaveBeenCalledWith("Client", "Customers");
      await waitFor(() => expect(filter()).toEqual({ kind: "tag", tag: "Customers" }));
    });

    it("keeps another filter when a tag is renamed from its menu", async () => {
      renderSidebar();
      await chooseFromTagMenu("Internal 1", "Rename Tag");

      const field = screen.getByRole("textbox", { name: "Tag name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Staff{Enter}");

      expect(mocked.renameTag).toHaveBeenCalledWith("Internal", "Staff");
      expect(filter()).toEqual({ kind: "all" });
    });

    it("keeps the filter when the rename fails", async () => {
      mocked.renameTag.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSidebar();
      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));

      const field = screen.getByRole("textbox", { name: "Tag name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Customers{Enter}");

      await waitFor(() => expect(toastMessages()).toContain("Couldn’t rename tag: Disk full"));
      expect(filter()).toEqual({ kind: "tag", tag: "Client" });
    });

    it("renames nothing on Escape", async () => {
      renderSidebar();
      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "s{Escape}");

      expect(mocked.renameTag).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Client 2" })).toBeInTheDocument();
    });

    it("renames nothing when the name is unchanged", async () => {
      renderSidebar();
      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "{Enter}");

      expect(mocked.renameTag).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Client 2" })).toBeInTheDocument();
    });

    it("renames onto an existing tag and lets Rust merge them, the filter following", async () => {
      renderSidebar();
      await userEvent.dblClick(screen.getByRole("button", { name: "Client 2" }));

      const field = screen.getByRole("textbox", { name: "Tag name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Internal{Enter}");

      expect(mocked.renameTag).toHaveBeenCalledWith("Client", "Internal");
      await waitFor(() => expect(filter()).toEqual({ kind: "tag", tag: "Internal" }));
    });
  });

  describe("tag menu", () => {
    it("offers Rename Tag and Delete Tag… on right-click", async () => {
      renderSidebar();

      fireEvent.contextMenu(screen.getByRole("button", { name: "Client 2" }));

      const menu = await screen.findByRole("menu", { name: "Tag" });
      expect(within(menu).getByRole("menuitem", { name: "Rename Tag" })).toBeInTheDocument();
      expect(within(menu).getByRole("menuitem", { name: "Delete Tag…" })).toBeInTheDocument();
    });

    it("starts the rename from the menu", async () => {
      renderSidebar();

      await chooseFromTagMenu("Client 2", "Rename Tag");

      expect(screen.getByRole("textbox", { name: "Tag name" })).toHaveValue("Client");
    });
  });

  describe("delete tag", () => {
    it("asks before deleting and names the projects that become untagged", async () => {
      renderSidebar();

      await chooseFromTagMenu("Client 2", "Delete Tag…");

      const dialog = await screen.findByRole("alertdialog", { name: "Delete tag “Client”?" });
      expect(dialog).toHaveAccessibleDescription("2 projects become untagged.");
      expect(mocked.deleteTag).not.toHaveBeenCalled();
    });

    it.each([
      ["Internal 1", "Delete tag “Internal”?", "1 project becomes untagged."],
      ["Ideas", "Delete tag “Ideas”?", "No projects use this tag."],
    ])("words the question for %s", async (row, title, text) => {
      renderSidebar();

      await chooseFromTagMenu(row, "Delete Tag…");

      expect(await screen.findByRole("alertdialog", { name: title })).toHaveAccessibleDescription(text);
    });

    it("deletes the tag once confirmed", async () => {
      renderSidebar();
      await chooseFromTagMenu("Client 2", "Delete Tag…");

      await userEvent.click(await screen.findByRole("button", { name: "Delete" }));

      await waitFor(() => expect(mocked.deleteTag).toHaveBeenCalledWith("Client"));
    });

    it("deletes nothing when cancelled", async () => {
      renderSidebar();
      await chooseFromTagMenu("Client 2", "Delete Tag…");

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
      expect(mocked.deleteTag).not.toHaveBeenCalled();
    });

    it("goes back to All Projects when the selected tag is deleted", async () => {
      resetLibrary({ filter: { kind: "tag", tag: "Client" } });
      renderSidebar();
      await chooseFromTagMenu("Client 2", "Delete Tag…");

      await userEvent.click(await screen.findByRole("button", { name: "Delete" }));

      await waitFor(() => expect(filter()).toEqual({ kind: "all" }));
    });

    it("keeps another filter when a different tag is deleted", async () => {
      resetLibrary({ filter: { kind: "starred" } });
      renderSidebar();
      await chooseFromTagMenu("Client 2", "Delete Tag…");

      await userEvent.click(await screen.findByRole("button", { name: "Delete" }));

      await waitFor(() => expect(mocked.deleteTag).toHaveBeenCalled());
      expect(filter()).toEqual({ kind: "starred" });
    });

    it("keeps the filter and toasts when the delete fails", async () => {
      mocked.deleteTag.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      resetLibrary({ filter: { kind: "tag", tag: "Client" } });
      renderSidebar();
      await chooseFromTagMenu("Client 2", "Delete Tag…");

      await userEvent.click(await screen.findByRole("button", { name: "Delete" }));

      await waitFor(() => expect(toastMessages()).toContain("Couldn’t delete tag: Disk full"));
      expect(filter()).toEqual({ kind: "tag", tag: "Client" });
    });
  });
});
