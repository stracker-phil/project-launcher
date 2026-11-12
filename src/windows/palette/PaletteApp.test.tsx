import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { clearRecentSearches } from "../../palette/recent";
import { createQueryClient } from "../../queries/queryClient";
import { EMPTY_PROJECT_CONFIG, type createApiMock } from "../../test/apiMock";
import { action, libraryWithTags, project } from "../../test/library";
import { PaletteApp } from "./PaletteApp";

const mocked = vi.mocked(api, true);
const emit = (currentId: string | null) => act(() => (api as ReturnType<typeof createApiMock>).emit("palette_opened", currentId));

const alpha = project({ id: "alpha", name: "Alpha" });
const beta = project({ id: "beta", name: "Beta Shop", meta: { tag: "Client" } });

async function renderPalette() {
  render(<PaletteApp client={createQueryClient()} />);
  await waitFor(() => expect(mocked.on).toHaveBeenCalledWith("palette_opened", expect.any(Function)));
}

const field = () => screen.getByRole("combobox");

beforeEach(() => {
  vi.clearAllMocks();
  clearRecentSearches();
  mocked.listProjects.mockResolvedValue([alpha, beta]);
  mocked.library.mockResolvedValue(libraryWithTags());
  mocked.menuCommands.mockResolvedValue([
    { menu: "File", label: "New Project", command: "app:new", key: "⌘N" },
    { menu: "Project", label: "Reveal in Finder", command: "app:reveal", key: null },
    { menu: "View", label: "Command Palette", command: "app:palette", key: "⌃⌘P" },
  ]);
  mocked.projectConfig.mockResolvedValue({ ...EMPTY_PROJECT_CONFIG, apps: [action("Open in Editor")] });
});

describe("PaletteApp", () => {
  it("lists the projects, the library's filters and the menus' commands when opened", async () => {
    await renderPalette();

    emit(null);

    expect(await screen.findByRole("option", { name: /Beta Shop/ })).toBeInTheDocument();
    expect(await screen.findByRole("option", { name: /^Client\s*2$/ })).toBeInTheDocument();
    expect(await screen.findByRole("option", { name: /New Project/ })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /Command Palette/ })).not.toBeInTheDocument();
  });

  it("lists the current project's actions and the Project menu under its name and marks it as current", async () => {
    await renderPalette();

    emit("alpha");

    const actions = await screen.findByRole("region", { name: "Alpha" });
    expect(actions).toHaveTextContent("Open in Editor");
    expect(actions).toHaveTextContent("Reveal in Finder");
    expect(screen.getByRole("option", { name: /Alpha\s*Current/ })).toBeInTheDocument();
  });

  it("lists the current project's dev environment rows under its name", async () => {
    mocked.devCommands.mockImplementation(async (id) =>
      id === "alpha"
        ? [
            { menu: "Dev", label: "Start DDEV", command: "dev:server-start:alpha", key: null },
            { menu: "Dev", label: "Stop Docker", command: "dev:docker-stop:alpha", key: null },
          ]
        : [],
    );
    await renderPalette();

    emit("alpha");

    const actions = await screen.findByRole("region", { name: "Alpha" });
    await waitFor(() => expect(actions).toHaveTextContent("Start DDEV"));
    expect(actions).toHaveTextContent("Stop Docker");
  });

  it("runs a dev environment row by its bus id when it is chosen", async () => {
    mocked.devCommands.mockResolvedValue([{ menu: "Dev", label: "Start DDEV", command: "dev:server-start:alpha", key: null }]);
    await renderPalette();
    emit("alpha");

    await userEvent.click(await screen.findByRole("option", { name: /Start DDEV/ }));

    expect(mocked.runCommand).toHaveBeenCalledWith("dev:server-start:alpha");
  });

  it("asks Rust for the menu commands of the current project", async () => {
    await renderPalette();

    emit("alpha");

    await waitFor(() => expect(mocked.menuCommands).toHaveBeenCalledWith("alpha"));
  });

  it("offers no dev rows when no project is current", async () => {
    mocked.devCommands.mockResolvedValue([{ menu: "Dev", label: "Start DDEV", command: "dev:server-start:alpha", key: null }]);
    await renderPalette();

    emit(null);
    await screen.findByRole("option", { name: /Beta Shop/ });

    expect(screen.queryByRole("option", { name: /Start DDEV/ })).not.toBeInTheDocument();
  });

  it("offers no actions when no project is current", async () => {
    await renderPalette();

    emit(null);
    await screen.findByRole("option", { name: /Beta Shop/ });

    expect(screen.queryByRole("option", { name: /Open in Editor/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /Reveal in Finder/ })).not.toBeInTheDocument();
  });

  it("tells Rust the panel is drawn once it has been opened", async () => {
    await renderPalette();

    emit(null);

    await waitFor(() => expect(mocked.paletteShown).toHaveBeenCalled());
  });

  it("starts each opening with an empty search field", async () => {
    await renderPalette();
    emit(null);
    await userEvent.type(field(), "week");
    expect(field()).toHaveValue("week");

    emit("alpha");

    await waitFor(() => expect(field()).toHaveValue(""));
  });

  it("asks Rust to close the palette on Escape", async () => {
    await renderPalette();
    emit(null);
    await screen.findByRole("option", { name: /Beta Shop/ });

    await userEvent.keyboard("{Escape}");

    expect(mocked.closePalette).toHaveBeenCalled();
  });

  it("closes and shows the project in the main window when a project is chosen", async () => {
    await renderPalette();
    emit(null);

    await userEvent.click(await screen.findByRole("option", { name: /Beta Shop/ }));

    expect(mocked.closePalette).toHaveBeenCalled();
    expect(mocked.runCommand).toHaveBeenCalledWith("project:show:beta");
  });

  it("asks the main window to launch the current project's action when it is chosen", async () => {
    await renderPalette();
    emit("alpha");

    await userEvent.click(await screen.findByRole("option", { name: /Open in Editor/ }));

    expect(mocked.runCommand).toHaveBeenCalledWith("page:action:apps:0");
  });

  it("asks the main window to show a library filter when it is chosen", async () => {
    await renderPalette();
    emit(null);

    await userEvent.click(await screen.findByRole("option", { name: /^Client\s*2$/ }));

    expect(mocked.runCommand).toHaveBeenCalledWith('page:filter:{"kind":"tag","tag":"Client"}');
  });
});
