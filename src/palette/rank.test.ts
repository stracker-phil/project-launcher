import { describe, expect, it } from "vitest";
import { fold, rankMatch, ranked } from "./rank";

describe("rankMatch", () => {
  it.each([
    ["the label starting with the query", "Toggle Sidebar", "", "tog", 0],
    ["a word of the label starting with the query", "Toggle Sidebar", "", "side", 1],
    ["every query word somewhere in label or detail", "Rename", "clients, billing", "billing ren", 2],
    ["the letters in order", "Toggle Sidebar", "", "tsb", 3],
  ])("ranks %s", (_tier, label, detail, query, expected) => {
    expect(rankMatch(label, detail, query)).toBe(expected);
  });

  it("finds a query word that sits in the middle of a label word", () => {
    expect(rankMatch("Toggle Sidebar", "", "bar")).toBe(2);
  });

  it("does not match when the letters are not in order", () => {
    expect(rankMatch("Toggle Sidebar", "", "bst")).toBeNull();
  });

  it("matches everything for an empty query", () => {
    expect(rankMatch("Toggle Sidebar", "", "")).toBe(0);
  });
});

describe("ranked", () => {
  const text = (item: { label: string; detail?: string }): [string, string] => [item.label, item.detail ?? ""];

  it("orders prefix, word start, every word, then letters in order", () => {
    const items = [
      { label: "Toggle Sidebar" },
      { label: "Open Folder", detail: "side panel" },
      { label: "Show Sidebar" },
      { label: "Side Menu" },
    ];

    expect(ranked(items, "side", text).map((i) => i.label)).toEqual(["Side Menu", "Toggle Sidebar", "Show Sidebar", "Open Folder"]);
  });

  it("ignores case and accents in the label and the query", () => {
    const items = [{ label: "Café Menü" }, { label: "Other" }];

    expect(ranked(items, "CAFE", text)).toEqual([{ label: "Café Menü" }]);
    expect(ranked([{ label: "Cafe Menu" }], "café", text)).toEqual([{ label: "Cafe Menu" }]);
  });

  it("keeps the input order of items that rank the same", () => {
    const items = [{ label: "Beta view" }, { label: "Alpha view" }, { label: "Gamma view" }];

    expect(ranked(items, "view", text).map((i) => i.label)).toEqual(["Beta view", "Alpha view", "Gamma view"]);
  });

  it("drops items that do not match", () => {
    const items = [{ label: "Rename" }, { label: "Delete" }];

    expect(ranked(items, "zzz", text)).toEqual([]);
  });

  it("trims the query", () => {
    expect(ranked([{ label: "Rename" }], "  ren  ", text)).toHaveLength(1);
  });
});

describe("fold", () => {
  it("lowercases and removes accents", () => {
    expect(fold("Éclair Ñandú")).toBe("eclair nandu");
  });
});
