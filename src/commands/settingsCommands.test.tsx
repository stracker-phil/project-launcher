import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resetStore } from "../state/store";
import { clearCommands, registerCommands } from "./registry";
import { SETTINGS_COMMANDS } from "./settingsCommands";
import { useShortcuts } from "./useShortcuts";

const mocked = vi.mocked(api, true);

function Host({ children }: { children?: React.ReactNode }) {
  useShortcuts();
  return <>{children}</>;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  clearCommands();
  registerCommands(SETTINGS_COMMANDS);
});

describe("Settings window close keys", () => {
  it("hides the window on ⌘Q", async () => {
    render(<Host />);

    await userEvent.keyboard("{Meta>}q{/Meta}");

    expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
  });

  it("hides the window on Escape when nothing has focus", async () => {
    render(<Host />);

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
  });

  it("keeps Escape for a focused text field", async () => {
    render(
      <Host>
        <input aria-label="Search Settings" />
      </Host>,
    );
    screen.getByRole("textbox", { name: "Search Settings" }).focus();

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
  });

  it("keeps Escape for an open menu", async () => {
    render(
      <Host>
        <div role="menu" aria-label="Accent" />
      </Host>,
    );

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
  });
});
