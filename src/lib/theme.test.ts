import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyInitialLook, schemeFor, useAppearance, type Appearance } from "./theme";

const originalMatchMedia = window.matchMedia;

/** A matchMedia whose dark-mode query starts at `dark` and can be flipped like macOS does. */
function stubSystemScheme(dark: boolean) {
  let matches = dark;
  const handlers = new Set<() => void>();
  window.matchMedia = vi.fn((query: string) => ({
    get matches() {
      return matches;
    },
    media: query,
    addEventListener: (_: string, handler: () => void) => handlers.add(handler),
    removeEventListener: (_: string, handler: () => void) => handlers.delete(handler),
  })) as unknown as typeof window.matchMedia;
  return {
    set(next: boolean) {
      matches = next;
      act(() => handlers.forEach((h) => h()));
    },
  };
}

const scheme = () => document.documentElement.dataset.scheme;

beforeEach(() => {
  delete document.documentElement.dataset.scheme;
});

afterEach(() => {
  window.matchMedia = originalMatchMedia;
});

describe("schemeFor", () => {
  it.each<[Appearance, boolean, string]>([
    ["system", false, "light"],
    ["system", true, "dark"],
    ["light", false, "light"],
    ["light", true, "light"],
    ["dark", false, "dark"],
    ["dark", true, "dark"],
  ])("with %s appearance and system dark = %s it is %s", (appearance, systemDark, expected) => {
    expect(schemeFor(appearance, systemDark)).toBe(expected);
  });
});

describe("applyInitialLook", () => {
  it.each([
    [false, "light"],
    [true, "dark"],
  ])("with system dark = %s sets the %s scheme on the root element", (dark, expected) => {
    stubSystemScheme(dark);

    applyInitialLook();

    expect(scheme()).toBe(expected);
  });
});

describe("useAppearance", () => {
  it("applies the scheme of a fixed appearance", () => {
    stubSystemScheme(false);

    renderHook(() => useAppearance("dark"));

    expect(scheme()).toBe("dark");
  });

  it("follows macOS when no appearance is given", () => {
    stubSystemScheme(true);

    renderHook(() => useAppearance());

    expect(scheme()).toBe("dark");
  });

  it("follows macOS while the appearance is Automatic", () => {
    const system = stubSystemScheme(false);
    renderHook(() => useAppearance("system"));
    expect(scheme()).toBe("light");

    system.set(true);
    expect(scheme()).toBe("dark");

    system.set(false);
    expect(scheme()).toBe("light");
  });

  it("switches the scheme when the appearance changes", () => {
    stubSystemScheme(false);
    const { rerender } = renderHook(({ appearance }) => useAppearance(appearance), { initialProps: { appearance: "light" as Appearance } });

    rerender({ appearance: "dark" });

    expect(scheme()).toBe("dark");
  });

  it.each<[Appearance, string]>([
    ["light", "light"],
    ["dark", "dark"],
  ])("ignores a macOS change while the appearance is %s", (appearance, expected) => {
    const system = stubSystemScheme(appearance === "light");
    renderHook(() => useAppearance(appearance));
    expect(scheme()).toBe(expected);

    system.set(appearance !== "light");

    expect(scheme()).toBe(expected);
  });
});
