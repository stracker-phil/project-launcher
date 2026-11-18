import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Outcome, type ScriptResult } from "../api";
import { Toasts } from "../components/Toasts";
import { resetRuns, useRuns } from "../state/runs";
import { resetStore } from "../state/store";
import { action, project } from "../test/library";
import { copyPath, editProjectFile, launchAction, openClaudeManager, openTerminal, revealProject } from "./launch";

const mocked = vi.mocked(api, true);
const diskFull = { kind: "io", message: "Disk full", code: null };

const alpha = project({ id: "alpha", name: "Alpha" });
const build = action("Build site", { command: "npm run build" }, "scripts", 2);
const editor = action("Open in Editor", { app: "Demo Editor" });

const ran = (patch: Partial<ScriptResult>): Outcome => ({
  kind: "ran",
  result: { ok: true, exitCode: 0, timedOut: false, output: "done", durationMs: 2400, ...patch },
});

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetRuns();
  render(<Toasts />);
});

describe("launchAction", () => {
  it("launches the action's source in the project", async () => {
    await launchAction(alpha, editor);

    expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 });
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("toasts how long a script took and keeps its output as the project's last run", async () => {
    mocked.launchAction.mockResolvedValueOnce(ran({ durationMs: 2400, output: "built 12 pages" }));

    await launchAction(alpha, build);

    expect(await screen.findByRole("status")).toHaveTextContent("Build site finished in 2.4 s");
    const state = useRuns.getState();
    expect(state.running.alpha).toEqual([]);
    expect(state.last.alpha).toMatchObject({ label: "Build site", result: { output: "built 12 pages" } });
  });

  it("rounds a long script's duration to whole seconds", async () => {
    mocked.launchAction.mockResolvedValueOnce(ran({ durationMs: 42_000 }));

    await launchAction(alpha, build);

    expect(await screen.findByRole("status")).toHaveTextContent("Build site finished in 42 s");
  });

  it("marks the script as running until its result arrives", async () => {
    let finish: (outcome: Outcome) => void = () => {};
    mocked.launchAction.mockReturnValueOnce(new Promise<Outcome>((resolve) => (finish = resolve)));

    const done = launchAction(alpha, build);

    expect(useRuns.getState().running.alpha).toEqual(["scripts:2"]);
    finish(ran({}));
    await done;
    expect(useRuns.getState().running.alpha).toEqual([]);
  });

  it.each([
    { why: "with its exit code", patch: { ok: false, exitCode: 2 }, message: "Build site failed with exit code 2" },
    { why: "without an exit code when it was killed", patch: { ok: false, exitCode: null }, message: "Build site failed" },
    { why: "as stopped when it timed out", patch: { ok: false, exitCode: null, timedOut: true, durationMs: 30_000 }, message: "Build site was stopped after 30 s" },
  ])("toasts a failed script $why", async ({ patch, message }) => {
    mocked.launchAction.mockResolvedValueOnce(ran(patch));

    await launchAction(alpha, build);

    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(useRuns.getState().last.alpha.result.ok).toBe(false);
  });

  it("explains the Automation permission when bringing a terminal tab forward needs it", async () => {
    mocked.launchAction.mockResolvedValueOnce({ kind: "opened", how: "needsPermission" });

    await launchAction(alpha, action("Terminal", { runsAs: "terminal" }, "apps", 1));

    expect(await screen.findByRole("status")).toHaveTextContent("allow Launcher to control your terminal in System Settings → Privacy & Security → Automation");
  });

  it("toasts the error and clears the running mark when launching fails", async () => {
    mocked.launchAction.mockRejectedValueOnce(diskFull);

    await launchAction(alpha, build);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t run “Build site”: Disk full");
    expect(useRuns.getState().running.alpha).toEqual([]);
    expect(useRuns.getState().last.alpha).toBeUndefined();
  });

  it("does not mark an app as running a script", async () => {
    mocked.launchAction.mockRejectedValueOnce(diskFull);

    await launchAction(alpha, editor);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t run “Open in Editor”");
    expect(useRuns.getState().running).toEqual({});
  });
});

describe("project commands", () => {
  it("openTerminal opens a terminal for the project", async () => {
    await openTerminal(alpha);

    expect(mocked.openTerminal).toHaveBeenCalledWith("alpha");
  });

  it("revealProject shows the project in Finder", async () => {
    await revealProject(alpha);

    expect(mocked.revealProject).toHaveBeenCalledWith("alpha");
  });

  it("editProjectFile opens the project's file", async () => {
    await editProjectFile(alpha);

    expect(mocked.editProjectFile).toHaveBeenCalledWith("alpha");
  });

  it("openClaudeManager opens the project in Claude Manager", async () => {
    await openClaudeManager(alpha);

    expect(mocked.openClaudeManager).toHaveBeenCalledWith("alpha");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("copyPath toasts once the path is copied", async () => {
    await copyPath(alpha);

    expect(mocked.copyProjectPath).toHaveBeenCalledWith("alpha");
    expect(await screen.findByRole("status")).toHaveTextContent("Copied the project’s path");
  });

  it.each([
    { name: "openTerminal", run: openTerminal, message: "Couldn’t open a terminal: Disk full", rejects: () => mocked.openTerminal },
    { name: "revealProject", run: revealProject, message: "Couldn’t show the folder in Finder: Disk full", rejects: () => mocked.revealProject },
    { name: "editProjectFile", run: editProjectFile, message: "Couldn’t open the project file: Disk full", rejects: () => mocked.editProjectFile },
    { name: "openClaudeManager", run: openClaudeManager, message: "Couldn’t open Claude Manager: Disk full", rejects: () => mocked.openClaudeManager },
    { name: "copyPath", run: copyPath, message: "Couldn’t copy the path: Disk full", rejects: () => mocked.copyProjectPath },
  ])("$name toasts the error when it fails", async ({ run, message, rejects }) => {
    rejects().mockRejectedValueOnce(diskFull);

    await run(alpha);

    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(screen.queryByText("Copied the project’s path")).not.toBeInTheDocument();
  });
});
