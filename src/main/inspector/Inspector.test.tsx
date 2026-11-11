import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type PackageScript, type Project } from "../../api";
import { MenuHost } from "../../components/menu";
import { Toasts } from "../../components/Toasts";
import { createQueryClient } from "../../queries/queryClient";
import { resetLibrary, useLibrary } from "../../state/library";
import { resetStore } from "../../state/store";
import { EMPTY_FILE_STATS, EMPTY_QUICK_STATS } from "../../test/apiMock";
import { project } from "../../test/library";
import { fileStats, gitStats, quickStats } from "../../test/stats";
import { Inspector } from "./Inspector";

const mocked = vi.mocked(api, true);

const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const alpha = project({ id: "alpha", name: "Alpha" });

function renderInspector(shown: Project | null = alpha) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <Inspector project={shown} />
      <MenuHost />
      <Toasts />
    </QueryClientProvider>,
  );
}

/** The row of a label inside a section: label and value share one parent. */
const row = (label: string) => screen.getByText(label).parentElement as HTMLElement;

const devScript: PackageScript = { manager: "npm", name: "dev", script: "vite", command: "npm run dev" };
const buildScript: PackageScript = { manager: "pnpm", name: "build-site", script: "vite build", command: "pnpm run build-site" };

beforeEach(() => {
  vi.clearAllMocks();
  mocked.quickStats.mockResolvedValue(EMPTY_QUICK_STATS);
  mocked.fileStats.mockResolvedValue(EMPTY_FILE_STATS);
  resetStore();
  resetLibrary();
});

describe("Inspector", () => {
  describe("without details to show", () => {
    it("says no project is selected", () => {
      renderInspector(null);

      expect(screen.getByText("No Project Selected")).toBeInTheDocument();
      expect(mocked.quickStats).not.toHaveBeenCalled();
    });

    it("shows the file error of a project that is unavailable and reads nothing", () => {
      renderInspector(project({ available: false, fileError: "The project file is missing" }));

      expect(screen.getByText("The project file is missing")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Refresh" })).not.toBeInTheDocument();
      expect(mocked.quickStats).not.toHaveBeenCalled();
      expect(mocked.fileStats).not.toHaveBeenCalled();
    });
  });

  describe("Overview", () => {
    it("shows the project's kind and its requirements joined by dots", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ kind: "Node project", requirements: ["Node 20", "pnpm 9"] }));
      renderInspector();

      expect(await screen.findByText("Node project")).toBeInTheDocument();
      expect(screen.getByText("Node 20 · pnpm 9")).toBeInTheDocument();
    });

    it("shows files, sizes and lines of the scan", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 1234, sourceBytes: 1536, diskBytes: 3 * 1024 * 1024, lines: 56789 }));
      renderInspector();

      await waitFor(() => expect(row("Files")).toHaveTextContent(/1\D?234/));
      expect(row("Source size")).toHaveTextContent("1.5");
      expect(row("On disk")).toHaveTextContent("3");
      expect(row("Lines of code")).toHaveTextContent(/56\D?789/);
    });

    it("marks the file count with a plus when the scan was cut short", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 50000, truncated: true }));
      renderInspector();

      await waitFor(() => expect(row("Files")).toHaveTextContent(/50\D?000\+/));
    });

    it("leaves out the plus when the scan was complete", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 40 }));
      renderInspector();

      await waitFor(() => expect(row("Files")).toHaveTextContent("40"));
      expect(row("Files")).not.toHaveTextContent("+");
    });

    it("shows a dash for a disk usage that could not be read", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 3, diskBytes: null }));
      renderInspector();

      await waitFor(() => expect(row("On disk")).toHaveTextContent("–"));
    });

    it("shows when the newest file was edited, with its path as the title", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 2, recent: [{ path: "src/app.ts", atMs: Date.now() - 3 * DAY }, { path: "README.md", atMs: Date.now() - 9 * DAY }] }));
      renderInspector();

      await waitFor(() => expect(row("Last edited")).toHaveTextContent("3 days ago"));
      expect(within(row("Last edited")).getByTitle("src/app.ts")).toBeInTheDocument();
    });

    it("shows a dash for Last edited when no file was found", async () => {
      renderInspector();

      await waitFor(() => expect(row("Last edited")).toHaveTextContent("–"));
    });

    it.each(["Files", "Source size", "On disk", "Lines of code", "Last edited"])("shows an ellipsis for %s while the file scan reads", (label) => {
      mocked.fileStats.mockReturnValue(new Promise(() => {}));
      renderInspector();

      expect(row(label)).toHaveTextContent("…");
    });
  });

  describe("Languages", () => {
    it("lists each language with its share and size", async () => {
      mocked.fileStats.mockResolvedValue(
        fileStats({
          files: 14,
          languages: [
            { name: "TypeScript", color: "Blue", files: 10, lines: 900, bytes: 7500 },
            { name: "CSS", color: null, files: 4, lines: 100, bytes: 2500 },
          ],
        }),
      );
      renderInspector();

      const languages = await screen.findByRole("region", { name: "Languages" });
      expect(within(languages).getByText("TypeScript")).toBeInTheDocument();
      expect(within(languages).getByText("CSS")).toBeInTheDocument();
      expect(within(languages).getByText("75%")).toBeInTheDocument();
      expect(within(languages).getByText("25%")).toBeInTheDocument();
      expect(within(languages).getByText("10 files · 900 lines")).toBeInTheDocument();
      expect(within(languages).getByRole("img", { name: "TypeScript, CSS" })).toBeInTheDocument();
    });

    it("is left out when no language was found", async () => {
      renderInspector();
      await waitFor(() => expect(row("Last edited")).toHaveTextContent("–"));

      expect(screen.queryByRole("region", { name: "Languages" })).not.toBeInTheDocument();
    });
  });

  describe("Git", () => {
    it("is left out for a project without git", async () => {
      renderInspector();
      await waitFor(() => expect(row("Last edited")).toHaveTextContent("–"));

      expect(screen.queryByRole("region", { name: "Git" })).not.toBeInTheDocument();
    });

    it("shows branch, last commit, commit count and latest tag", async () => {
      mocked.quickStats.mockResolvedValue(
        quickStats({
          git: gitStats({
            branch: "feature/search",
            lastCommit: { subject: "Fix the search", author: "Sam Example", atMs: Date.now() - 2 * DAY },
            commits: 1200,
            firstCommitMs: Date.now() - 800 * DAY,
            latestTag: "v1.4.0",
          }),
        }),
      );
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Branch")).toHaveTextContent("feature/search");
      expect(row("Last commit")).toHaveTextContent("Fix the search · 2 days ago");
      expect(within(row("Last commit")).getByTitle("Sam Example")).toBeInTheDocument();
      expect(row("Commits")).toHaveTextContent(/^Commits1\D?200$/);
      expect(row("First commit")).toHaveTextContent("2 years ago");
      expect(row("Latest tag")).toHaveTextContent("v1.4.0");
    });

    it("leaves out First commit when there is none", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ firstCommitMs: null }) }));
      renderInspector();

      const git = await screen.findByRole("region", { name: "Git" });
      expect(within(git).queryByText("First commit")).not.toBeInTheDocument();
    });

    it("says Detached without a branch", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ branch: null }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Branch")).toHaveTextContent("Detached");
    });

    it.each([
      { why: "nothing changed", modified: 0, untracked: 0, text: "None" },
      { why: "files changed and are new", modified: 2, untracked: 1, text: "2 modified, 1 untracked" },
    ])("shows $text for Changes when $why", async ({ modified, untracked, text }) => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ modified, untracked }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Changes")).toHaveTextContent(text);
    });

    it.each([
      { why: "ahead and behind", ahead: 2, behind: 1, text: "2 ahead, 1 behind" },
      { why: "level", ahead: 0, behind: 0, text: "Up to date" },
    ])("shows $text for Upstream when the branch is $why", async ({ ahead, behind, text }) => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ hasUpstream: true, ahead, behind }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Upstream")).toHaveTextContent(text);
    });

    it("leaves out Upstream, Last commit and Latest tag when there are none", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ hasUpstream: false, ahead: 3, lastCommit: null, latestTag: null }) }));
      renderInspector();

      const git = await screen.findByRole("region", { name: "Git" });
      expect(within(git).queryByText("Upstream")).not.toBeInTheDocument();
      expect(within(git).queryByText("Last commit")).not.toBeInTheDocument();
      expect(within(git).queryByText("Latest tag")).not.toBeInTheDocument();
    });

    it("summarizes the weekly commits as an image", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ weekly: [1, 0, 4] }) }));
      renderInspector();

      expect(await screen.findByRole("img", { name: "5 commits in 3 weeks" })).toBeInTheDocument();
    });
  });

  describe("Recently Edited", () => {
    const recent = [
      { path: "src/app.ts", atMs: Date.now() - 3 * HOUR },
      { path: "docs/guide.md", atMs: Date.now() - 2 * DAY },
    ];

    it("lists the files by name with when they changed", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 2, recent }));
      renderInspector();

      const list = await screen.findByRole("region", { name: "Recently Edited" });
      expect(within(list).getByRole("button", { name: /app\.ts/ })).toHaveTextContent("3 hours ago");
      expect(within(list).getByRole("button", { name: /guide\.md/ })).toHaveTextContent("2 days ago");
    });

    it("opens the file through api.openProjectFile when its button is clicked", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 2, recent }));
      renderInspector();

      await userEvent.click(await screen.findByRole("button", { name: /guide\.md/ }));

      expect(mocked.openProjectFile).toHaveBeenCalledWith("alpha", "docs/guide.md");
    });

    it("toasts the error when the file cannot be opened", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 2, recent }));
      mocked.openProjectFile.mockRejectedValue({ kind: "not_found", message: "File is gone", code: null });
      renderInspector();

      await userEvent.click(await screen.findByRole("button", { name: /app\.ts/ }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t open the file: File is gone");
    });

    it("is left out without recent files", async () => {
      renderInspector();
      await waitFor(() => expect(row("Last edited")).toHaveTextContent("–"));

      expect(screen.queryByRole("region", { name: "Recently Edited" })).not.toBeInTheDocument();
    });
  });

  describe("Package Scripts", () => {
    beforeEach(() => {
      mocked.quickStats.mockResolvedValue(quickStats({ scripts: [devScript, buildScript] }));
    });

    it("lists each script with its manager", async () => {
      renderInspector();

      const scripts = await screen.findByRole("region", { name: "Package Scripts" });
      expect(within(scripts).getByRole("button", { name: /^dev/ })).toHaveTextContent("npm");
      expect(within(scripts).getByRole("button", { name: /^build-site/ })).toHaveTextContent("pnpm");
    });

    it("is left out without scripts", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ kind: "Node project" }));
      renderInspector();
      await screen.findByText("Node project");

      expect(screen.queryByRole("region", { name: "Package Scripts" })).not.toBeInTheDocument();
    });

    it("runs the script with its manager through api.runPackageScript when clicked", async () => {
      renderInspector();

      await userEvent.click(await screen.findByRole("button", { name: /^build-site/ }));

      expect(mocked.runPackageScript).toHaveBeenCalledWith("alpha", "pnpm", "build-site");
    });

    it("toasts the error when the script cannot run", async () => {
      mocked.runPackageScript.mockRejectedValue({ kind: "not_found", message: "pnpm is not installed", code: null });
      renderInspector();

      await userEvent.click(await screen.findByRole("button", { name: /^build-site/ }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t run build-site: pnpm is not installed");
    });

    it("requests a terminal action with the script's command from Add as Action…", async () => {
      renderInspector();
      fireEvent.contextMenu(await screen.findByRole("button", { name: /^dev/ }));

      await userEvent.click(screen.getByRole("menuitem", { name: "Add as Action…" }));

      expect(useLibrary.getState().newAction?.draft).toEqual({ runsAs: "terminal", label: "dev", command: "npm run dev" });
    });

    it("disables Add as Action… for a locked project", async () => {
      renderInspector(project({ id: "alpha", locked: true }));
      fireEvent.contextMenu(await screen.findByRole("button", { name: /^dev/ }));

      const item = screen.getByRole("menuitem", { name: "Add as Action…" });
      await userEvent.click(item);

      expect(item).toHaveAttribute("aria-disabled", "true");
      expect(useLibrary.getState().newAction).toBeNull();
    });
  });

  describe("Refresh and Updated", () => {
    it("shows when the older of the two reads was computed", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ computedAtMs: Date.now() - 2 * MINUTE }));
      mocked.fileStats.mockResolvedValue(fileStats({ computedAtMs: Date.now() - 10 * MINUTE }));
      renderInspector();

      expect(await screen.findByText("Updated 10 minutes ago")).toBeInTheDocument();
    });

    it("reads both stats past the cache and shows what they now say", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ kind: "Old kind" }));
      renderInspector();
      await screen.findByText("Old kind");
      mocked.quickStats.mockResolvedValue(quickStats({ kind: "New kind" }));
      mocked.fileStats.mockResolvedValue(fileStats({ files: 77 }));

      await userEvent.click(screen.getByRole("button", { name: "Refresh" }));

      expect(mocked.quickStats).toHaveBeenCalledWith("alpha", true);
      expect(mocked.fileStats).toHaveBeenCalledWith("alpha", true);
      expect(await screen.findByText("New kind")).toBeInTheDocument();
      expect(row("Files")).toHaveTextContent("77");
    });

    it("toasts the error when refreshing fails", async () => {
      renderInspector();
      await waitFor(() => expect(row("Last edited")).toHaveTextContent("–"));
      mocked.quickStats.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await userEvent.click(screen.getByRole("button", { name: "Refresh" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t refresh the details: Disk full");
    });
  });
});
