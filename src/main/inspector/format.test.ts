import { describe, expect, it } from "vitest";
import { ago, bytes, count, monthYear, short } from "./format";

const NOW = Date.UTC(2026, 5, 15, 12, 0, 0);
const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("bytes", () => {
  it("shows a small size in bytes", () => {
    expect(bytes(512)).toContain("512");
  });

  it("scales to kilobytes with one decimal", () => {
    expect(bytes(1536)).toContain("1.5");
  });

  it.each([
    { value: 1024 * 1024 * 3, digit: "3" },
    { value: 1024 * 1024 * 1024 * 2, digit: "2" },
  ])("scales $value bytes to a larger unit that reads $digit", ({ value, digit }) => {
    const text = bytes(value);

    expect(text).toContain(digit);
    expect(text).not.toBe(bytes(value / 1024));
  });
});

describe("count", () => {
  it("groups the digits of a large number", () => {
    expect(count(1234567)).toMatch(/^1\D?234\D?567$/);
  });

  it("keeps a small number as it is", () => {
    expect(count(42)).toBe("42");
  });
});

describe("short", () => {
  it.each([
    [42, "42"],
    [1834, "1.8K"],
    [12000, "12K"],
    [1_200_000, "1.2M"],
  ])("shortens %i to %j", (value, text) => {
    expect(short(value)).toBe(text);
  });
});

describe("monthYear", () => {
  it("names the month by its short English name and the year", () => {
    expect(monthYear(Date.UTC(2024, 2, 15, 12))).toBe("Mar 2024");
  });
});

describe("ago", () => {
  it("uses English words whatever the locale", () => {
    expect(ago(NOW - 3 * HOUR, NOW)).toBe("3 hours ago");
  });

  it("says now for under a minute", () => {
    expect(ago(NOW - 20 * SECOND, NOW)).toBe("now");
  });

  it.each([
    { label: "minutes", ms: NOW - 5 * MINUTE, text: /5 minutes ago/ },
    { label: "hours", ms: NOW - 3 * HOUR, text: /3 hours ago/ },
    { label: "days", ms: NOW - 3 * DAY, text: /3 days ago/ },
    { label: "weeks", ms: NOW - 14 * DAY, text: /2 weeks ago/ },
    { label: "months", ms: NOW - 90 * DAY, text: /3 months ago/ },
    { label: "years", ms: NOW - 800 * DAY, text: /2 years ago/ },
  ])("counts in $label", ({ ms, text }) => {
    expect(ago(ms, NOW)).toMatch(text);
  });

  it("says yesterday for a day back", () => {
    expect(ago(NOW - DAY, NOW)).toBe("yesterday");
  });

  it("looks ahead for a time in the future", () => {
    expect(ago(NOW + 5 * MINUTE, NOW)).toMatch(/in 5 minutes/);
  });
});
