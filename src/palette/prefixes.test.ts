import { describe, expect, it } from "vitest";
import { parsePrefix } from "./prefixes";

describe("parsePrefix", () => {
  it.each([
    [">", "commands"],
    [":", "projects"],
    ["#", "library"],
    ["/", "actions"],
  ])("reads %s as the %s prefix", (char, mode) => {
    expect(parsePrefix(`${char}query`).prefix?.mode).toBe(mode);
  });

  it("returns the query after the prefix, trimmed", () => {
    expect(parsePrefix(">  toggle sidebar  ")).toMatchObject({ prefix: { char: ">" }, query: "toggle sidebar" });
  });

  it("returns an empty query for a prefix alone", () => {
    expect(parsePrefix(">")).toMatchObject({ prefix: { char: ">" }, query: "" });
  });

  it("returns no prefix and the trimmed text when none is typed", () => {
    expect(parsePrefix("  toggle ")).toEqual({ prefix: null, query: "toggle" });
  });

  it("only reads a prefix in the first character", () => {
    expect(parsePrefix("a > b").prefix).toBeNull();
  });
});
