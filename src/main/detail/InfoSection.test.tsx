import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ProjectConfig } from "../../api";
import { resetStore } from "../../state/store";
import { EMPTY_PROJECT_CONFIG } from "../../test/apiMock";
import { project } from "../../test/library";
import { InfoSection } from "./InfoSection";

const mocked = vi.mocked(api, true);

const configWith = (claude: ProjectConfig["claude"]): ProjectConfig => ({ ...EMPTY_PROJECT_CONFIG, claude });

const renderInfo = (config: ProjectConfig, canManageClaude = true) =>
  render(<InfoSection project={project({ canManageClaude })} config={config} readOnly={false} />);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("InfoSection Claude row", () => {
  it("is absent when the project has no Claude setup", () => {
    renderInfo(configWith(null));

    expect(screen.queryByText("Claude")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Manage" })).not.toBeInTheDocument();
  });

  it("shows the profile of the Claude setup", () => {
    renderInfo(configWith({ profile: "work" }));

    expect(screen.getByText("Claude")).toBeInTheDocument();
    expect(screen.getByText("work")).toBeInTheDocument();
  });

  it("opens the project in Claude Manager when Manage is clicked", async () => {
    renderInfo(configWith({ profile: "work" }), true);

    const manage = screen.getByRole("button", { name: "Manage" });
    expect(manage).toBeEnabled();
    expect(manage).toHaveAttribute("title", "Open in Claude Manager");
    await userEvent.click(manage);

    expect(mocked.openClaudeManager).toHaveBeenCalledWith("alpha");
  });

  it("disables Manage and says why when the project cannot be managed", async () => {
    renderInfo(configWith({ profile: "work" }), false);

    const manage = screen.getByRole("button", { name: "Manage" });
    expect(manage).toBeDisabled();
    expect(manage).toHaveAttribute("title", "Claude Manager isn’t installed");
    await userEvent.click(manage);

    expect(mocked.openClaudeManager).not.toHaveBeenCalled();
  });
});
