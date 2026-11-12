import { render, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { settingsPages } from "../registry/settingsPages";
import { resetStore } from "../state/store";
import { DEFAULT_SETTINGS } from "../test/apiMock";
import { SettingsApp } from "../windows/settings/SettingsApp";
import { registerCoreSettingsPages } from "./pages";

const mocked = vi.mocked(api, true);

const PAGE_IDS = ["general", "appearance", "shortcuts", "status", "privacy", "about"];

const anchored = () => [...document.querySelectorAll<HTMLElement>("[data-anchor]")];

/** The title a row or group is known by on screen. */
function shownTitle(element: HTMLElement): string | undefined {
  const text = element.matches(".settings-row") ? element.querySelector(".settings-row-label") : element.querySelector("h3");
  return text?.textContent ?? undefined;
}

async function renderPageLoaded(id: string) {
  window.history.replaceState({}, "", `/?page=${id}`);
  render(<SettingsApp client={createQueryClient()} />);
  await waitFor(() => expect(document.querySelector(".settings-row"), `page "${id}" shows no rows`).not.toBeNull());
}

const pageRows = (id: string) => settingsPages().find((page) => page.id === id)?.rows ?? [];

beforeAll(async () => {
  mocked.menuShortcuts.mockResolvedValue([
    { menu: "File", label: "Quit", command: "app:quit", defaultKey: "Mod+Q", key: "Mod+Q" },
    { menu: "Edit", label: "Find…", command: "app:find", defaultKey: "Mod+F", key: null },
  ]);
  registerCoreSettingsPages();
  await vi.waitFor(() => expect(pageRows("shortcuts").map((row) => row.id)).toContain("shortcut:app:quit"));
});

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

afterEach(() => {
  mocked.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
  window.history.replaceState({}, "", "/");
});

describe("the Settings search index", () => {
  it("covers every registered page", () => {
    expect(settingsPages().map((page) => page.id).sort()).toEqual([...PAGE_IDS].sort());
  });

  describe.each(PAGE_IDS)("page %s", (id) => {
    it("has an entry for every anchored row or group it shows, under the same title", async () => {
      await renderPageLoaded(id);

      for (const element of anchored()) {
        const anchor = element.dataset.anchor!;
        const entry = pageRows(id).find((row) => row.id === anchor);
        expect(entry, `page "${id}" shows "${anchor}" but registers no row with that id`).toBeDefined();
        expect(entry?.title, `page "${id}", row "${anchor}": the entry's title differs from the title shown`).toBe(shownTitle(element));
      }
    });

    it("shows every row it registers, except those that wait under another row", async () => {
      await renderPageLoaded(id);

      const shown = new Set(anchored().map((element) => element.dataset.anchor));
      for (const entry of pageRows(id).filter((row) => !row.under)) {
        expect(shown.has(entry.id), `page "${id}" registers "${entry.id}" (${entry.title}) but shows no element with that anchor`).toBe(true);
      }
    });

    it("gives every row an anchor", async () => {
      await renderPageLoaded(id);

      for (const element of document.querySelectorAll<HTMLElement>(".settings-row")) {
        expect(element.dataset.anchor, `page "${id}" shows the row "${shownTitle(element)}" without an anchor`).toBeTruthy();
      }
    });

    it("names an existing row in every `under`", () => {
      const ids = new Set(pageRows(id).map((row) => row.id));
      for (const entry of pageRows(id).filter((row) => row.under)) {
        expect(ids.has(entry.under!), `page "${id}", row "${entry.id}": under "${entry.under}" is not a row of the page`).toBe(true);
      }
    });
  });

  it("shows the rows that wait under another row once the choice that adds them is made", async () => {
    mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, dockerManager: "custom" });

    await renderPageLoaded("general");
    await waitFor(() => expect(anchored().map((element) => element.dataset.anchor)).toContain("dockerStartCommand"));

    for (const entry of pageRows("general").filter((row) => row.under)) {
      const element = anchored().find((candidate) => candidate.dataset.anchor === entry.id);
      expect(element, `page "general" registers "${entry.id}" but a custom Docker app does not show it`).toBeDefined();
      expect(shownTitle(element!), `row "${entry.id}"`).toBe(entry.title);
    }
  });
});
