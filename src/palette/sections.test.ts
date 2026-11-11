import { describe, expect, it, vi } from "vitest";
import { paletteSections, SOURCED_PREFIXES } from "./sections";
import type { PaletteCommand } from "./sources";

const command = (label: string, section: string, detail?: string): PaletteCommand => ({
  id: `${section}:${label}`,
  label,
  section,
  detail,
  run: vi.fn(),
});

const COMMANDS = [
  command("File: New Project", "File"),
  command("File: Open Folder", "File"),
  command("View: Week View", "View"),
  command("Settings: Status", "Settings", "log, version"),
];

const outline = (sections: ReturnType<typeof paletteSections>) =>
  sections.map((s) => ({ heading: s.heading, items: s.items.map((i) => (i.kind === "recent" ? i.text : i.command.label)) }));

describe("paletteSections", () => {
  it("shows the recent searches first and then the commands under their menus when the field is empty", () => {
    expect(outline(paletteSections(COMMANDS, ["week"], ""))).toEqual([
      { heading: "Recent Searches", items: ["week"] },
      { heading: "File", items: ["File: New Project", "File: Open Folder"] },
      { heading: "View", items: ["View: Week View"] },
      { heading: "Settings", items: ["Settings: Status"] },
    ]);
  });

  it("leaves out the recent searches when there are none", () => {
    expect(paletteSections(COMMANDS, [], "").map((s) => s.heading)).toEqual(["File", "View", "Settings"]);
  });

  it("shows the sections without the recent searches for the commands prefix alone", () => {
    expect(paletteSections(COMMANDS, ["week"], ">").map((s) => s.heading)).toEqual(["File", "View", "Settings"]);
  });

  it("shows one ranked Commands section for a query", () => {
    const found = paletteSections(COMMANDS, ["week"], "open");

    expect(outline(found)).toEqual([{ heading: "Commands", items: ["File: Open Folder"] }]);
  });

  it("ranks a label match above a match in the detail", () => {
    const commands = [command("Status Page", "Settings", "login"), command("Log Viewer", "View")];

    expect(outline(paletteSections(commands, [], "log"))[0].items).toEqual(["Log Viewer", "Status Page"]);
  });

  it("searches the detail and the section too", () => {
    expect(outline(paletteSections(COMMANDS, [], "version"))[0].items).toEqual(["Settings: Status"]);
    expect(outline(paletteSections(COMMANDS, [], "week"))[0].items).toEqual(["View: Week View"]);
  });

  it("applies the query after the commands prefix", () => {
    expect(outline(paletteSections(COMMANDS, [], ">  open"))).toEqual([{ heading: "Commands", items: ["File: Open Folder"] }]);
  });

  it("shows nothing when a query matches no command", () => {
    expect(paletteSections(COMMANDS, [], "zzzz")).toEqual([]);
  });

  it.each([":", "#", "/", ":proj"])("shows nothing for the prefix %s that has no source yet", (text) => {
    expect(paletteSections(COMMANDS, ["week"], text)).toEqual([]);
  });
});

describe("SOURCED_PREFIXES", () => {
  it("is only the commands prefix", () => {
    expect(SOURCED_PREFIXES.map((p) => p.char)).toEqual([">"]);
  });
});
