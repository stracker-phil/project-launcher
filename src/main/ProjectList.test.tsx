import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Library } from "../api";
import { MenuHost } from "../components/menu";
import { Toasts } from "../components/Toasts";
import { createQueryClient } from "../queries/queryClient";
import { library, project, row, section } from "../test/library";
import { resetLibrary, useLibrary } from "../state/library";
import { resetStore } from "../state/store";
import { ProjectList, SearchField } from "./ProjectList";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const beta = project({ id: "beta", name: "Beta", path: "~/Projects/beta" });
const gamma = project({ id: "gamma", name: "Gamma", path: "~/Projects/gamma" });
const threeProjects = library([section(null, alpha, beta, gamma)]);

/** `null` renders the list without a library, as while it loads. */
function renderList(lib: Library | null = threeProjects, settled = true) {
  const client = createQueryClient();
  const ui = (s: boolean) => (
    <QueryClientProvider client={client}>
      <SearchField />
      <ProjectList library={lib ?? undefined} settled={s} />
      <MenuHost />
      <Toasts />
    </QueryClientProvider>
  );
  const view = render(ui(settled));
  return { ...view, settle: () => view.rerender(ui(true)) };
}

const option = (name: string) => screen.getByRole("option", { name: new RegExp(`^${name}`) });
const selectedId = () => useLibrary.getState().selectedId;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetLibrary();
});

describe("ProjectList", () => {
  describe("rendering", () => {
    it("shows the sections under their headings with a row per project", () => {
      renderList(library([section("Recent", beta), section("All", alpha, gamma)]));

      const recent = screen.getByRole("group", { name: "Recent" });
      const all = screen.getByRole("group", { name: "All" });
      expect(within(recent).getAllByRole("option")).toHaveLength(1);
      expect(within(recent).getByText("Beta")).toBeInTheDocument();
      expect(within(all).getAllByRole("option").map((o) => o.textContent)).toEqual(["Alpha~/Projects/alpha", "Gamma~/Projects/gamma"]);
    });

    it("shows a row's subtitle under its name", () => {
      renderList(library([{ title: null, rows: [row(alpha, "Demo note line")] }]));

      expect(within(option("Alpha")).getByText("Demo note line")).toBeInTheDocument();
    });

    it("shows a star mark on starred projects only", () => {
      renderList(library([section(null, project({ id: "alpha", name: "Alpha", meta: { starred: true } }), beta)]));

      expect(within(option("Alpha")).getByLabelText("Starred")).toBeInTheDocument();
      expect(within(option("Beta")).queryByLabelText("Starred")).not.toBeInTheDocument();
    });

    it("shows an alert mark on a project whose file is missing or broken", () => {
      renderList(library([section(null, project({ id: "alpha", name: "Alpha", available: false }), beta)]));

      expect(within(option("Alpha")).getByLabelText("File missing or broken")).toBeInTheDocument();
      expect(within(option("Beta")).queryByLabelText("File missing or broken")).not.toBeInTheDocument();
    });

    it("shows the library's empty text when it has one", () => {
      renderList(library([], { emptyText: "No projects match “zzz”" }));

      expect(screen.getByText("No projects match “zzz”")).toBeInTheDocument();
      expect(screen.queryByRole("option")).not.toBeInTheDocument();
    });

    it("renders an empty list while the library is still loading", () => {
      renderList(null, false);

      expect(screen.getByRole("listbox", { name: "Projects" })).toBeEmptyDOMElement();
    });
  });

  describe("selection", () => {
    it("selects a row and makes it the current project when it is clicked", async () => {
      renderList();

      await userEvent.click(option("Beta"));

      expect(option("Beta")).toHaveAttribute("aria-selected", "true");
      expect(selectedId()).toBe("beta");
      expect(useLibrary.getState().current?.id).toBe("beta");
    });

    it("selects the first row once the library has settled and nothing is selected", () => {
      renderList();

      expect(option("Alpha")).toHaveAttribute("aria-selected", "true");
      expect(selectedId()).toBe("alpha");
      expect(useLibrary.getState().current?.name).toBe("Alpha");
    });

    it("selects nothing while the library has not settled, then the first row once it has", () => {
      const { settle } = renderList(threeProjects, false);
      expect(selectedId()).toBeNull();

      settle();

      expect(selectedId()).toBe("alpha");
    });

    it("keeps the selected project when it is shown", () => {
      resetLibrary({ selectedId: "beta" });

      renderList();

      expect(selectedId()).toBe("beta");
    });

    it("selects the project last selected under the filter when the selected one is not shown", () => {
      resetLibrary({ selectedId: "elsewhere", lastSelected: { all: "gamma" } });

      renderList();

      expect(selectedId()).toBe("gamma");
    });

    it("selects the first row when the remembered project is not shown either", () => {
      resetLibrary({ selectedId: "elsewhere", lastSelected: { all: "removed" } });

      renderList();

      expect(selectedId()).toBe("alpha");
    });

    it("selects nothing and clears the current project when the list is empty", () => {
      resetLibrary({ selectedId: "alpha", current: alpha });

      renderList(library([], { emptyText: "No projects here" }));

      expect(selectedId()).toBeNull();
      expect(useLibrary.getState().current).toBeNull();
    });
  });

  describe("keyboard", () => {
    const listbox = () => screen.getByRole("listbox", { name: "Projects" });

    it("moves the selection down and up with the arrow keys", async () => {
      renderList();
      listbox().focus();

      await userEvent.keyboard("{ArrowDown}");
      expect(selectedId()).toBe("beta");
      await userEvent.keyboard("{ArrowDown}");
      expect(selectedId()).toBe("gamma");
      await userEvent.keyboard("{ArrowUp}");
      expect(selectedId()).toBe("beta");
    });

    it("stops at the first row on ArrowUp", async () => {
      renderList();
      listbox().focus();

      await userEvent.keyboard("{ArrowUp}{ArrowUp}");

      expect(selectedId()).toBe("alpha");
    });

    it("stops at the last row on ArrowDown", async () => {
      renderList();
      listbox().focus();

      await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}{ArrowDown}");

      expect(selectedId()).toBe("gamma");
    });

    it.each([
      { key: "ArrowDown", id: "beta" },
      { key: "ArrowUp", id: "alpha" },
    ])("moves the selection on $key with nothing focused and gives the list the focus", ({ key, id }) => {
      resetLibrary({ selectedId: key === "ArrowUp" ? "beta" : "alpha" });
      renderList();
      expect(document.body).toHaveFocus();

      fireEvent.keyDown(window, { key });

      expect(selectedId()).toBe(id);
      expect(listbox()).toHaveFocus();
    });

    it("leaves the arrow keys to a text field that has focus", async () => {
      renderList();
      screen.getByRole("searchbox", { name: "Search Projects" }).focus();

      await userEvent.keyboard("{ArrowDown}");

      expect(selectedId()).toBe("alpha");
    });

    it("leaves the arrow keys to an open menu", () => {
      renderList();
      fireEvent.contextMenu(option("Alpha"));

      fireEvent.keyDown(document.body, { key: "ArrowDown" });

      expect(selectedId()).toBe("alpha");
    });

    it("leaves the arrow keys to an open confirmation", () => {
      renderList();
      act(() => resetStore({ confirmation: { title: "Remove?", text: "Demo", action: "Remove", answer: vi.fn() } }));

      fireEvent.keyDown(document.body, { key: "ArrowDown" });

      expect(selectedId()).toBe("alpha");
    });

    it.each([
      { name: "a dialog", markup: <div role="dialog" aria-label="Demo dialog"><button type="button">Inside</button></div> },
      { name: "a popover", markup: <div className="popover"><button type="button">Inside</button></div> },
    ])("leaves the arrow keys to $name", async ({ markup }) => {
      renderList();
      render(markup);
      screen.getByRole("button", { name: "Inside" }).focus();

      await userEvent.keyboard("{ArrowDown}");

      expect(selectedId()).toBe("alpha");
    });
  });

  describe("dev server mark", () => {
    it("marks the rows whose project's dev server runs", async () => {
      mocked.devRunning.mockResolvedValue(["beta"]);

      renderList();

      expect(await within(option("Beta")).findByRole("img", { name: "Dev server running" })).toBeInTheDocument();
      expect(within(option("Alpha")).queryByRole("img", { name: "Dev server running" })).not.toBeInTheDocument();
    });
  });

  describe("context menu", () => {
    const item = (name: string) => screen.getByRole("menuitem", { name: new RegExp(`^${name}`) });

    it("opens the project's menu on right-click and selects the row", () => {
      renderList();

      fireEvent.contextMenu(option("Beta"));

      const menu = screen.getByRole("menu", { name: "Beta" });
      expect(selectedId()).toBe("beta");
      for (const name of ["Star Project", "Archive Project", "Move to Tag", "Rename Project", "Change Icon & Color…", "Edit Note", "Lock Project Details", "Remove from List"]) {
        expect(within(menu).getByRole("menuitem", { name: new RegExp(`^${name}`) })).not.toHaveAttribute("aria-disabled");
      }
    });

    it("offers Unstar, Unarchive and Unlock for a project that is starred, archived and locked", () => {
      renderList(library([section(null, project({ name: "Alpha", locked: true, meta: { starred: true, archived: true } }))]));

      fireEvent.contextMenu(option("Alpha"));

      expect(item("Unstar Project")).toBeInTheDocument();
      expect(item("Unarchive Project")).toBeInTheDocument();
      expect(item("Unlock Project Details")).toBeInTheDocument();
    });

    it.each(["Move to Tag", "Rename Project", "Change Icon & Color…", "Edit Note"])("disables %s for a locked project", (name) => {
      renderList(library([section(null, project({ name: "Alpha", locked: true }))]));

      fireEvent.contextMenu(option("Alpha"));

      expect(item(name)).toHaveAttribute("aria-disabled", "true");
      expect(item("Unlock Project Details")).not.toHaveAttribute("aria-disabled");
    });

    it("sends the flipped starred flag when Star Project is chosen", async () => {
      renderList();
      fireEvent.contextMenu(option("Alpha"));

      await userEvent.click(item("Star Project"));

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "starred", value: true });
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("puts the name into edit mode when Rename Project is chosen", async () => {
      renderList();
      fireEvent.contextMenu(option("Beta"));

      await userEvent.click(item("Rename Project"));

      expect(useLibrary.getState().editing).toEqual({ id: "beta", field: "name" });
    });

    it("removes the project and offers Undo when Remove from List is chosen", async () => {
      renderList();
      fireEvent.contextMenu(option("Alpha"));

      await userEvent.click(item("Remove from List"));

      expect(mocked.removeProject).toHaveBeenCalledWith("alpha");
      expect(await screen.findByRole("button", { name: "Undo" })).toBeInTheDocument();
    });

    it("lists the library's tags under Move to Tag and sets the chosen one", async () => {
      const withTags = library([section(null, alpha)], {
        sidebar: { all: 1, starred: 0, untagged: 1, archived: 0, tags: [{ name: "client", count: 0 }] },
      });
      renderList(withTags);
      fireEvent.contextMenu(option("Alpha"));

      await userEvent.click(item("Move to Tag"));
      await userEvent.click(screen.getByRole("menuitemradio", { name: "client" }));

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "tag", value: "client" });
    });
  });
});

describe("SearchField", () => {
  const field = () => screen.getByRole("searchbox", { name: "Search Projects" });

  it("shows what is typed and keeps it as the library's search", async () => {
    renderList();

    await userEvent.type(field(), "alp");

    expect(field()).toHaveValue("alp");
    expect(useLibrary.getState().search).toBe("alp");
  });

  it("starts from the stored search", () => {
    resetLibrary({ search: "beta" });

    renderList();

    expect(field()).toHaveValue("beta");
  });

  it("clears the search on Escape", async () => {
    resetLibrary({ search: "beta" });
    renderList();
    field().focus();

    await userEvent.keyboard("{Escape}");

    expect(field()).toHaveValue("");
    expect(useLibrary.getState().search).toBe("");
  });

  it("takes the focus when the library is asked to focus the search", () => {
    renderList();
    expect(field()).not.toHaveFocus();

    act(() => useLibrary.getState().focusSearch());

    expect(field()).toHaveFocus();
  });
});
