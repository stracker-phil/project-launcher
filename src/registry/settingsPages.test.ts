import { Info } from "lucide-react";
import { beforeAll, describe, expect, it } from "vitest";
import { registerSettingsPage, resolveSettingsPage, searchSettingsPages, settingsGroups, settingsPages, type SettingsPage } from "./settingsPages";

const page = (patch: Partial<SettingsPage> & Pick<SettingsPage, "id" | "title">): SettingsPage => ({
  group: "App",
  icon: Info,
  order: 0,
  component: () => null,
  ...patch,
});

const mine = (ids: string[]) => ids.filter((id) => id.startsWith("t-"));
const mineInOrder = () => mine(settingsPages().map((p) => p.id));

beforeAll(() => {
  registerSettingsPage(page({ id: "t-beta", title: "Beta", order: -90 }));
  registerSettingsPage(page({ id: "t-alpha", title: "Alpha", order: -90 }));
  registerSettingsPage(page({ id: "t-first", title: "Zed", order: -100 }));
  registerSettingsPage(
    page({
      id: "t-quokka",
      title: "Quokka Island",
      order: -80,
      keywords: ["Wombat"],
      rows: [{ id: "t-numbat-row", title: "Numbat Habitat" }],
    }),
  );
});

describe("settingsPages", () => {
  it("lists pages by order, then by title", () => {
    expect(mineInOrder()).toEqual(["t-first", "t-alpha", "t-beta", "t-quokka"]);
  });

  it("replaces the page registered under the same id", () => {
    registerSettingsPage(page({ id: "t-replaced", title: "Old Title", order: -70 }));
    registerSettingsPage(page({ id: "t-replaced", title: "New Title", order: -70 }));

    const matches = settingsPages().filter((p) => p.id === "t-replaced");
    expect(matches.map((p) => p.title)).toEqual(["New Title"]);
  });
});

describe("resolveSettingsPage", () => {
  it("returns the page with the given id", () => {
    expect(resolveSettingsPage("t-beta")?.title).toBe("Beta");
  });

  it("returns the page holding the given row id", () => {
    expect(resolveSettingsPage("t-numbat-row")?.id).toBe("t-quokka");
  });

  it.each([["nope"], [undefined]])("falls back to the first page for %s", (id) => {
    expect(resolveSettingsPage(id)?.id).toBe(settingsPages()[0].id);
  });
});

describe("searchSettingsPages", () => {
  it("finds a page by its title in any case", () => {
    expect(searchSettingsPages("QUOKKA").map((m) => [m.page.id, m.row])).toEqual([["t-quokka", undefined]]);
  });

  it("finds a page by a keyword in any case", () => {
    expect(searchSettingsPages("wombat").map((m) => [m.page.id, m.row])).toEqual([["t-quokka", undefined]]);
  });

  it("finds a page by a row title and returns that row", () => {
    const matches = searchSettingsPages("numbat");

    expect(matches.map((m) => [m.page.id, m.row?.id])).toEqual([["t-quokka", "t-numbat-row"]]);
  });

  it("returns every page for an empty query", () => {
    expect(searchSettingsPages("").map((m) => m.page.id)).toEqual(settingsPages().map((p) => p.id));
  });

  it("returns nothing when no page matches", () => {
    expect(searchSettingsPages("zzzqqq")).toEqual([]);
  });
});

describe("page groups", () => {
  beforeAll(() => {
    registerSettingsPage(page({ id: "t-system", title: "Disks", group: "System", order: -300 }));
    registerSettingsPage(page({ id: "t-zebra", title: "Stripes", group: "Zebra Crossing", order: -300 }));
    registerSettingsPage(page({ id: "t-aardvark", title: "Ants", group: "Aardvark Den", order: 500 }));
  });

  it("lists App, then System, then other groups by name, whatever the order values", () => {
    const groups = settingsPages().map((p) => p.group);

    expect(groups.indexOf("System")).toBeGreaterThan(groups.lastIndexOf("App"));
    expect(groups.indexOf("Aardvark Den")).toBeGreaterThan(groups.lastIndexOf("System"));
    expect(groups.indexOf("Zebra Crossing")).toBeGreaterThan(groups.lastIndexOf("Aardvark Den"));
  });

  it("finds a page by its group name", () => {
    expect(searchSettingsPages("zebra crossing").map((m) => [m.page.id, m.row])).toEqual([["t-zebra", undefined]]);
  });

  it("collects the matches of each group under its heading", () => {
    const groups = settingsGroups("");

    expect(groups.map((g) => g.group)).toEqual([...new Set(settingsPages().map((p) => p.group))]);
    expect(groups.find((g) => g.group === "App")?.matches.map((m) => m.page.id)).toContain("t-quokka");
  });

  it("keeps only the groups that have a match for the query", () => {
    expect(settingsGroups("ants").map((g) => [g.group, g.matches.map((m) => m.page.id)])).toEqual([["Aardvark Den", ["t-aardvark"]]]);
  });

  it("returns no groups when nothing matches", () => {
    expect(settingsGroups("zzzqqq")).toEqual([]);
  });
});
