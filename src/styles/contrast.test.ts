/// <reference types="node" />
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { contrastRatio, hexToRgb, over, type Rgb } from "./contrast";

const SCHEMES = ["light", "dark"] as const;

const sheets = Object.fromEntries(
  SCHEMES.map((scheme) => [scheme, readFileSync(join(resolve(process.cwd(), "src"), "styles", "palettes", `${scheme}.css`), "utf8")]),
);

describe("contrastRatio", () => {
  it("is 21 for black on white and 1 for identical colors", () => {
    expect(contrastRatio(hexToRgb("#000000"), hexToRgb("#ffffff"))).toBeCloseTo(21, 5);
    expect(contrastRatio(hexToRgb("#777777"), hexToRgb("#777777"))).toBe(1);
  });
});

/** Splits `a, b(c, d), e` at the commas outside parentheses. */
function splitArgs(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "(") depth++;
    else if (text[i] === ")") depth--;
    else if (text[i] === "," && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map((p) => p.trim());
}

/**
 * Resolves a palette's tokens to colors: hex, rgba(), var(--x) and color-mix(in srgb, A P%, B). Mixing with
 * `transparent` composites A at P% over the background.
 */
function paletteColors(css: string) {
  function token(name: string): string {
    const m = css.match(new RegExp(`^\\s*${name}:\\s*([^;]+);`, "m"));
    if (!m) throw new Error(`missing token ${name}`);
    return m[1].trim();
  }

  function resolve(value: string, bg?: Rgb): Rgb {
    if (value.startsWith("#")) return hexToRgb(value);
    const variable = value.match(/^var\((--[\w-]+)\)$/);
    if (variable) return resolve(token(variable[1]), bg);
    const rgba = value.match(/^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/);
    if (rgba && bg) return over([+rgba[1] / 255, +rgba[2] / 255, +rgba[3] / 255], +rgba[4], bg);
    const mix = value.match(/^color-mix\(in srgb,(.*)\)$/);
    if (mix) {
      const [first, second] = splitArgs(mix[1]);
      const share = first.match(/^(.+?)\s+([\d.]+)%$/);
      if (share && second) {
        const alpha = +share[2] / 100;
        if (second === "transparent") {
          if (!bg) throw new Error(`cannot resolve ${value} without a background`);
          return over(resolve(share[1], bg), alpha, bg);
        }
        return over(resolve(share[1], bg), alpha, resolve(second, bg));
      }
    }
    throw new Error(`cannot resolve ${value}`);
  }

  const color = (name: string, bg?: Rgb): Rgb => resolve(token(name), bg);

  return (fg: string, bg: string) => {
    const background = color(bg);
    return contrastRatio(color(fg, background), background);
  };
}

describe.each(SCHEMES)("%s palette contrast", (scheme) => {
  const ratio = paletteColors(sheets[scheme]);

  it.each(["--text", "--text-secondary", "--text-tertiary", "--text-muted", "--danger-text", "--link", "--accent-text"])(
    "%s reads at 4.5:1 on the panel",
    (fg) => {
      expect(ratio(fg, "--panel-bg")).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each([
    ["--selection-text", "--selection-bg"],
    ["--text-on-accent", "--accent"],
    ["--button-primary-text", "--button-primary-bg"],
    ["--banner-info-text", "--banner-info-bg"],
    ["--banner-warn-text", "--banner-warn-bg"],
    ["--banner-error-text", "--banner-error-bg"],
    ["--tooltip-text", "--tooltip-bg"],
    ["--sidebar-text", "--sidebar-bg"],
    ["--sidebar-text-tertiary", "--sidebar-bg"],
  ])("%s reads at 4.5:1 on %s", (fg, bg) => {
    expect(ratio(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["--settings-text", "--settings-text-secondary", "--settings-text-strong"])(
    "%s reads at 4.5:1 on the settings background",
    (fg) => {
      expect(ratio(fg, "--settings-bg")).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each(["--settings-bg", "--settings-chrome"])("--settings-text-tertiary reads at 3:1 on %s", (bg) => {
    expect(ratio("--settings-text-tertiary", bg)).toBeGreaterThanOrEqual(3);
  });
});
