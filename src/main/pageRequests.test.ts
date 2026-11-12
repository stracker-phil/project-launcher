import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { clearCommands, registerCommands } from "../commands/registry";
import { resetLibrary, useLibrary } from "../state/library";
import { resetRuns } from "../state/runs";
import { resetStore, useStore } from "../state/store";
import { EMPTY_PROJECT_CONFIG } from "../test/apiMock";
import { action, project } from "../test/library";
import { runPageRequest } from "./pageRequests";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const editor = action("Open in Editor");
const build = action("Build site", { command: "npm run build" }, "scripts", 1);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetRuns();
  resetLibrary({ current: alpha });
  clearCommands();
  mocked.projectConfig.mockResolvedValue({ ...EMPTY_PROJECT_CONFIG, apps: [editor], scripts: [build] });
});

describe("runPageRequest", () => {
  describe("action requests", () => {
    it.each([
      ["apps", 0],
      ["scripts", 1],
    ] as const)("launches the current project's action at %s index %i", async (list, index) => {
      await runPageRequest(`action:${list}:${index}`);

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list, index });
    });

    it("launches nothing while no project is current", async () => {
      resetLibrary({ current: null });

      await runPageRequest("action:apps:0");

      expect(mocked.launchAction).not.toHaveBeenCalled();
    });

    it.each(["action:other:0", "action:apps:7"])("launches nothing for %s", async (id) => {
      await runPageRequest(id);

      expect(mocked.launchAction).not.toHaveBeenCalled();
    });

    it("shows an error toast when the project's config cannot be read", async () => {
      mocked.projectConfig.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await runPageRequest("action:apps:0");

      expect(useStore.getState().toasts).toMatchObject([{ kind: "error", message: "Couldn’t run the action: Disk full" }]);
      expect(mocked.launchAction).not.toHaveBeenCalled();
    });
  });

  describe("filter requests", () => {
    it("shows the filter in the library", async () => {
      await runPageRequest('filter:{"kind":"tag","tag":"Client"}');

      expect(useLibrary.getState().filter).toEqual({ kind: "tag", tag: "Client" });
    });

    it("brings the sidebar back when it is hidden", async () => {
      useStore.getState().toggleSidebar();

      await runPageRequest('filter:{"kind":"starred"}');

      expect(useStore.getState().sidebarHidden).toBe(false);
      expect(useLibrary.getState().filter).toEqual({ kind: "starred" });
    });

    it("keeps the filter when the request is not valid JSON", async () => {
      await runPageRequest("filter:{nope");

      expect(useLibrary.getState().filter).toEqual({ kind: "all" });
    });
  });

  describe("tag requests", () => {
    it.each([
      ['tag:"Work"', "Work"],
      ["tag:null", null],
    ])("sends the tag for %s to the current project", async (id, value) => {
      await runPageRequest(id);

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "tag", value });
    });

    it("changes nothing while the project is locked", async () => {
      resetLibrary({ current: project({ id: "alpha", locked: true }) });

      await runPageRequest('tag:"Work"');

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
    });

    it.each(["tag:{nope", "tag:42"])("changes nothing for %s", async (id) => {
      await runPageRequest(id);

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
    });

    it("changes nothing while no project is current", async () => {
      resetLibrary({ current: null });

      await runPageRequest('tag:"Work"');

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
    });
  });

  describe("page commands", () => {
    it("runs the registered command with that id", async () => {
      const run = vi.fn();
      registerCommands([{ id: "view.toggleSidebar", title: "Toggle Sidebar", run }]);

      await runPageRequest("view.toggleSidebar");

      expect(run).toHaveBeenCalled();
    });

    it("changes nothing for an unknown id", async () => {
      await runPageRequest("view.unknown");

      expect(useStore.getState().sidebarHidden).toBe(false);
      expect(mocked.launchAction).not.toHaveBeenCalled();
    });
  });
});
