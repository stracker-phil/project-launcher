import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { inspectorShown, resetStore, useStore } from "./store";

const mocked = vi.mocked(api, true);
const state = () => useStore.getState();

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("toasts", () => {
  it("shows a new toast with a count of 1", () => {
    state().toast("info", "Saved");

    expect(state().toasts).toMatchObject([{ kind: "info", message: "Saved", count: 1 }]);
  });

  it("counts up the same kind and message instead of adding a second toast", () => {
    state().toast("success", "Copied");
    state().toast("success", "Copied");

    expect(state().toasts).toMatchObject([{ message: "Copied", count: 2 }]);
  });

  it("keeps toasts apart that differ in kind or message", () => {
    state().toast("info", "Copied");
    state().toast("success", "Copied");
    state().toast("info", "Saved");

    expect(state().toasts.map((t) => `${t.kind}:${t.message}`)).toEqual(["info:Copied", "success:Copied", "info:Saved"]);
  });

  it("keeps at most 5 toasts and drops the oldest", () => {
    for (let i = 1; i <= 6; i++) state().toast("info", `Message ${i}`);

    expect(state().toasts.map((t) => t.message)).toEqual(["Message 2", "Message 3", "Message 4", "Message 5", "Message 6"]);
  });

  it("removes a dismissed toast and keeps the others", () => {
    state().toast("info", "First");
    state().toast("info", "Second");

    state().dismissToast(state().toasts[0].id);

    expect(state().toasts.map((t) => t.message)).toEqual(["Second"]);
  });

  it("reports an error toast through api.logError", () => {
    state().toast("error", "Could not open the demo project");

    expect(mocked.logError).toHaveBeenCalledWith("Could not open the demo project");
  });

  it("does not report an info toast through api.logError", () => {
    state().toast("info", "Saved");

    expect(mocked.logError).not.toHaveBeenCalled();
  });

  it("still shows an error toast when api.logError rejects", async () => {
    mocked.logError.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });

    state().toast("error", "Could not open the demo project");
    await Promise.resolve();

    expect(state().toasts).toMatchObject([{ kind: "error", message: "Could not open the demo project" }]);
  });
});

describe("confirm", () => {
  const question = { title: "Remove project?", text: "It stays on disk.", action: "Remove" };

  it("shows the question", () => {
    void state().confirm(question);

    expect(state().confirmation).toMatchObject(question);
  });

  it.each([true, false])("resolves %s when the sheet is answered with it, and closes the sheet", async (answer) => {
    const result = state().confirm(question);

    state().confirmation?.answer(answer);

    await expect(result).resolves.toBe(answer);
    expect(state().confirmation).toBeNull();
  });

  it("answers the first question with false when a second one is asked", async () => {
    const first = state().confirm(question);

    const second = state().confirm({ ...question, title: "Delete notes?" });

    await expect(first).resolves.toBe(false);
    expect(state().confirmation?.title).toBe("Delete notes?");
    state().confirmation?.answer(true);
    await expect(second).resolves.toBe(true);
  });
});

describe("layout", () => {
  it("shows the sidebar at first and toggles it", () => {
    expect(state().sidebarHidden).toBe(false);

    state().toggleSidebar();
    expect(state().sidebarHidden).toBe(true);

    state().toggleSidebar();
    expect(state().sidebarHidden).toBe(false);
  });

  it("starts with the inspector closed with the sidebar and open without it", () => {
    expect(inspectorShown(state())).toBe(false);

    state().toggleSidebar();

    expect(inspectorShown(state())).toBe(true);
  });

  it("toggles only the inspector choice of the current layout", () => {
    state().toggleInspector();
    expect(inspectorShown(state())).toBe(true);

    state().toggleSidebar();
    expect(inspectorShown(state())).toBe(true);

    state().toggleInspector();
    expect(inspectorShown(state())).toBe(false);

    state().toggleSidebar();
    expect(inspectorShown(state())).toBe(true);
  });

  it("remembers a closed inspector without the sidebar when the sidebar comes back", () => {
    state().toggleSidebar();
    state().toggleInspector();
    state().toggleSidebar();
    expect(inspectorShown(state())).toBe(false);

    state().toggleSidebar();

    expect(inspectorShown(state())).toBe(false);
  });
});
