import { afterEach, describe, expect, it, vi } from "vitest";
import { trackWindowFocus } from "./windowFocus";

const blurred = () => document.documentElement.classList.contains("window-blurred");

/** Makes the document report `focused`, then fires the window event macOS gives. */
function setFocus(focused: boolean) {
  vi.spyOn(document, "hasFocus").mockReturnValue(focused);
  window.dispatchEvent(new Event(focused ? "focus" : "blur"));
}

afterEach(() => {
  vi.restoreAllMocks();
  document.documentElement.classList.remove("window-blurred");
});

describe("trackWindowFocus", () => {
  it("marks the page blurred at once when the window is not key", () => {
    vi.spyOn(document, "hasFocus").mockReturnValue(false);

    const stop = trackWindowFocus();

    expect(blurred()).toBe(true);
    stop();
  });

  it("leaves the page unmarked while the window is key", () => {
    vi.spyOn(document, "hasFocus").mockReturnValue(true);

    const stop = trackWindowFocus();

    expect(blurred()).toBe(false);
    stop();
  });

  it("marks the page on blur and clears it on focus", () => {
    vi.spyOn(document, "hasFocus").mockReturnValue(true);
    const stop = trackWindowFocus();

    setFocus(false);
    expect(blurred()).toBe(true);

    setFocus(true);
    expect(blurred()).toBe(false);
    stop();
  });

  it("clears the mark and stops listening when stopped", () => {
    vi.spyOn(document, "hasFocus").mockReturnValue(false);
    const stop = trackWindowFocus();

    stop();
    expect(blurred()).toBe(false);

    setFocus(false);
    expect(blurred()).toBe(false);
  });
});
