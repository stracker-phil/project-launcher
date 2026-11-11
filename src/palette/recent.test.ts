import { beforeEach, describe, expect, it } from "vitest";
import { clearRecentSearches, recentSearches, rememberSearch } from "./recent";

describe("recent searches", () => {
  beforeEach(clearRecentSearches);

  it("lists the newest search first", () => {
    rememberSearch("sidebar");
    rememberSearch("week");

    expect(recentSearches()).toEqual(["week", "sidebar"]);
  });

  it("moves a repeated search to the front instead of listing it twice", () => {
    rememberSearch("sidebar");
    rememberSearch("week");
    rememberSearch("sidebar");

    expect(recentSearches()).toEqual(["sidebar", "week"]);
  });

  it("keeps the five newest", () => {
    for (const text of ["a", "b", "c", "d", "e", "f"]) rememberSearch(text);

    expect(recentSearches()).toEqual(["f", "e", "d", "c", "b"]);
  });

  it("stores a trimmed search and ignores a blank one", () => {
    rememberSearch("  week  ");
    rememberSearch("   ");

    expect(recentSearches()).toEqual(["week"]);
  });

  it("returns a copy that does not change the list", () => {
    rememberSearch("week");

    recentSearches().push("other");

    expect(recentSearches()).toEqual(["week"]);
  });
});
