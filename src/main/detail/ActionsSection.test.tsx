import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ActionSource, type BuiltinApp, type ProjectConfig, type ProjectStatus, type ScriptResult } from "../../api";
import { MenuHost } from "../../components/menu";
import { Toasts } from "../../components/Toasts";
import { resetLibrary, useLibrary } from "../../state/library";
import { resetRuns, useRuns } from "../../state/runs";
import { resetStore } from "../../state/store";
import { EMPTY_PROJECT_CONFIG, EMPTY_STATUS } from "../../test/apiMock";
import { action, project } from "../../test/library";
import { ActionsSection, shownActions } from "./ActionsSection";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const editor = action("Open in Editor", { app: "Demo Editor", shortcut: "Mod+O" });
const terminal = action("Dev Shell", { runsAs: "terminal", command: "npm run dev" }, "apps", 1);const build = action("Build site", { command: "npm run build" }, "scripts", 0);
const lint = action("Lint", { command: "npm run lint" }, "scripts", 1);

const builtins: BuiltinApp[] = [
  { preset: "editor", label: "Editor", icon: "Code", enabled: true, unavailableReason: null },
  { preset: "browser", label: "Browser", icon: "Globe", enabled: false, unavailableReason: null },
  { preset: "git", label: "Git Client", icon: "GitBranch", enabled: false, unavailableReason: "Not a git repository" },
];

function renderSection(patch: Partial<ProjectConfig> = {}, { status = EMPTY_STATUS, editable = true }: { status?: ProjectStatus; editable?: boolean } = {}) {
  const config = { ...EMPTY_PROJECT_CONFIG, apps: [editor, terminal], scripts: [build, lint], builtins, ...patch };
  return render(
    <>
      <ActionsSection project={alpha} config={config} status={status} editable={editable} />
      <MenuHost />
      <Toasts />
    </>,
  );
}

const tile = (name: string | RegExp) => screen.getByRole("button", { name });
const result = (patch: Partial<ScriptResult> = {}): ScriptResult => ({ ok: true, exitCode: 0, timedOut: false, output: "all good", durationMs: 1500, ...patch });
const source = (list: ActionSource["list"], index: number): ActionSource => ({ list, index });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetRuns();
  resetLibrary();
});

describe("shownActions", () => {
  it("drops the actions the status hides", () => {
    expect(shownActions([editor, terminal, build], { ...EMPTY_STATUS, hidden: [source("apps", 1)] })).toEqual([editor, build]);
  });

  it("keeps every action without a status", () => {
    expect(shownActions([editor, terminal], undefined)).toEqual([editor, terminal]);
  });
});

describe("ActionsSection", () => {
  describe("the Open grid", () => {
    it("shows a tile per action with what it opens and its shortcut", () => {
      renderSection({ apps: [editor, terminal, action("Website", { runsAs: "url", url: "https://example.com/shop" }, "apps", 2)] });

      expect(tile(/Open in Editor/)).toHaveTextContent("Demo Editor");
      expect(within(tile(/Open in Editor/)).getByLabelText("⌘O")).toBeInTheDocument();
      expect(within(tile(/Dev Shell/)).queryByLabelText(/⌘/)).not.toBeInTheDocument();
      expect(tile(/Dev Shell/)).toHaveTextContent("Terminal · npm run dev");
      expect(tile(/Website/)).toHaveTextContent("example.com");
    });

    it("launches the action's source in the project when its tile is clicked", async () => {
      renderSection();

      await userEvent.click(tile(/Dev Shell/));

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", source("apps", 1));
    });

    it("leaves out the actions the status hides", () => {
      renderSection({}, { status: { ...EMPTY_STATUS, hidden: [source("apps", 1), source("scripts", 1)] } });

      expect(screen.queryByRole("button", { name: /Dev Shell/ })).not.toBeInTheDocument();
      expect(screen.queryByText("Lint")).not.toBeInTheDocument();
      expect(tile(/Open in Editor/)).toBeInTheDocument();
      expect(screen.getByText("Build site")).toBeInTheDocument();
    });

    it("marks the tile of a running terminal as Running", () => {
      renderSection({}, { status: { ...EMPTY_STATUS, running: [source("apps", 1)] } });

      expect(within(tile(/Dev Shell/)).getByLabelText("Running")).toBeInTheDocument();
      expect(within(tile(/Open in Editor/)).queryByLabelText("Running")).not.toBeInTheDocument();
    });

    it.each([
      { editable: true, text: "Nothing to open yet: add an action or a built-in app with +." },
      { editable: false, text: "Nothing to open." },
    ])("says '$text' without apps when editable is $editable", ({ editable, text }) => {
      renderSection({ apps: [] }, { editable });

      expect(screen.getByText(text)).toBeInTheDocument();
    });
  });

  describe("scripts", () => {
    it("lists the scripts with their commands and a Run button each", () => {
      renderSection();

      expect(screen.getByRole("heading", { name: "Scripts" })).toBeInTheDocument();
      expect(screen.getByText("npm run build")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Run Lint" })).toBeEnabled();
    });

    it("labels each Run button with the visible text Run", () => {
      renderSection();

      expect(screen.getByRole("button", { name: "Run Lint" })).toHaveTextContent("Run");
    });

    it("shows a script's shortcut as keycaps", () => {
      renderSection({ scripts: [action("Build site", { command: "npm run build", shortcut: "Mod+Shift+B" }, "scripts", 0)] });

      expect(screen.getByLabelText("⇧⌘B")).toBeInTheDocument();
    });

    it("runs the script and shows its output under Last Run", async () => {
      mocked.launchAction.mockResolvedValueOnce({ kind: "ran", result: result({ output: "built 12 pages" }) });
      renderSection();

      await userEvent.click(screen.getByRole("button", { name: "Run Build site" }));

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", source("scripts", 0));
      const region = await screen.findByRole("region", { name: "Last Run" });
      expect(region).toHaveTextContent("Build site");
      expect(region).toHaveTextContent("Finished · 1.5 s");
      expect(region).toHaveTextContent("built 12 pages");
    });

    it("disables the Run button of a script while it runs", () => {
      useRuns.getState().start("alpha", source("scripts", 0));

      renderSection();

      expect(screen.getByRole("button", { name: "Build site is running" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Build site is running" })).toHaveTextContent("Running");
      expect(screen.queryByRole("button", { name: "Run Build site" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Run Lint" })).toBeEnabled();
    });
  });

  describe("Last Run", () => {
    it.each([
      { why: "a failure with its exit code", patch: { ok: false, exitCode: 2 }, outcome: "Failed (exit 2) · 1.5 s" },
      { why: "a failure without an exit code", patch: { ok: false, exitCode: null }, outcome: "Failed · 1.5 s" },
      { why: "a timeout", patch: { ok: false, exitCode: null, timedOut: true }, outcome: "Stopped · 1.5 s" },
    ])("shows $why", ({ patch, outcome }) => {
      useRuns.getState().finish("alpha", source("scripts", 0), { label: "Build site", result: result(patch) });

      renderSection();

      expect(screen.getByRole("region", { name: "Last Run" })).toHaveTextContent(outcome);
    });

    it("says there was no output for a silent script", () => {
      useRuns.getState().finish("alpha", source("scripts", 0), { label: "Build site", result: result({ output: "" }) });

      renderSection();

      expect(screen.getByText("No output.")).toBeInTheDocument();
    });

    it("is gone after Close Output", async () => {
      useRuns.getState().finish("alpha", source("scripts", 0), { label: "Build site", result: result() });
      renderSection();

      await userEvent.click(screen.getByRole("button", { name: "Close Output" }));

      expect(screen.queryByRole("region", { name: "Last Run" })).not.toBeInTheDocument();
    });

    it("shows another project's run nowhere", () => {
      useRuns.getState().finish("beta", source("scripts", 0), { label: "Build site", result: result() });

      renderSection();

      expect(screen.queryByRole("region", { name: "Last Run" })).not.toBeInTheDocument();
    });

    it("stays under Scripts for a project whose script is gone", () => {
      useRuns.getState().finish("alpha", source("scripts", 0), { label: "Old script", result: result() });

      renderSection({ scripts: [] });

      expect(screen.getByRole("heading", { name: "Scripts" })).toBeInTheDocument();
      expect(screen.getByRole("region", { name: "Last Run" })).toHaveTextContent("Old script");
    });
  });

  describe("Add Action", () => {
    it("is offered only while editable", () => {
      renderSection({}, { editable: false });

      expect(screen.queryByRole("button", { name: "Add Action" })).not.toBeInTheDocument();
    });

    it("opens a menu with New Action…, New Script… and the built-ins as toggles", async () => {
      renderSection();

      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      const menu = screen.getByRole("menu", { name: "Add Action" });
      expect(within(menu).getByRole("menuitem", { name: "New Action…" })).toBeInTheDocument();
      expect(within(menu).getByRole("menuitem", { name: "New Script…" })).toBeInTheDocument();
      const group = within(menu).getByRole("group", { name: "Built-in Apps" });
      expect(within(group).getByRole("menuitemcheckbox", { name: "Editor" })).toHaveAttribute("aria-checked", "true");
      expect(within(group).getByRole("menuitemcheckbox", { name: "Browser" })).toHaveAttribute("aria-checked", "false");
    });

    it("disables a built-in that is unavailable and shows why", async () => {
      renderSection();

      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      const git = screen.getByRole("menuitemcheckbox", { name: /Git Client/ });
      expect(git).toHaveAttribute("aria-disabled", "true");
      expect(git).toHaveTextContent("Not a git repository");
    });

    it("enables a built-in through api.setBuiltinEnabled", async () => {
      renderSection();
      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      await userEvent.click(screen.getByRole("menuitemcheckbox", { name: "Browser" }));

      expect(mocked.setBuiltinEnabled).toHaveBeenCalledWith("alpha", "browser", true);
    });

    it("disables an enabled built-in the same way", async () => {
      renderSection();
      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      await userEvent.click(screen.getByRole("menuitemcheckbox", { name: "Editor" }));

      expect(mocked.setBuiltinEnabled).toHaveBeenCalledWith("alpha", "editor", false);
    });

    it("opens the editor for a new action from New Action…", async () => {
      renderSection();
      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      await userEvent.click(screen.getByRole("menuitem", { name: "New Action…" }));

      const dialog = await screen.findByRole("dialog", { name: "New Action" });
      expect(within(dialog).getByRole("button", { name: "App" })).toHaveAttribute("aria-pressed", "true");
    });

    it("opens the editor for a new script from New Script…", async () => {
      renderSection();
      await userEvent.click(screen.getByRole("button", { name: "Add Action" }));

      await userEvent.click(screen.getByRole("menuitem", { name: "New Script…" }));

      const dialog = await screen.findByRole("dialog", { name: "New Action" });
      expect(within(dialog).getByRole("button", { name: "Script" })).toHaveAttribute("aria-pressed", "true");
    });
  });

  describe("a new-action request from the inspector", () => {
    const request = () => act(() => useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "dev", command: "npm run dev" }));

    it("opens the editor with the draft's title and command prefilled", async () => {
      renderSection();

      request();

      const dialog = await screen.findByRole("dialog", { name: "New Action" });
      expect(within(dialog).getByRole("textbox", { name: "Title" })).toHaveValue("dev");
      expect(within(dialog).getByRole("textbox", { name: "Command" })).toHaveValue("npm run dev");
      expect(within(dialog).getByRole("button", { name: "Terminal" })).toHaveAttribute("aria-pressed", "true");
    });

    it("is taken once the editor is open", async () => {
      renderSection();

      request();

      await screen.findByRole("dialog", { name: "New Action" });
      expect(useLibrary.getState().newAction).toBeNull();
    });

    it("does not reopen the editor when the section is mounted again", async () => {
      const { unmount } = renderSection();
      request();
      await userEvent.click(await within(await screen.findByRole("dialog", { name: "New Action" })).findByRole("button", { name: "Cancel" }));
      unmount();

      renderSection();

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("opens nothing while the section is not editable, and keeps the request", () => {
      renderSection({}, { editable: false });

      request();

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(useLibrary.getState().newAction?.draft.label).toBe("dev");
    });
  });

  describe("an action's context menu", () => {
    const openMenu = (name: string | RegExp) => fireEvent.contextMenu(tile(name));

    it("offers Edit…, Duplicate, Move Up, Move Down and Remove", () => {
      renderSection();

      openMenu(/Open in Editor/);

      const menu = screen.getByRole("menu", { name: "Open in Editor" });
      expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Edit…", "Duplicate", "Move Up", "Move Down", "Remove"]);
    });

    it("opens the editor on the action from Edit…", async () => {
      renderSection();
      openMenu(/Open in Editor/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Edit…" }));

      const dialog = await screen.findByRole("dialog", { name: "Edit Action" });
      expect(within(dialog).getByRole("textbox", { name: "Title" })).toHaveValue("Open in Editor");
    });

    it("removes the action through api.removeAction", async () => {
      renderSection();
      openMenu(/Dev Shell/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Remove" }));

      expect(mocked.removeAction).toHaveBeenCalledWith("alpha", source("apps", 1));
    });

    it("duplicates the action as it is from Duplicate", async () => {
      renderSection();
      openMenu(/Open in Editor/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));

      expect(mocked.duplicateAction).toHaveBeenCalledWith("alpha", source("apps", 0), null);
    });

    it("toasts the error when duplicating fails", async () => {
      mocked.duplicateAction.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderSection();
      openMenu(/Open in Editor/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t duplicate the action: Disk full");
    });

    it("moves an action to the place of its neighbor", async () => {
      renderSection();
      openMenu(/Dev Shell/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Move Up" }));

      expect(mocked.moveAction).toHaveBeenCalledWith("alpha", source("apps", 1), 0);
    });

    it("disables Move Up on the first action and Move Down on the last", () => {
      renderSection();

      openMenu(/Open in Editor/);
      expect(screen.getByRole("menuitem", { name: "Move Up" })).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByRole("menuitem", { name: "Move Down" })).not.toHaveAttribute("aria-disabled");
    });

    it("offers Duplicate as Terminal Command on a script and works on script rows", async () => {
      renderSection();

      fireEvent.contextMenu(screen.getByText("Build site"));

      expect(screen.getByRole("menuitem", { name: "Duplicate as Terminal Command" })).toBeInTheDocument();
      await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate as Terminal Command" }));
      expect(mocked.duplicateAction).toHaveBeenCalledWith("alpha", source("scripts", 0), "terminal");
    });

    it("duplicates a terminal command as a script", async () => {
      renderSection();
      openMenu(/Dev Shell/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate as Script" }));

      expect(mocked.duplicateAction).toHaveBeenCalledWith("alpha", source("apps", 1), "script");
    });

    it("offers no Duplicate as Script for a terminal without a command", () => {
      renderSection({ apps: [editor, action("Shell", { runsAs: "terminal" }, "apps", 1)] });

      openMenu(/Shell/);

      expect(screen.queryByRole("menuitem", { name: "Duplicate as Script" })).not.toBeInTheDocument();
    });

    it("toasts the error when removing fails", async () => {
      mocked.removeAction.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderSection();
      openMenu(/Dev Shell/);

      await userEvent.click(screen.getByRole("menuitem", { name: "Remove" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t remove “Dev Shell”: Disk full");
    });

    it("opens no menu when the section is not editable", () => {
      renderSection({}, { editable: false });

      openMenu(/Open in Editor/);

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });
});
