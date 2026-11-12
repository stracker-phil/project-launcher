import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resolveSettingsPage, searchSettings, settingsGroups, settingsPages } from "../registry/settingsPages";
import { registerCoreSettingsPages } from "./pages";

const mocked = vi.mocked(api, true);
const pageIds = (query: string) => [...new Set(searchSettings(query).map((match) => match.page.id))];

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
    expect(pageIds("shortcuts")).toContain("shortcuts");
  });

  it.each(["Git client", "Project file editor", "Docker app", "Open at login"])("finds the General page by its %s row", (title) => {
    registerCoreSettingsPages();

    expect(pageIds(title)).toEqual(["general"]);
  });

  it("finds a row by its keyword", () => {
    registerCoreSettingsPages();

    expect(searchSettings("colima").map((m) => m.row?.id)).toContain("docker");
  });

  it("lists the pages under App and System in order", () => {
    registerCoreSettingsPages();

    expect(settingsGroups().map(({ group, pages }) => [group, pages.map((p) => p.title)])).toEqual([
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

    expect(pageIds(title)).toContain(id);
  });

  it("finds the Status page's command row by its title", () => {
    registerCoreSettingsPages();

    expect(searchSettings("Command").filter((m) => m.page.id === "status").map((m) => m.row?.id)).toContain("cli");
  });

  it("keeps the Docker start and stop commands under the Docker app row", () => {
    registerCoreSettingsPages();

    const rows = settingsPages().find((p) => p.id === "general")?.rows ?? [];
    expect(rows.filter((row) => row.under).map((row) => [row.title, row.under])).toEqual([
      ["Start command", "docker"],
      ["Stop command", "docker"],
    ]);
  });

  it("opens the Privacy & Data page for its row ids", () => {
    registerCoreSettingsPages();

    expect(resolveSettingsPage("network")?.id).toBe("privacy");
  });

  it("adds the menu commands to the Shortcuts page's searchable rows", async () => {
    mocked.menuShortcuts.mockResolvedValue([{ menu: "File", label: "Quit", command: "app:quit", defaultKey: "Mod+Q", key: "Mod+Q" }]);

    registerCoreSettingsPages();

    const quit = () => searchSettings("quit").filter((m) => m.row?.title === "Quit");
    await vi.waitFor(() => expect(quit()).toHaveLength(1));
    expect(quit()[0].page.id).toBe("shortcuts");
  });

  it("finds a menu command by its title but not by the name of its menu", async () => {
    mocked.menuShortcuts.mockResolvedValue([{ menu: "File", label: "Archive project", command: "project:archive", defaultKey: null, key: null }]);

    registerCoreSettingsPages();

    const titles = (query: string) => searchSettings(query).map((m) => m.row?.title);
    await vi.waitFor(() => expect(titles("archive project")).toContain("Archive project"));
    expect(titles("file menu")).not.toContain("Archive project");
  });
});
