import { describe, expect, it, vi } from "vitest";
import type { PaletteMode } from "./prefixes";
import { paletteSections, sourcedPrefixes } from "./sections";
import type { PaletteCommand } from "./sources";

const command = (mode: PaletteMode, label: string, section: string, detail?: string): PaletteCommand => ({
  id: `${section}:${label}`,
  mode,
  label,
  section,
  detail,
  run: vi.fn(),
});

const PROJECTS = [command("projects", "Alpha", "Projects"), command("projects", "Beta Shop", "Projects", "~/Projects/beta")];
const LIBRARY = [command("library", "All Projects", "Library", "library"), command("library", "Client", "Library", "tag")];
const ACTIONS = [command("actions", "Open in Editor", "Alpha"), command("actions", "Reveal in Finder", "Alpha")];
const MENU = [
  command("commands", "New Project", "File"),
  command("commands", "Week View", "View"),
  command("commands", "Settings: Status", "Settings", "log, version"),
];
const ALL = [...MENU, ...ACTIONS, ...LIBRARY, ...PROJECTS];

const outline = (sections: ReturnType<typeof paletteSections>) =>
  sections.map((s) => ({ heading: s.heading, items: s.items.map((i) => (i.kind === "recent" ? i.text : i.command.label)) }));

describe("paletteSections", () => {
  describe("without a query", () => {
    it("shows the recent searches first, then projects, library, the project's actions and the menus", () => {
      expect(outline(paletteSections(ALL, ["week"], "")).map((s) => s.heading)).toEqual([
        "Recent Searches",
        "Projects",
        "Library",
        "Alpha",
        "File",
        "View",
        "Settings",
      ]);
    });

    it("leaves out the recent searches when there are none", () => {
      expect(paletteSections(ALL, [], "").map((s) => s.heading)[0]).toBe("Projects");
    });

    it("keeps the order of the commands inside a section", () => {
      expect(outline(paletteSections(ALL, [], ""))[0]).toEqual({ heading: "Projects", items: ["Alpha", "Beta Shop"] });
    });
  });

  describe("with a prefix", () => {
    it.each([
      [">", ["File", "View", "Settings"]],
      [":", ["Projects"]],
      ["#", ["Library"]],
      ["/", ["Alpha"]],
    ])("keeps only the kind of the %s prefix and no recent searches", (text, headings) => {
      expect(paletteSections(ALL, ["week"], text).map((s) => s.heading)).toEqual(headings);
    });

    it("applies the query after the prefix", () => {
      expect(outline(paletteSections(ALL, [], ">  week"))).toEqual([{ heading: "Commands", items: ["Week View"] }]);
    });

    it("shows nothing for a prefix whose kind has no commands", () => {
      expect(paletteSections(MENU, ["week"], ":")).toEqual([]);
    });
  });

  describe("with a query", () => {
    it("ranks each kind's matches under its own heading, in kind order", () => {
      const found = paletteSections(ALL, ["x"], "pro");

      expect(outline(found)).toEqual([
        { heading: "Projects", items: ["Alpha", "Beta Shop"] },
        { heading: "Library", items: ["All Projects"] },
        { heading: "Commands", items: ["New Project"] },
      ]);
    });

    it("heads the current project's actions with its name", () => {
      expect(outline(paletteSections(ALL, [], "editor"))).toEqual([{ heading: "Alpha", items: ["Open in Editor"] }]);
    });

    it("ranks a label match above a match in the detail", () => {
      const commands = [command("commands", "Status Page", "Settings", "login"), command("commands", "Log Viewer", "View")];

      expect(outline(paletteSections(commands, [], "log"))[0].items).toEqual(["Log Viewer", "Status Page"]);
    });

    it("searches the detail and the section too", () => {
      expect(outline(paletteSections(ALL, [], "version"))[0].items).toEqual(["Settings: Status"]);
      expect(outline(paletteSections(ALL, [], "shop"))[0].items).toEqual(["Beta Shop"]);
    });

    it("shows nothing when a query matches no command", () => {
      expect(paletteSections(ALL, [], "zzzz")).toEqual([]);
    });
  });
});

describe("sourcedPrefixes", () => {
  it("is only the prefixes whose kind has commands, in prefix order", () => {
    expect(sourcedPrefixes([...MENU, ...ACTIONS]).map((p) => p.char)).toEqual([">", "/"]);
  });

  it("is every prefix when every kind has commands", () => {
    expect(sourcedPrefixes(ALL).map((p) => p.char)).toEqual([">", ":", "#", "/"]);
  });

  it("is empty without commands", () => {
    expect(sourcedPrefixes([])).toEqual([]);
  });
});
