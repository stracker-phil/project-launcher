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

/** The row of a label: label and value share one parent. "Files" and "Lines" are also table headers, so rows can be scoped to a section. */
const row = (label: string, section?: string) =>
  (section ? within(screen.getByRole("region", { name: section })) : screen).getByText(label).parentElement as HTMLElement;
const overview = (label: string) => row(label, "Overview");

const devScript: PackageScript = { manager: "npm", name: "dev", script: "vite", command: "npm run dev" };
const buildScript: PackageScript = { manager: "pnpm", name: "build-site", script: "vite build", command: "pnpm run build-site" };

beforeEach(() => {
  vi.clearAllMocks();
  mocked.quickStats.mockResolvedValue(EMPTY_QUICK_STATS);
  mocked.fileStats.mockResolvedValue(EMPTY_FILE_STATS);
  resetStore();
  resetLibrary();
  localStorage.clear();
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
    it("shows the project's kind as Type and its requirements as Requires, joined by dots", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ kind: "Node project", requirements: ["Node 20", "pnpm 9"] }));
      renderInspector();

      await screen.findByText("Node project");
      expect(overview("Type")).toHaveTextContent("Node project");
      expect(overview("Requires")).toHaveTextContent("Node 20 · pnpm 9");
    });

    it("leaves out Type, Requires and Packages when there is nothing to show", async () => {
      renderInspector();
      await waitFor(() => expect(overview("Last edited")).toHaveTextContent("–"));

      const section = within(screen.getByRole("region", { name: "Overview" }));
      expect(section.queryByText("Type")).not.toBeInTheDocument();
      expect(section.queryByText("Requires")).not.toBeInTheDocument();
      expect(section.queryByText("Packages")).not.toBeInTheDocument();
    });

    it("shows the packages of the project", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ packages: "42 dependencies" }));
      renderInspector();

      await waitFor(() => expect(overview("Packages")).toHaveTextContent("42 dependencies"));
    });

    it("shows files, sizes and lines of the scan", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 1234, sourceBytes: 1536, diskBytes: 3 * 1024 * 1024, lines: 56789 }));
      renderInspector();

      await waitFor(() => expect(overview("Files")).toHaveTextContent(/1\D?234/));
      expect(overview("Source size")).toHaveTextContent("1.5");
      expect(overview("On disk")).toHaveTextContent("3");
      expect(overview("Lines")).toHaveTextContent(/56\D?789/);
    });

    it("marks the file count with a plus when the scan was cut short", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 50000, truncated: true }));
      renderInspector();

      await waitFor(() => expect(overview("Files")).toHaveTextContent(/50\D?000\+/));
    });

    it("leaves out the plus when the scan was complete", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 40 }));
      renderInspector();

      await waitFor(() => expect(overview("Files")).toHaveTextContent("40"));
      expect(overview("Files")).not.toHaveTextContent("+");
    });

    it("shows a dash for a disk usage that could not be read", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 3, diskBytes: null }));
      renderInspector();

      await waitFor(() => expect(overview("On disk")).toHaveTextContent("–"));
    });

    it("shows when the newest file was edited, with its path as the title", async () => {
      mocked.fileStats.mockResolvedValue(fileStats({ files: 2, recent: [{ path: "src/app.ts", atMs: Date.now() - 3 * DAY }, { path: "README.md", atMs: Date.now() - 9 * DAY }] }));
      renderInspector();

      await waitFor(() => expect(overview("Last edited")).toHaveTextContent("3 days ago"));
      expect(within(overview("Last edited")).getByTitle("src/app.ts")).toBeInTheDocument();
    });

    it("shows a dash for Last edited when no file was found", async () => {
      renderInspector();

      await waitFor(() => expect(overview("Last edited")).toHaveTextContent("–"));
    });

    it.each(["Files", "Source size", "On disk", "Lines", "Last edited"])("shows an ellipsis for %s while the file scan reads", (label) => {
      mocked.fileStats.mockReturnValue(new Promise(() => {}));
      renderInspector();

      expect(overview(label)).toHaveTextContent("…");
    });
  });

  describe("Languages", () => {
    const withLanguages = () =>
      mocked.fileStats.mockResolvedValue(
        fileStats({
          files: 1838,
          languages: [
            { name: "TypeScript", color: "Blue", files: 1834, lines: 12000, bytes: 7500 },
            { name: "CSS", color: null, files: 4, lines: 1_200_000, bytes: 2500 },
          ],
        }),
      );

    it("tabulates each language with its files, lines and size under column headers", async () => {
      withLanguages();
      renderInspector();

      const languages = await screen.findByRole("region", { name: "Languages" });
      for (const header of ["Language", "Files", "Lines", "Size"]) expect(within(languages).getByText(header)).toBeInTheDocument();
      expect(within(languages).getByRole("img", { name: "TypeScript, CSS" })).toBeInTheDocument();
    });

    it("shortens the file and line counts of each language", async () => {
      withLanguages();
      renderInspector();

      const typescript = await screen.findByTitle("TypeScript: 75%");
      const css = screen.getByTitle("CSS: 25%");
      expect([...typescript.children].slice(0, 3).map((cell) => cell.textContent)).toEqual(["TypeScript", "1.8K", "12K"]);
      expect([...css.children].slice(0, 3).map((cell) => cell.textContent)).toEqual(["CSS", "4", "1.2M"]);
    });

    it("keeps the percentages out of the visible text", async () => {
      withLanguages();
      renderInspector();

      const languages = await screen.findByRole("region", { name: "Languages" });
      expect(within(languages).queryByText("75%")).not.toBeInTheDocument();
      expect(within(languages).queryByText("25%")).not.toBeInTheDocument();
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

    it("shows branch, commit count, start month and latest tag", async () => {
      mocked.quickStats.mockResolvedValue(
        quickStats({
          git: gitStats({ branch: "feature/search", commits: 1200, started: "Mar 2024", latestTag: "v1.4.0" }),
        }),
      );
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Branch")).toHaveTextContent("feature/search");
      expect(row("Commits")).toHaveTextContent(/^Commits1\D?200$/);
      expect(row("Started")).toHaveTextContent("Mar 2024");
      expect(row("Latest tag")).toHaveTextContent("v1.4.0");
    });

    it("shows the last commit's subject, then when and by whom", async () => {
      mocked.quickStats.mockResolvedValue(
        quickStats({ git: gitStats({ lastCommit: { subject: "Fix the search", author: "Sam Example", atMs: Date.now() - 2 * DAY } }) }),
      );
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      const commit = row("Last commit");
      expect(commit).toHaveTextContent(/^Last commitFix the search2 days ago · Sam Example$/);
      expect(within(commit).getByTitle("Sam Example")).toHaveTextContent("2 days ago · Sam Example");
    });

    it("leaves out Started when there is no start date", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ started: null }) }));
      renderInspector();

      const git = await screen.findByRole("region", { name: "Git" });
      expect(within(git).queryByText("Started")).not.toBeInTheDocument();
    });

    it("says Detached without a branch", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ branch: null }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Branch")).toHaveTextContent("Detached");
    });

    it.each([
      { why: "nothing changed", modified: 0, untracked: 0, text: "Clean" },
      { why: "files changed and are new", modified: 3, untracked: 1, text: "3 modified · 1 untracked" },
      { why: "files only changed", modified: 3, untracked: 0, text: "3 modified" },
      { why: "files are only new", modified: 0, untracked: 1, text: "1 untracked" },
    ])("shows '$text' for Changes when $why", async ({ modified, untracked, text }) => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ modified, untracked }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Changes")).toHaveTextContent(`Changes${text}`);
    });

    it.each([
      { why: "there is no remote", hasUpstream: false, ahead: 0, behind: 0, text: "No remote" },
      { why: "the branch is level", hasUpstream: true, ahead: 0, behind: 0, text: "Up to date" },
      { why: "the branch is ahead", hasUpstream: true, ahead: 2, behind: 0, text: "2 ahead" },
      { why: "the branch is behind", hasUpstream: true, ahead: 0, behind: 1, text: "1 behind" },
      { why: "the branch is ahead and behind", hasUpstream: true, ahead: 2, behind: 1, text: "2 ahead · 1 behind" },
      { why: "there is no remote but commits are ahead", hasUpstream: false, ahead: 3, behind: 0, text: "No remote" },
    ])("shows '$text' for Remote when $why", async ({ hasUpstream, ahead, behind, text }) => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ hasUpstream, ahead, behind }) }));
      renderInspector();

      await screen.findByRole("region", { name: "Git" });
      expect(row("Remote")).toHaveTextContent(`Remote${text}`);
    });

    it("leaves out Last commit and Latest tag when there are none", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ lastCommit: null, latestTag: null }) }));
      renderInspector();

      const git = await screen.findByRole("region", { name: "Git" });
      expect(within(git).queryByText("Last commit")).not.toBeInTheDocument();
      expect(within(git).queryByText("Latest tag")).not.toBeInTheDocument();
    });

    it("summarizes the weekly commits as an image", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ weekly: [1, 0, 4] }) }));
      renderInspector();

      expect(await screen.findByRole("img", { name: "5 commits in 3 weeks" })).toBeInTheDocument();
    });

    it.each([
      { weekly: [1, 0, 4], text: "5 commits in the last 3 weeks" },
      { weekly: [1, 0], text: "1 commit in the last 2 weeks" },
    ])("captions the activity '$text'", async ({ weekly, text }) => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ weekly }) }));
      renderInspector();

      expect(await screen.findByText(text)).toBeInTheDocument();
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

  describe("collapsing a section", () => {
    const toggle = (title: string) => screen.getByRole("button", { name: title });

    beforeEach(() => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ branch: "feature/search" }) }));
    });

    it("starts expanded", async () => {
      renderInspector();

      await screen.findByText("feature/search");
      expect(toggle("Git")).toHaveAttribute("aria-expanded", "true");
    });

    it("hides the section's body and reports it collapsed when its title is clicked", async () => {
      renderInspector();
      await screen.findByText("feature/search");

      await userEvent.click(toggle("Git"));

      expect(toggle("Git")).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("feature/search")).not.toBeInTheDocument();
      expect(toggle("Overview")).toHaveAttribute("aria-expanded", "true");
    });

    it("shows the body again on a second click", async () => {
      renderInspector();
      await screen.findByText("feature/search");

      await userEvent.click(toggle("Git"));
      await userEvent.click(toggle("Git"));

      expect(toggle("Git")).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByText("feature/search")).toBeInTheDocument();
    });

    it("remembers a collapsed section the next time the inspector shows", async () => {
      const first = renderInspector();
      await screen.findByText("feature/search");
      await userEvent.click(toggle("Git"));
      first.unmount();

      renderInspector();

      await waitFor(() => expect(toggle("Git")).toHaveAttribute("aria-expanded", "false"));
      expect(screen.queryByText("feature/search")).not.toBeInTheDocument();
    });

    it("forgets the fold once the section is expanded again", async () => {
      const first = renderInspector();
      await screen.findByText("feature/search");
      await userEvent.click(toggle("Git"));
      await userEvent.click(toggle("Git"));
      first.unmount();

      renderInspector();

      expect(await screen.findByText("feature/search")).toBeInTheDocument();
    });

    it("starts expanded when the stored fold cannot be read", async () => {
      const read = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("Storage is blocked");
      });
      renderInspector();

      expect(await screen.findByText("feature/search")).toBeInTheDocument();
      read.mockRestore();
    });

    it("still folds for the session when the fold cannot be stored", async () => {
      const write = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("Storage is full");
      });
      renderInspector();
      await screen.findByText("feature/search");

      await userEvent.click(toggle("Git"));

      expect(toggle("Git")).toHaveAttribute("aria-expanded", "false");
      write.mockRestore();
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
