import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { resetLibrary, useLibrary } from "../state/library";
import { resetStore } from "../state/store";
import { project } from "../test/library";
import { addProjects, removeProject, rename, setLocked, toggleArchive, toggleStar } from "./actions";

const mocked = vi.mocked(api, true);
const diskFull = { kind: "io", message: "Disk full", code: null };

const alpha = project({ id: "alpha", name: "Alpha" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetLibrary();
  render(<Toasts />);
});

describe("addProjects", () => {
  it("selects the first added project under All Projects and puts its name into edit mode", async () => {
    resetLibrary({ filter: { kind: "starred" } });
    mocked.addProjectFolders.mockResolvedValueOnce({ added: ["demo-a"], existing: [] });

    await addProjects();

    const library = useLibrary.getState();
    expect(library.filter).toEqual({ kind: "all" });
    expect(library.selectedId).toBe("demo-a");
    expect(library.editing).toEqual({ id: "demo-a", field: "name" });
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("toasts how many projects were added when there are several, and selects the first", async () => {
    mocked.addProjectFolders.mockResolvedValueOnce({ added: ["demo-a", "demo-b"], existing: [] });

    await addProjects();

    expect(useLibrary.getState().selectedId).toBe("demo-a");
    expect(await screen.findByRole("status")).toHaveTextContent("Added 2 projects");
  });

  it.each([
    { existing: ["demo-x"], message: "This folder is already in your list" },
    { existing: ["demo-x", "demo-y"], message: "These folders are already in your list" },
  ])("selects the listed folder without renaming and toasts '$message'", async ({ existing, message }) => {
    resetLibrary({ filter: { kind: "archived" } });
    mocked.addProjectFolders.mockResolvedValueOnce({ added: [], existing });

    await addProjects();

    const library = useLibrary.getState();
    expect(library.filter).toEqual({ kind: "all" });
    expect(library.selectedId).toBe("demo-x");
    expect(library.editing).toBeNull();
    expect(await screen.findByRole("status")).toHaveTextContent(message);
  });

  it("changes nothing when no folder was chosen", async () => {
    resetLibrary({ filter: { kind: "starred" }, selectedId: "alpha" });
    mocked.addProjectFolders.mockResolvedValueOnce({ added: [], existing: [] });

    await addProjects();

    const library = useLibrary.getState();
    expect(library.filter).toEqual({ kind: "starred" });
    expect(library.selectedId).toBe("alpha");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows an error toast when adding fails", async () => {
    mocked.addProjectFolders.mockRejectedValueOnce(diskFull);

    await addProjects();

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t add project: Disk full");
    expect(useLibrary.getState().selectedId).toBeNull();
  });
});

describe("removeProject", () => {
  it("removes the project and toasts that the folder is untouched", async () => {
    await removeProject(alpha);

    expect(mocked.removeProject).toHaveBeenCalledWith("alpha");
    expect(await screen.findByRole("status")).toHaveTextContent("Removed “Alpha”. The folder and its config file are untouched.");
  });

  it("undoes the removal through the toast's Undo button", async () => {
    await removeProject(alpha);

    await userEvent.click(await screen.findByRole("button", { name: "Undo" }));

    expect(mocked.undo).toHaveBeenCalled();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows an error and no success toast when the removal fails", async () => {
    mocked.removeProject.mockRejectedValueOnce(diskFull);

    await removeProject(alpha);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t remove project: Disk full");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows an error when Undo fails", async () => {
    mocked.undo.mockRejectedValueOnce(diskFull);
    await removeProject(alpha);

    await userEvent.click(await screen.findByRole("button", { name: "Undo" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t restore project: Disk full");
  });
});

describe("toggleStar", () => {
  it.each([
    { starred: false, value: true },
    { starred: true, value: false },
  ])("sends starred $value for a project with starred $starred", async ({ starred, value }) => {
    await toggleStar(project({ meta: { starred } }));

    expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "starred", value });
  });

  it("shows an error toast when the update fails", async () => {
    mocked.setProjectMeta.mockRejectedValueOnce(diskFull);

    await toggleStar(alpha);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t update project: Disk full");
  });
});

describe("toggleArchive", () => {
  it.each([
    { archived: false, value: true, message: "Archived Alpha" },
    { archived: true, value: false, message: "Unarchived Alpha" },
  ])("sends archived $value and toasts '$message'", async ({ archived, value, message }) => {
    await toggleArchive(project({ meta: { archived } }));

    expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "archived", value });
    expect(await screen.findByRole("status")).toHaveTextContent(message);
  });

  it("shows only the error when the update fails", async () => {
    mocked.setProjectMeta.mockRejectedValueOnce(diskFull);

    await toggleArchive(alpha);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t update project: Disk full");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});

describe("setLocked", () => {
  it.each([true, false])("sends locked %s to Rust", async (locked) => {
    await setLocked(alpha, locked);

    expect(mocked.setProjectLocked).toHaveBeenCalledWith("alpha", locked);
  });

  it.each([
    { locked: true, message: "Couldn’t lock the project: Disk full" },
    { locked: false, message: "Couldn’t unlock the project: Disk full" },
  ])("shows '$message' when it fails", async ({ locked, message }) => {
    mocked.setProjectLocked.mockRejectedValueOnce(diskFull);

    await setLocked(alpha, locked);

    expect(await screen.findByRole("alert")).toHaveTextContent(message);
  });
});

describe("rename", () => {
  it("sends the new name to Rust", async () => {
    await rename(alpha, "Alpha Two");

    expect(mocked.setProjectName).toHaveBeenCalledWith("alpha", "Alpha Two");
  });

  it("shows an error toast when it fails", async () => {
    mocked.setProjectName.mockRejectedValueOnce({ kind: "invalid_input", message: "Name is empty", code: null });

    await rename(alpha, "");

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t update project: Name is empty");
  });
});
