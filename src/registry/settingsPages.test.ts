import { Info } from "lucide-react";
import { beforeAll, describe, expect, it } from "vitest";
import { findRow, registerSettingsPage, resolveSettingsPage, searchSettings, settingsGroups, settingsPages, type SettingsPage } from "./settingsPages";

const page = (patch: Partial<SettingsPage> & Pick<SettingsPage, "id" | "title">): SettingsPage => ({
  group: "App",
  icon: Info,
  order: 0,
  component: () => null,
  ...patch,
});

const mine = (ids: string[]) => ids.filter((id) => id.startsWith("t-"));
const mineInOrder = () => mine(settingsPages().map((p) => p.id));
const hits = (query: string) => searchSettings(query).map((m) => (m.row ? `${m.page.id}/${m.row.id}` : m.page.id));

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
      rows: [
        { id: "t-numbat-row", title: "Numbat Habitat" },
        { id: "t-numbat-nest", title: "Numbat Nest", keywords: ["burrow"] },
      ],
    }),
  );
  registerSettingsPage(
    page({
      id: "t-ranking",
      title: "Ranking Pier",
      order: -75,
      rows: [
        { id: "t-keyword-hit", title: "Tide", keywords: ["walrus"] },
        { id: "t-word-hit", title: "Giant Walrus" },
        { id: "t-prefix-hit", title: "Walrus Tusks" },
      ],
    }),
  );
});

describe("settingsPages", () => {
  it("lists pages by order, then by title", () => {
    expect(mineInOrder()).toEqual(["t-first", "t-alpha", "t-beta", "t-quokka", "t-ranking"]);
  });

  it("replaces the page registered under the same id", () => {
    registerSettingsPage(page({ id: "t-replaced", title: "Old Title", order: -70 }));
    registerSettingsPage(page({ id: "t-replaced", title: "New Title", order: -70 }));

    const matches = settingsPages().filter((p) => p.id === "t-replaced");
    expect(matches.map((p) => p.title)).toEqual(["New Title"]);
  });
});

describe("findRow", () => {
  const quokka = () => settingsPages().find((p) => p.id === "t-quokka")!;

  it("returns the row with the given id", () => {
    expect(findRow(quokka(), "t-numbat-nest")?.title).toBe("Numbat Nest");
  });

  it.each([["nope"], [undefined]])("returns nothing for %s", (id) => {
    expect(findRow(quokka(), id)).toBeUndefined();
  });

  it("returns nothing for a page without rows", () => {
    expect(findRow(settingsPages().find((p) => p.id === "t-beta")!, "t-numbat-row")).toBeUndefined();
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

describe("searchSettings", () => {
  it("finds a page by its title in any case", () => {
    expect(hits("QUOKKA")).toEqual(["t-quokka"]);
  });

  it("finds a page by a keyword in any case", () => {
    expect(hits("wombat")).toEqual(["t-quokka"]);
  });

  it("finds a row by its title", () => {
    expect(hits("habitat")).toEqual(["t-quokka/t-numbat-row"]);
  });

  it("finds a row by a keyword of the row", () => {
    expect(hits("burrow")).toEqual(["t-quokka/t-numbat-nest"]);
  });

  it("lists every matching row of one page", () => {
    expect(hits("numbat")).toEqual(["t-quokka/t-numbat-row", "t-quokka/t-numbat-nest"]);
  });

  it("does not find a row by its page's title or keywords", () => {
    expect(hits("island")).toEqual(["t-quokka"]);
    expect(hits("wombat")).not.toContain("t-quokka/t-numbat-row");
  });

  it("ranks a title start before a word of the title before a keyword", () => {
    expect(hits("walrus")).toEqual(["t-ranking/t-prefix-hit", "t-ranking/t-word-hit", "t-ranking/t-keyword-hit"]);
  });

  it("returns nothing when nothing matches", () => {
    expect(searchSettings("zzzqqq")).toEqual([]);
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
    expect(hits("zebra crossing")).toEqual(["t-zebra"]);
  });

  it("collects the pages of each group under its heading", () => {
    const groups = settingsGroups();

    expect(groups.map((g) => g.group)).toEqual([...new Set(settingsPages().map((p) => p.group))]);
    expect(groups.find((g) => g.group === "App")?.pages.map((p) => p.id)).toContain("t-quokka");
    expect(groups.find((g) => g.group === "Aardvark Den")?.pages.map((p) => p.id)).toEqual(["t-aardvark"]);
  });
});
