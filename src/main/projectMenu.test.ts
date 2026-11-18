import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { isItem, type MenuItem } from "../components/menu";
import { resetStore } from "../state/store";
import { project } from "../test/library";
import { projectMenu } from "./projectMenu";

const mocked = vi.mocked(api, true);

const item = (text: string, shown = project()): MenuItem => {
  const found = projectMenu(shown, []).find((entry): entry is MenuItem => isItem(entry) && entry.text === text);
  if (!found) throw new Error(`No menu item "${text}"`);
  return found;
};

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("projectMenu", () => {
  it("lists the launch entries after the editing ones", () => {
    const texts = projectMenu(project(), []).flatMap((entry) => (isItem(entry) ? [entry.text] : []));

    expect(texts.slice(texts.indexOf("Open Terminal Here"), texts.indexOf("Remove from List"))).toEqual([
      "Open Terminal Here",
      "Reveal in Finder",
      "Copy Path",
      "Manage Claude",
      "Edit Project File…",
    ]);
  });

  it.each([
    { text: "Open Terminal Here", call: () => mocked.openTerminal },
    { text: "Reveal in Finder", call: () => mocked.revealProject },
    { text: "Copy Path", call: () => mocked.copyProjectPath },
    { text: "Edit Project File…", call: () => mocked.editProjectFile },
  ])("$text acts on the project", ({ text, call }) => {
    item(text).run?.();

    expect(call()).toHaveBeenCalledWith("alpha");
  });

  it.each(["Open Terminal Here", "Edit Project File…"])("disables %s for an unavailable project", (text) => {
    expect(item(text, project({ available: false })).enabled).toBe(false);
    expect(item(text).enabled).toBe(true);
  });

  it("places Manage Claude after Copy Path", () => {
    const texts = projectMenu(project(), []).flatMap((entry) => (isItem(entry) ? [entry.text] : []));

    expect(texts.indexOf("Manage Claude")).toBe(texts.indexOf("Copy Path") + 1);
  });

  it("opens Claude Manager for the project", () => {
    item("Manage Claude", project({ canManageClaude: true })).run?.();

    expect(mocked.openClaudeManager).toHaveBeenCalledWith("alpha");
  });

  it.each([
    { why: "the manager can manage it and it is available", patch: { canManageClaude: true }, enabled: true },
    { why: "the manager cannot manage it", patch: { canManageClaude: false }, enabled: false },
    { why: "the project is unavailable", patch: { canManageClaude: true, available: false }, enabled: false },
  ])("enables Manage Claude only when it applies: $why", ({ patch, enabled }) => {
    expect(item("Manage Claude", project(patch)).enabled).toBe(enabled);
  });

  it.each(["Reveal in Finder", "Copy Path"])("keeps %s enabled for an unavailable project", (text) => {
    expect(item(text, project({ available: false })).enabled).not.toBe(false);
  });
});
