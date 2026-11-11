import { renderHook } from "@testing-library/react";
import { Settings } from "lucide-react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuCommand } from "../api";
import { clearCommands, registerCommands, type PageCommand } from "../commands/registry";
import type { MenuEntry } from "../components/menu";
import { registerSettingsPage } from "../registry/settingsPages";
import { resetStore, useStore } from "../state/store";
import { usePaletteMenu } from "./menus";
import { collectCommands, snapshotPage, type PaletteCommand } from "./sources";

const mocked = vi.mocked(api, true);

const menuRow = (patch: Partial<MenuCommand> = {}): MenuCommand => ({ menu: "File", label: "New Project", command: "app:new", key: null, ...patch });
const pageCommand = (patch: Partial<PageCommand> = {}): PageCommand => ({ id: "demo.go", title: "Go Demo", run: vi.fn(), ...patch });

const collect = (menu: MenuCommand[]) => collectCommands(menu, snapshotPage());
const find = (commands: PaletteCommand[], id: string) => commands.find((c) => c.id === id);

describe("collectCommands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
    clearCommands();
    mocked.runCommand.mockResolvedValue(undefined);
  });

  describe("bus items", () => {
    it("lists a menu item under its menu with the key Rust shows", () => {
      const found = find(collect([menuRow({ key: "⌘N" })]), "app:new");

      expect(found).toMatchObject({ label: "New Project", section: "File", shortcut: "⌘N" });
    });

    it("runs the bus command by id", () => {
      find(collect([menuRow()]), "app:new")?.run();

      expect(mocked.runCommand).toHaveBeenCalledWith("app:new");
    });

    it("shows an error toast with the failure when the command fails", async () => {
      mocked.runCommand.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      find(collect([menuRow()]), "app:new")?.run();

      await vi.waitFor(() => expect(useStore.getState().toasts).toMatchObject([{ kind: "error", message: "Could not run “New Project”: Disk full" }]));
    });
  });

  describe("page items", () => {
    const pageRow = menuRow({ menu: "View", label: "Go", command: "page:demo.go", key: "⌘G" });

    it("runs the page command", () => {
      const run = vi.fn();
      registerCommands([pageCommand({ run })]);

      find(collect([pageRow]), "page:demo.go")?.run();

      expect(run).toHaveBeenCalled();
      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("shows the command's own key label before the one Rust reports", () => {
      registerCommands([pageCommand({ defaultBinding: "Mod+Shift+G" })]);

      expect(find(collect([pageRow]), "page:demo.go")?.shortcut).toBe("⌘⇧G");
    });

    it("falls back to the key Rust reports when the command has no binding", () => {
      registerCommands([pageCommand()]);

      expect(find(collect([pageRow]), "page:demo.go")?.shortcut).toBe("⌘G");
    });

    it("is left out when no such command is registered", () => {
      expect(find(collect([pageRow]), "page:demo.go")).toBeUndefined();
    });

    it("is left out when the command cannot run right now", () => {
      registerCommands([pageCommand({ when: () => false })]);

      expect(find(collect([pageRow]), "page:demo.go")).toBeUndefined();
    });
  });

  describe("page commands outside the menus", () => {
    it("lists a titled command under Window with its key", () => {
      registerCommands([pageCommand({ id: "view.toggleSidebar", title: "Toggle Sidebar", defaultBinding: "Mod+B" })]);

      expect(find(collect([]), "page:view.toggleSidebar")).toMatchObject({ label: "Toggle Sidebar", section: "Window", shortcut: "⌘B" });
    });

    it("runs through the page command", () => {
      const run = vi.fn();
      registerCommands([pageCommand({ run })]);

      find(collect([]), "page:demo.go")?.run();

      expect(run).toHaveBeenCalled();
    });

    it("is not listed twice when a menu item already runs it", () => {
      registerCommands([pageCommand()]);

      const found = collect([menuRow({ menu: "View", label: "Go", command: "page:demo.go" })]).filter((c) => c.id === "page:demo.go");

      expect(found).toMatchObject([{ section: "View", label: "Go" }]);
    });

    it("is left out when it cannot run right now", () => {
      registerCommands([pageCommand({ when: () => false })]);

      expect(find(collect([]), "page:demo.go")).toBeUndefined();
    });

    it("is left out without a title", () => {
      registerCommands([pageCommand({ title: "" })]);

      expect(find(collect([]), "page:demo.go")).toBeUndefined();
    });

    it("never lists the palette's own command", () => {
      registerCommands([pageCommand({ id: "palette.open", title: "Command Palette" })]);

      const all = collect([menuRow({ menu: "View", label: "Palette", command: "page:palette.open" })]);

      expect(all.some((c) => c.label === "Command Palette" || c.id.includes("palette.open"))).toBe(false);
    });
  });

  describe("Settings pages", () => {
    registerSettingsPage({
      id: "palette-test-shortcuts",
      title: "Shortcuts",
      icon: Settings,
      order: 1,
      keywords: ["keys", "bindings"],
      rows: [{ id: "reset", title: "Reset all shortcuts" }],
      component: () => null,
    });
    registerSettingsPage({ id: "palette-test-plain", title: "Plain", icon: Settings, order: 2, component: () => null });

    it("lists each page as 'Settings: <title>' with its keywords and row titles as detail", () => {
      expect(find(collect([]), "app:settings:palette-test-shortcuts")).toMatchObject({
        label: "Settings: Shortcuts",
        section: "Settings",
        detail: "keys, bindings, Reset all shortcuts",
      });
    });

    it("has no detail for a page without keywords or rows", () => {
      expect(find(collect([]), "app:settings:palette-test-plain")?.detail).toBeUndefined();
    });

    it("opens the page through the bus command with its id", () => {
      find(collect([]), "app:settings:palette-test-shortcuts")?.run();

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:palette-test-shortcuts");
    });
  });

  describe("menus the page offers", () => {
    const rename = vi.fn();
    const moveToClients = vi.fn();
    const entries = (): MenuEntry[] => [
      { caption: "Edit" },
      { text: "Rename", hint: "F2", shortcut: "⌘R", run: rename },
      { text: "Archive", enabled: false, run: vi.fn() },
      "separator",
      { text: "Move to Tag", items: [{ text: "Clients", checked: true, run: moveToClients }, { text: "Private", enabled: false, run: vi.fn() }] },
      { text: "No Action" },
    ];

    it("flattens items and submenus into rows labelled with the menu's title", () => {
      renderHook(() => usePaletteMenu("test-menu", { section: "Project", title: "Project", entries }));

      const rows = collect([]).filter((c) => c.section === "Project");

      expect(rows.map((c) => c.label)).toEqual(["Project: Rename", "Project: Move to Tag › Clients"]);
    });

    it("carries the hint, shortcut and checked state of an item and runs it", () => {
      renderHook(() => usePaletteMenu("test-menu", { section: "Project", title: "Project", entries }));
      const rows = collect([]).filter((c) => c.section === "Project");

      expect(rows[0]).toMatchObject({ detail: "F2", shortcut: "⌘R" });
      expect(rows[1]).toMatchObject({ checked: true });
      rows[1].run();
      expect(moveToClients).toHaveBeenCalled();
    });

    it("stops listing a menu once it is unmounted", () => {
      const { unmount } = renderHook(() => usePaletteMenu("test-menu", { section: "Project", title: "Project", entries }));
      unmount();

      expect(collect([]).some((c) => c.section === "Project")).toBe(false);
    });

    it("lists nothing for a menu that is null", () => {
      renderHook(() => usePaletteMenu("test-menu", null));

      expect(collect([]).some((c) => c.section === "Project")).toBe(false);
    });
  });
});
