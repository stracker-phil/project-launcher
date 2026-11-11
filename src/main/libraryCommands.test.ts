import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { clearCommands, registerCommands, runPageCommand } from "../commands/registry";
import { resetLibrary, useLibrary } from "../state/library";
import { resetStore, useStore } from "../state/store";
import { project } from "../test/library";
import { LIBRARY_COMMANDS } from "./libraryCommands";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetLibrary();
  clearCommands();
  registerCommands(LIBRARY_COMMANDS);
});

describe("filter commands", () => {
  it.each([
    { id: "library.all", filter: { kind: "all" } },
    { id: "library.starred", filter: { kind: "starred" } },
    { id: "library.archived", filter: { kind: "archived" } },
  ])("$id sets the filter", ({ id, filter }) => {
    resetLibrary({ filter: { kind: "untagged" }, search: "alp" });

    expect(runPageCommand(id)).toBe(true);

    expect(useLibrary.getState().filter).toEqual(filter);
    expect(useLibrary.getState().search).toBe("");
  });

  it("library.starred shows the sidebar when it is hidden", () => {
    resetStore({ sidebarHidden: true });

    runPageCommand("library.starred");

    expect(useStore.getState().sidebarHidden).toBe(false);
  });

  it("library.starred leaves a shown sidebar shown", () => {
    runPageCommand("library.starred");

    expect(useStore.getState().sidebarHidden).toBe(false);
  });
});

describe("tag.new", () => {
  it("starts a new tag in the sidebar and shows the sidebar when it is hidden", () => {
    resetStore({ sidebarHidden: true });

    runPageCommand("tag.new");

    expect(useLibrary.getState().tagEdit).toEqual({ mode: "new" });
    expect(useStore.getState().sidebarHidden).toBe(false);
  });
});

describe("view.search", () => {
  it("asks the search field for the focus and shows the sidebar when it is hidden", () => {
    resetStore({ sidebarHidden: true });

    runPageCommand("view.search");

    expect(useLibrary.getState().searchFocus).toBe(1);
    expect(useStore.getState().sidebarHidden).toBe(false);
  });
});

describe("project.new", () => {
  it("opens the folder panel", () => {
    runPageCommand("project.new");

    expect(mocked.addProjectFolders).toHaveBeenCalled();
  });
});

describe("commands on the current project", () => {
  describe.each([
    { id: "project.rename", field: "name" },
    { id: "project.icon", field: "icon" },
    { id: "project.notes", field: "notes" },
  ])("$id", ({ id, field }) => {
    it(`selects the current project and puts its ${field} into edit mode`, () => {
      resetLibrary({ current: alpha });

      expect(runPageCommand(id)).toBe(true);

      expect(useLibrary.getState().editing).toEqual({ id: "alpha", field });
      expect(useLibrary.getState().selectedId).toBe("alpha");
    });

    it.each([
      { why: "there is no current project", current: null },
      { why: "the current project is locked", current: project({ locked: true }) },
      { why: "the current project is unavailable", current: project({ available: false }) },
    ])("is disabled when $why", ({ current }) => {
      resetLibrary({ current });

      expect(runPageCommand(id)).toBe(false);

      expect(useLibrary.getState().editing).toBeNull();
    });
  });

  describe("project.star", () => {
    it.each([
      { starred: false, value: true },
      { starred: true, value: false },
    ])("sends starred $value for a project with starred $starred", ({ starred, value }) => {
      resetLibrary({ current: project({ meta: { starred } }) });

      expect(runPageCommand("project.star")).toBe(true);

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "starred", value });
    });

    it.each([
      { why: "there is no current project", current: null },
      { why: "the current project is unavailable", current: project({ available: false }) },
    ])("is disabled when $why", ({ current }) => {
      resetLibrary({ current });

      expect(runPageCommand("project.star")).toBe(false);

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
    });

    it("also works on a locked project", () => {
      resetLibrary({ current: project({ locked: true }) });

      expect(runPageCommand("project.star")).toBe(true);

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "starred", value: true });
    });
  });

  describe("project.archive", () => {
    it("sends archived true for an active project", () => {
      resetLibrary({ current: alpha });

      expect(runPageCommand("project.archive")).toBe(true);

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "archived", value: true });
    });

    it("is disabled without a current project", () => {
      expect(runPageCommand("project.archive")).toBe(false);
    });
  });

  describe("project.lock", () => {
    it.each([
      { locked: false, value: true },
      { locked: true, value: false },
    ])("sends locked $value for a project with locked $locked", ({ locked, value }) => {
      resetLibrary({ current: project({ locked }) });

      expect(runPageCommand("project.lock")).toBe(true);

      expect(mocked.setProjectLocked).toHaveBeenCalledWith("alpha", value);
    });

    it("is disabled when the current project is unavailable", () => {
      resetLibrary({ current: project({ available: false }) });

      expect(runPageCommand("project.lock")).toBe(false);
    });
  });

  describe.each([
    { id: "project.terminal", call: () => mocked.openTerminal, needsAvailable: true },
    { id: "project.reveal", call: () => mocked.revealProject, needsAvailable: false },
    { id: "project.copyPath", call: () => mocked.copyProjectPath, needsAvailable: false },
    { id: "project.editFile", call: () => mocked.editProjectFile, needsAvailable: true },
  ])("$id", ({ id, call, needsAvailable }) => {
    it("runs on the current project", () => {
      resetLibrary({ current: alpha });

      expect(runPageCommand(id)).toBe(true);

      expect(call()).toHaveBeenCalledWith("alpha");
    });

    it("also runs on a locked project", () => {
      resetLibrary({ current: project({ locked: true }) });

      expect(runPageCommand(id)).toBe(true);

      expect(call()).toHaveBeenCalledWith("alpha");
    });

    it("is disabled without a current project", () => {
      expect(runPageCommand(id)).toBe(false);

      expect(call()).not.toHaveBeenCalled();
    });

    it(needsAvailable ? "is disabled for an unavailable project" : "also runs on an unavailable project", () => {
      resetLibrary({ current: project({ available: false }) });

      expect(runPageCommand(id)).toBe(!needsAvailable);

      if (needsAvailable) expect(call()).not.toHaveBeenCalled();
      else expect(call()).toHaveBeenCalledWith("alpha");
    });
  });

  describe("project.database", () => {
    it.each([
      { why: "an active project", current: alpha },
      { why: "a locked project", current: project({ locked: true }) },
    ])("opens the database of $why", ({ current }) => {
      resetLibrary({ current });

      expect(runPageCommand("project.database")).toBe(true);

      expect(mocked.openDatabase).toHaveBeenCalledWith("alpha");
    });

    it.each([
      { why: "there is no current project", current: null },
      { why: "the current project is unavailable", current: project({ available: false }) },
    ])("is disabled when $why", ({ current }) => {
      resetLibrary({ current });

      expect(runPageCommand("project.database")).toBe(false);

      expect(mocked.openDatabase).not.toHaveBeenCalled();
    });
  });

  describe("project.remove", () => {
    it("removes the current project", () => {
      resetLibrary({ current: alpha });

      expect(runPageCommand("project.remove")).toBe(true);

      expect(mocked.removeProject).toHaveBeenCalledWith("alpha");
    });

    it("also removes an unavailable project", () => {
      resetLibrary({ current: project({ available: false }) });

      expect(runPageCommand("project.remove")).toBe(true);

      expect(mocked.removeProject).toHaveBeenCalledWith("alpha");
    });

    it("is disabled without a current project", () => {
      expect(runPageCommand("project.remove")).toBe(false);
      expect(mocked.removeProject).not.toHaveBeenCalled();
    });
  });
});
