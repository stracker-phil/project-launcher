import { Info } from "lucide-react";
import { beforeAll, describe, expect, it } from "vitest";
import { registerSettingsPage, resolveSettingsPage, searchSettingsPages, settingsPages, type SettingsPage } from "./settingsPages";

const page = (patch: Partial<SettingsPage> & Pick<SettingsPage, "id" | "title">): SettingsPage => ({
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
