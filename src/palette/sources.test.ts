import { Settings } from "lucide-react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuCommand, type Sidebar } from "../api";
import { registerSettingsPage } from "../registry/settingsPages";
import { action, project } from "../test/library";
import { actionCommands, libraryCommands, menuCommands, projectCommands, settingsCommands, type PaletteCommand } from "./sources";

const mocked = vi.mocked(api, true);

const menuRow = (patch: Partial<MenuCommand> = {}): MenuCommand => ({ menu: "File", label: "New Project", command: "app:new", key: null, ...patch });
const find = (commands: PaletteCommand[], id: string) => commands.find((c) => c.id === id);
const noIcon = () => null;

beforeEach(() => {
  vi.clearAllMocks();
});

describe("menuCommands", () => {
  it("lists a menu item as a command under its menu with the key Rust shows", () => {
    const found = find(menuCommands([menuRow({ key: "⌘N" })]), "app:new");

    expect(found).toMatchObject({ label: "New Project", section: "File", mode: "commands", shortcut: "⌘N" });
  });

  it("has no shortcut for an item without a key", () => {
    expect(find(menuCommands([menuRow()]), "app:new")?.shortcut).toBeUndefined();
  });

  it("runs the bus command by id", () => {
    find(menuCommands([menuRow()]), "app:new")?.run();

    expect(mocked.runCommand).toHaveBeenCalledWith("app:new");
  });

  it("leaves out the Project menu's items and the palette's own command", () => {
    const commands = menuCommands([
      menuRow(),
      menuRow({ menu: "Project", label: "Reveal in Finder", command: "app:reveal" }),
      menuRow({ menu: "View", label: "Command Palette", command: "app:palette" }),
    ]);

    expect(commands.map((c) => c.id)).toEqual(["app:new"]);
  });
});

describe("settingsCommands", () => {
  registerSettingsPage({
    id: "palette-test-shortcuts",
    title: "Shortcuts",
    icon: Settings,
    order: 1,
    keywords: ["keys", "bindings"],
    rows: [{ id: "reset", title: "Reset all shortcuts" }],
    component: () => null,
  });
  registerSettingsPage({ id: "palette-test-plain", title: "Plain", icon: Settings, order: 2, component: () => null });

  it("lists each page as 'Settings: <title>' with its keywords and row titles as detail", () => {
    expect(find(settingsCommands(), "app:settings:palette-test-shortcuts")).toMatchObject({
      label: "Settings: Shortcuts",
      section: "Settings",
      mode: "commands",
      detail: "keys, bindings, Reset all shortcuts",
    });
  });

  it("has no detail for a page without keywords or rows", () => {
    expect(find(settingsCommands(), "app:settings:palette-test-plain")?.detail).toBeUndefined();
  });

  it("opens the page through the bus command with its id", () => {
    find(settingsCommands(), "app:settings:palette-test-shortcuts")?.run();

    expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:palette-test-shortcuts");
  });
});

describe("projectCommands", () => {
  const alpha = project({ id: "alpha", name: "Alpha", path: "~/Projects/alpha", meta: { tag: "Client", notes: ["Launch in May"] } });
  const beta = project({ id: "beta", name: "Beta", path: "~/Projects/beta" });
  const old = project({ id: "old", name: "Old Site", meta: { archived: true, tag: "Client" } });

  it("lists archived projects last", () => {
    const commands = projectCommands([old, alpha, beta], null, noIcon);

    expect(commands.map((c) => c.label)).toEqual(["Alpha", "Beta", "Old Site"]);
  });

  it("hints the current project, an archived one, and otherwise the tag", () => {
    const commands = projectCommands([alpha, beta, old], "beta", noIcon);

    expect(commands.map((c) => c.hint)).toEqual(["Client", "Current", "Archived"]);
  });

  it("marks the current project even when it is archived", () => {
    expect(find(projectCommands([old], "old", noIcon), "project:old")?.hint).toBe("Current");
  });

  it("searches the tag, path and notes along with the name", () => {
    const found = find(projectCommands([alpha], null, noIcon), "project:alpha");

    expect(found).toMatchObject({ mode: "projects", section: "Projects" });
    expect(found?.detail).toContain("Client");
    expect(found?.detail).toContain("~/Projects/alpha");
    expect(found?.detail).toContain("Launch in May");
  });

  it("shows the project in the main window through the bus", () => {
    find(projectCommands([alpha], null, noIcon), "project:alpha")?.run();

    expect(mocked.runCommand).toHaveBeenCalledWith("project:show:alpha");
  });
});

describe("libraryCommands", () => {
  const sidebar: Sidebar = { all: 5, starred: 1, untagged: 2, archived: 1, tags: [{ name: "Client", count: 2 }, { name: "Ideas", count: 0 }] };

  it("lists All, Starred, the tags, Untagged and Archived with their counts", () => {
    const commands = libraryCommands(sidebar, noIcon);

    expect(commands.map((c) => [c.label, c.hint])).toEqual([
      ["All Projects", "5"],
      ["Starred", "1"],
      ["Client", "2"],
      ["Ideas", "0"],
      ["Untagged", "2"],
      ["Archived", "1"],
    ]);
    expect(commands.every((c) => c.mode === "library" && c.section === "Library")).toBe(true);
  });

  it("leaves out Untagged when the sidebar has none", () => {
    const commands = libraryCommands({ ...sidebar, untagged: null }, noIcon);

    expect(commands.map((c) => c.label)).not.toContain("Untagged");
  });

  it("asks the main window for the filter through a page request", () => {
    const commands = libraryCommands(sidebar, noIcon);

    find(commands, 'filter:{"kind":"tag","tag":"Client"}')?.run();
    find(commands, 'filter:{"kind":"archived"}')?.run();

    expect(mocked.runCommand).toHaveBeenCalledWith('page:filter:{"kind":"tag","tag":"Client"}');
    expect(mocked.runCommand).toHaveBeenCalledWith('page:filter:{"kind":"archived"}');
  });
});

describe("actionCommands", () => {
  const alpha = project({ id: "alpha", name: "Alpha" });
  const editor = action("Open in Editor", { shortcut: "Mod+O" });
  const build = action("Build site", { command: "npm run build" }, "scripts", 1);
  const projectItem = menuRow({ menu: "Project", label: "Reveal in Finder", command: "app:reveal", key: "⌘R" });

  it("lists the project's actions, then the Project menu's items, under the project's name", () => {
    const commands = actionCommands(alpha, [editor, build], [menuRow(), projectItem], noIcon);

    expect(commands.map((c) => [c.label, c.section, c.mode])).toEqual([
      ["Open in Editor", "Alpha", "actions"],
      ["Build site", "Alpha", "actions"],
      ["Reveal in Finder", "Alpha", "actions"],
    ]);
  });

  it("shows an action's shortcut and marks a script as one in the detail", () => {
    const commands = actionCommands(alpha, [editor, build], [], noIcon);

    expect(find(commands, "action:apps:0")?.shortcut).toBe("⌘O");
    expect(find(commands, "action:scripts:1")?.detail).toBe("run script");
  });

  it("launches an action through a page request naming its list and index", () => {
    find(actionCommands(alpha, [editor, build], [], noIcon), "action:scripts:1")?.run();

    expect(mocked.runCommand).toHaveBeenCalledWith("page:action:scripts:1");
  });

  it("runs a Project menu item by its bus id with the key Rust shows", () => {
    const found = find(actionCommands(alpha, [], [projectItem], noIcon), "app:reveal");

    expect(found?.shortcut).toBe("⌘R");
    found?.run();
    expect(mocked.runCommand).toHaveBeenCalledWith("app:reveal");
  });
});
