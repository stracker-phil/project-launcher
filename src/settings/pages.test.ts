import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resolveSettingsPage, searchSettingsPages, settingsGroups, settingsPages } from "../registry/settingsPages";
import { registerCoreSettingsPages } from "./pages";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  mocked.menuShortcuts.mockResolvedValue([]);
  mocked.logError.mockResolvedValue(undefined);
});

describe("registerCoreSettingsPages", () => {
  it("logs a failed read of the menu's shortcuts and keeps the Shortcuts page", async () => {
    mocked.menuShortcuts.mockRejectedValue({ kind: "io", message: "Permission denied", code: null });

    registerCoreSettingsPages();

    await vi.waitFor(() => expect(mocked.logError).toHaveBeenCalledWith(expect.stringContaining("Permission denied")));
    expect(searchSettingsPages("shortcuts").map((match) => match.page.id)).toContain("shortcuts");
  });

  it.each(["Git client", "Project file editor", "Docker app", "Open at login"])("finds the General page by its %s row", (title) => {
    registerCoreSettingsPages();

    expect(searchSettingsPages(title).map((match) => match.page.id)).toEqual(["general"]);
  });

  it("lists the pages under App and System in order", () => {
    registerCoreSettingsPages();

    expect(settingsGroups("").map(({ group, matches }) => [group, matches.map((m) => m.page.title)])).toEqual([
      ["App", ["General", "Appearance", "Shortcuts"]],
      ["System", ["Status", "Privacy & Data", "About"]],
    ]);
  });

  it("gives every page an intro for under its title", () => {
    registerCoreSettingsPages();

    expect(settingsPages().map((page) => page.intro?.length ?? 0).every((length) => length > 0)).toBe(true);
  });

  it.each([
    ["Accent color", "appearance"],
    ["On this Mac", "privacy"],
    ["Network", "privacy"],
  ])("finds the %s row on the %s page", (title, id) => {
    registerCoreSettingsPages();

    expect(searchSettingsPages(title).map((match) => match.page.id)).toContain(id);
  });

  it("opens the Privacy & Data page for its row ids", () => {
    registerCoreSettingsPages();

    expect(resolveSettingsPage("network")?.id).toBe("privacy");
  });

  it("adds the menu commands to the Shortcuts page's searchable rows", async () => {
    mocked.menuShortcuts.mockResolvedValue([{ menu: "File", label: "Quit", command: "app:quit", defaultKey: "Mod+Q", key: "Mod+Q" }]);

    registerCoreSettingsPages();

    await vi.waitFor(() => expect(searchSettingsPages("quit")).toHaveLength(1));
    expect(searchSettingsPages("quit")[0].page.id).toBe("shortcuts");
  });
});
