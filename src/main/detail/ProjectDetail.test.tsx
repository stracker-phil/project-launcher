import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Library, type Project, type ProjectConfig } from "../../api";
import { MenuHost } from "../../components/menu";
import { Toasts } from "../../components/Toasts";
import { createQueryClient } from "../../queries/queryClient";
import { resetLibrary } from "../../state/library";
import { inspectorShown, resetStore, useStore } from "../../state/store";
import { EMPTY_DEV_STATUS, EMPTY_LIBRARY, EMPTY_PROJECT_CONFIG, EMPTY_STATUS } from "../../test/apiMock";
import { action, library, project, section } from "../../test/library";
import { gitStats, quickStats } from "../../test/stats";
import { DetailTools, ProjectDetail } from "./ProjectDetail";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const NOTHING_YET = "Nothing to open yet: add an action or a built-in app with +.";

/** `null` as `lib` renders without a library, as while it loads. */
function renderDetail(shown: Project | null, lib: Library | null = library(shown ? [section(null, shown)] : [])) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <ProjectDetail project={shown} library={lib ?? undefined} />
      <MenuHost />
      <Toasts />
    </QueryClientProvider>,
  );
}

const withConfig = (patch: Partial<ProjectConfig>) => mocked.projectConfig.mockResolvedValue({ ...EMPTY_PROJECT_CONFIG, ...patch });

/** InlineText selects its text on the next frame; typing before that would be replaced by it. */
const nextFrame = () => new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));

async function edit(label: string, text: string, finish = "{Enter}") {
  await userEvent.click(screen.getByRole("button", { name: label }));
  await nextFrame();
  const field = screen.getByRole("textbox", { name: label });
  await userEvent.clear(field);
  if (text) await userEvent.type(field, text);
  await userEvent.keyboard(finish);
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.projectConfig.mockResolvedValue(EMPTY_PROJECT_CONFIG);
  mocked.projectStatus.mockResolvedValue(EMPTY_STATUS);
  mocked.devStatus.mockResolvedValue(EMPTY_DEV_STATUS);
  mocked.quickStats.mockResolvedValue(quickStats({ git: null }));
  resetStore();
  resetLibrary();
});

describe("ProjectDetail", () => {
  describe("without a project", () => {
    it("invites to add a project folder when the library has no projects at all", async () => {
      renderDetail(null, EMPTY_LIBRARY);

      expect(screen.getByRole("heading", { name: "No projects yet" })).toBeInTheDocument();
      await userEvent.click(screen.getByRole("button", { name: "Add Project Folder…" }));

      expect(mocked.addProjectFolders).toHaveBeenCalled();
    });

    it.each([
      { why: "the library has projects", lib: library([section(null, alpha)]) },
      { why: "only archived projects exist", lib: library([], { sidebar: { ...EMPTY_LIBRARY.sidebar, archived: 2 } }) },
      { why: "the library has not loaded", lib: null },
    ])("asks to choose a project when $why", ({ lib }) => {
      renderDetail(null, lib);

      expect(screen.getByRole("heading", { name: "No Project Selected" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Add Project Folder…" })).not.toBeInTheDocument();
    });
  });

  describe("header", () => {
    it("shows the name, tag and notes of the project", () => {
      renderDetail(project({ name: "Alpha", meta: { tag: "client", notes: ["Call the demo client", "Ship v2"] } }));

      expect(screen.getByRole("button", { name: "Project name" })).toHaveTextContent("Alpha");
      expect(screen.getByRole("button", { name: "client" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Notes" })).toHaveTextContent(/Call the demo client\s+Ship v2/);
    });

    it("offers Add Tag and Add a note… for a project without them", () => {
      renderDetail(alpha);

      expect(screen.getByRole("button", { name: "Add Tag" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Notes" })).toHaveTextContent("Add a note…");
    });

    it("shows the git branch as a pill", async () => {
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ branch: "feature/demo" }) }));

      renderDetail(alpha);

      expect(await screen.findByTitle("feature/demo")).toHaveTextContent("feature/demo");
    });

    it("keeps the full name of a long branch as the pill's title", async () => {
      const branch = "feature/demo-a-very-long-branch-name-that-the-pill-has-to-cut-with-an-ellipsis";
      mocked.quickStats.mockResolvedValue(quickStats({ git: gitStats({ branch }) }));

      renderDetail(alpha);

      expect(await screen.findByTitle(branch)).toHaveTextContent(branch);
    });

    it("shows no branch pill for a folder that is not a git repository", async () => {
      renderDetail(alpha);
      await screen.findByText(NOTHING_YET);

      expect(screen.queryByTitle(/feature|main/)).not.toBeInTheDocument();
    });

    it("shows an Archived badge for an archived project", () => {
      renderDetail(project({ meta: { archived: true } }));

      expect(screen.getByText("Archived")).toBeInTheDocument();
    });

    it("shows no badge for an active, unlocked project", () => {
      renderDetail(alpha);

      expect(screen.queryByText("Archived")).not.toBeInTheDocument();
      expect(screen.queryByText("Locked")).not.toBeInTheDocument();
    });

    it("shows the file error as a callout for an unavailable project, without reading its config", () => {
      renderDetail(project({ available: false, fileError: "The project file is not valid JSON" }));

      expect(screen.getByText("The project file is not valid JSON")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Project name" })).not.toBeInTheDocument();
      expect(mocked.projectConfig).not.toHaveBeenCalled();
    });

    it("renames the project through the name field", async () => {
      renderDetail(alpha);

      await edit("Project name", "Beta");

      expect(mocked.setProjectName).toHaveBeenCalledWith("alpha", "Beta");
      expect(screen.getByRole("button", { name: "Project name" })).toBeInTheDocument();
    });

    it("keeps the name when the edit is cancelled with Escape", async () => {
      renderDetail(alpha);

      await edit("Project name", "Beta", "{Escape}");

      expect(mocked.setProjectName).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Project name" })).toHaveTextContent("Alpha");
    });

    it("shows the name field when a menu or command put the name into edit mode", () => {
      resetLibrary({ editing: { id: "alpha", field: "name" } });

      renderDetail(alpha);

      expect(screen.getByRole("textbox", { name: "Project name" })).toHaveValue("Alpha");
    });

    it("saves the note lines on ⌘↩", async () => {
      renderDetail(alpha);

      await edit("Notes", "First line{Enter}Second line", "{Meta>}{Enter}{/Meta}");

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "notes", value: ["First line", "Second line"] });
    });

    it("saves no lines when the note is cleared", async () => {
      renderDetail(project({ meta: { notes: ["Old note"] } }));

      await edit("Notes", "", "{Meta>}{Enter}{/Meta}");

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "notes", value: [] });
    });
  });

  describe("a locked project", () => {
    const locked = project({ name: "Alpha", locked: true, meta: { tag: "client", notes: ["Read only note"] } });

    it("shows the name, tag and notes as plain text without a Locked badge", () => {
      renderDetail(locked);

      expect(screen.getByText("Alpha")).toBeInTheDocument();
      expect(screen.getByText("client")).toBeInTheDocument();
      expect(screen.getByText("Read only note")).toBeInTheDocument();
      expect(screen.queryByText("Locked")).not.toBeInTheDocument();
    });

    it.each(["Project name", "Notes", "Change Icon & Color", "Website", "Repository"])("offers no %s button", (name) => {
      renderDetail(locked);

      expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
    });

    it("hides the note and the empty links when there is nothing to read", () => {
      renderDetail(project({ locked: true }));

      expect(screen.queryByText("Add a note…")).not.toBeInTheDocument();
      expect(screen.queryByText("Website")).not.toBeInTheDocument();
      expect(screen.queryByText("Repository")).not.toBeInTheDocument();
    });

    it("shows the links as text", () => {
      renderDetail(project({ locked: true, meta: { url: "https://example.com", repoUrl: "https://git.example.com/alpha" } }));

      expect(screen.getByText("https://example.com")).toBeInTheDocument();
      expect(screen.getByText("https://git.example.com/alpha")).toBeInTheDocument();
    });
  });

  describe("launch config", () => {
    it("lists the project's apps under Open, with their shortcuts", async () => {
      withConfig({ apps: [action("Open in Editor", { shortcut: "Mod+E" }), action("Terminal", {}, "apps", 1)] });

      renderDetail(alpha);

      await screen.findByText("Open in Editor");
      expect(screen.getByRole("heading", { name: "Open" })).toBeInTheDocument();
      expect(screen.getByText("Terminal")).toBeInTheDocument();
      expect(screen.getByLabelText("⌘E")).toBeInTheDocument();
    });

    it("lists the scripts with their commands", async () => {
      withConfig({ scripts: [action("Build site", { command: "npm run build" }, "scripts")] });

      renderDetail(alpha);

      expect(await screen.findByRole("heading", { name: "Scripts" })).toBeInTheDocument();
      expect(screen.getByText("Build site")).toBeInTheDocument();
      expect(screen.getByText("npm run build")).toBeInTheDocument();
    });

    it("says nothing is there to open and shows no Scripts section for an empty config", async () => {
      renderDetail(alpha);

      expect(await screen.findByText(NOTHING_YET)).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: "Scripts" })).not.toBeInTheDocument();
    });

    it("shows the project's states as chips above the actions", async () => {
      mocked.projectStatus.mockResolvedValue({ ...EMPTY_STATUS, states: [{ label: "Branch", value: "main" }, { label: "Server", value: "Running" }] });

      renderDetail(alpha);

      const states = await screen.findByRole("list", { name: "States" });
      expect(within(states).getAllByRole("listitem").map((chip) => chip.textContent)).toEqual(["Branchmain", "ServerRunning"]);
      expect(mocked.projectStatus).toHaveBeenCalledWith("alpha");
    });

    it("shows no states row for a project without states", async () => {
      renderDetail(alpha);
      await screen.findByText(NOTHING_YET);

      expect(screen.queryByRole("list", { name: "States" })).not.toBeInTheDocument();
    });

    it("leaves out the actions the project's status hides", async () => {
      withConfig({ apps: [action("Open in Editor"), action("Terminal", {}, "apps", 1)] });
      mocked.projectStatus.mockResolvedValue({ ...EMPTY_STATUS, hidden: [{ list: "apps", index: 1 }] });

      renderDetail(alpha);

      await screen.findByRole("button", { name: /Open in Editor/ });
      await waitFor(() => expect(screen.queryByRole("button", { name: /Terminal/ })).not.toBeInTheDocument());
    });

    it("launches an action from its tile", async () => {
      withConfig({ apps: [action("Open in Editor")] });
      renderDetail(alpha);

      await userEvent.click(await screen.findByRole("button", { name: /Open in Editor/ }));

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 });
    });

    it("launches an action with its shortcut", async () => {
      withConfig({ apps: [action("Open in Editor", { shortcut: "Mod+E" })] });
      renderDetail(alpha);
      await screen.findByRole("button", { name: /Open in Editor/ });

      await userEvent.keyboard("{Meta>}e{/Meta}");

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 });
    });

    it("does not launch an action hidden by the status with its shortcut", async () => {
      withConfig({ apps: [action("Open in Editor", { shortcut: "Mod+E" })] });
      mocked.projectStatus.mockResolvedValue({ ...EMPTY_STATUS, hidden: [{ list: "apps", index: 0 }] });
      renderDetail(alpha);
      await screen.findByText(NOTHING_YET);

      await userEvent.keyboard("{Meta>}e{/Meta}");

      expect(mocked.launchAction).not.toHaveBeenCalled();
    });

    it("shows the project's dev environment from its dev status", async () => {
      mocked.devStatus.mockResolvedValue({
        docker: { running: true, activity: null, manager: "OrbStack" },
        server: null,
        database: { source: "ddev", engine: "MariaDB" },
      });

      renderDetail(alpha);

      expect(await screen.findByText("Running · OrbStack")).toBeInTheDocument();
      expect(screen.getByText("MariaDB · DDEV")).toBeInTheDocument();
      expect(mocked.devStatus).toHaveBeenCalledWith("alpha");
    });

    it("puts the dev environment before the Open section", async () => {
      mocked.devStatus.mockResolvedValue({ docker: { running: true, activity: null, manager: "OrbStack" }, server: null, database: null });

      renderDetail(alpha);

      const dev = await screen.findByRole("heading", { name: "Dev Environment" });
      const open = screen.getByRole("heading", { name: "Open" });
      expect(dev.compareDocumentPosition(open) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("shows no dev environment section for a project without one", async () => {
      renderDetail(alpha);
      await screen.findByText(NOTHING_YET);

      expect(screen.queryByRole("heading", { name: "Dev Environment" })).not.toBeInTheDocument();
    });

    it("offers adding actions while unlocked", async () => {
      renderDetail(alpha);

      expect(await screen.findByRole("button", { name: "Add Action" })).toBeInTheDocument();
    });

    it("lists the environment variables", async () => {
      withConfig({ env: [{ name: "APP_ENV", value: "local" }, { name: "APP_PORT", value: "8080" }] });

      renderDetail(alpha);

      expect(await screen.findByRole("heading", { name: "Environment Variables" })).toBeInTheDocument();
      expect(screen.getByText("APP_ENV")).toBeInTheDocument();
      expect(screen.getByText("local")).toBeInTheDocument();
      expect(screen.getByText("APP_PORT")).toBeInTheDocument();
    });

    it("shows no environment section without variables", async () => {
      renderDetail(alpha);
      await screen.findByText(NOTHING_YET);

      expect(screen.queryByRole("heading", { name: "Environment Variables" })).not.toBeInTheDocument();
    });
  });

  describe("info", () => {
    it("shows the folder and the editor of the project", () => {
      renderDetail(project({ path: "~/Projects/alpha", meta: { editor: "Demo Editor" } }));

      expect(screen.getByText("~/Projects/alpha")).toBeInTheDocument();
      expect(screen.getByText("Demo Editor")).toBeInTheDocument();
    });

    it.each([
      { label: "Website", field: "url" },
      { label: "Repository", field: "repoUrl" },
    ])("saves the $label link as $field", async ({ label, field }) => {
      renderDetail(alpha);

      await edit(label, "https://example.com/demo");

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field, value: "https://example.com/demo" });
    });

    it.each([
      { name: "Open Website", meta: { url: "https://example.com" }, opens: "https://example.com" },
      { name: "Open Repository", meta: { repoUrl: "https://git.example.com/alpha" }, opens: "https://git.example.com/alpha" },
    ])("opens the link with $name", async ({ name, meta, opens }) => {
      renderDetail(project({ meta }));

      await userEvent.click(screen.getByRole("button", { name }));

      expect(mocked.openUrl).toHaveBeenCalledWith(opens);
    });

    it.each(["Open Website", "Open Repository"])("disables %s while the link is empty", (name) => {
      renderDetail(alpha);

      expect(screen.getByRole("button", { name })).toBeDisabled();
    });

    it("turns the Website value into its field with Edit Links", async () => {
      renderDetail(alpha);

      await userEvent.click(screen.getByRole("button", { name: "Edit Links" }));

      expect(screen.getByRole("textbox", { name: "Website" })).toBeInTheDocument();
    });

    it("offers no Edit Links for a locked project", () => {
      renderDetail(project({ locked: true }));

      expect(screen.queryByRole("button", { name: "Edit Links" })).not.toBeInTheDocument();
    });

    it("removes the link when its field is cleared", async () => {
      renderDetail(project({ meta: { url: "https://example.com" } }));

      await edit("Website", "");

      expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "url", value: null });
    });

    it("shows an error toast when saving the link fails", async () => {
      mocked.setProjectMeta.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderDetail(alpha);

      await edit("Website", "https://example.com");

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t update link: Disk full");
    });

    describe("dev environment", () => {
      it("shows the project's mode on a button that opens the choices with the mode checked", async () => {
        withConfig({ dev: "ddev" });
        renderDetail(alpha);

        await userEvent.click(await screen.findByRole("button", { name: "Docker, DDEV" }));

        const menu = screen.getByRole("menu", { name: "Dev Environment" });
        expect(within(menu).getAllByRole("menuitemradio").map((r) => r.textContent)).toEqual(["None", "Docker", "Docker, DDEV", "Docker, wp-env"]);
        expect(within(menu).getByRole("menuitemradio", { name: "Docker, DDEV" })).toHaveAttribute("aria-checked", "true");
        expect(within(menu).getByRole("menuitemradio", { name: "None" })).toHaveAttribute("aria-checked", "false");
      });

      it("sets the chosen mode through api.setDevMode", async () => {
        withConfig({ dev: "ddev" });
        renderDetail(alpha);
        await userEvent.click(await screen.findByRole("button", { name: "Docker, DDEV" }));

        await userEvent.click(screen.getByRole("menuitemradio", { name: "Docker, wp-env" }));

        expect(mocked.setDevMode).toHaveBeenCalledWith("alpha", "wp-env");
      });

      it("shows an error toast when changing the mode fails", async () => {
        withConfig({ dev: "none" });
        mocked.setDevMode.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
        renderDetail(alpha);
        await userEvent.click(await screen.findByRole("button", { name: "None" }));

        await userEvent.click(screen.getByRole("menuitemradio", { name: "Docker" }));

        expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change the dev environment: Disk full");
      });

      it("shows the mode as text for a locked project", async () => {
        withConfig({ dev: "ddev", locked: true });
        renderDetail(project({ locked: true }));

        expect(await screen.findByText("Docker, DDEV")).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: "Docker, DDEV" })).not.toBeInTheDocument();
      });

      it("leaves the row out for a locked project without a dev environment", async () => {
        renderDetail(project({ locked: true }));
        await screen.findByText("Nothing to open.");

        expect(screen.queryByText("Dev Environment")).not.toBeInTheDocument();
      });
    });
  });
});

describe("DetailTools", () => {
  const renderTools = (shown: Project | null) => render(<DetailTools project={shown} />);

  it.each([
    { locked: false, name: "Lock Project Details", value: true },
    { locked: true, name: "Unlock Project Details", value: false },
  ])("sends locked $value when $name is clicked", async ({ locked, name, value }) => {
    renderTools(project({ locked }));

    await userEvent.click(screen.getByRole("button", { name }));

    expect(mocked.setProjectLocked).toHaveBeenCalledWith("alpha", value);
  });

  it.each([
    { starred: false, name: "Star Project", pressed: "false", value: true },
    { starred: true, name: "Unstar Project", pressed: "true", value: false },
  ])("sends starred $value when $name is clicked", async ({ starred, name, pressed, value }) => {
    renderTools(project({ meta: { starred } }));
    const button = screen.getByRole("button", { name });
    expect(button).toHaveAttribute("aria-pressed", pressed);

    await userEvent.click(button);

    expect(mocked.setProjectMeta).toHaveBeenCalledWith("alpha", { field: "starred", value });
  });

  it("flips the inspector when its toggle is clicked", async () => {
    renderTools(alpha);
    expect(inspectorShown(useStore.getState())).toBe(false);

    await userEvent.click(screen.getByRole("button", { name: "Toggle Inspector" }));

    expect(inspectorShown(useStore.getState())).toBe(true);
  });

  it.each([
    { why: "no project", shown: null },
    { why: "an unavailable project", shown: project({ available: false }) },
  ])("offers only the inspector toggle for $why", ({ shown }) => {
    renderTools(shown);

    expect(screen.getAllByRole("button").map((b) => b.getAttribute("aria-label"))).toEqual(["Toggle Inspector"]);
  });
});
