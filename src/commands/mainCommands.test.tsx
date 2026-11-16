import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { resetStore } from "../state/store";
import { MAIN_COMMANDS } from "./mainCommands";
import { clearCommands, registerCommands } from "./registry";
import { useShortcuts } from "./useShortcuts";

const mocked = vi.mocked(api, true);

function Host({ children }: { children?: React.ReactNode }) {
  useShortcuts();
  return (
    <>
      {children}
      <Toasts />
    </>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  clearCommands();
  registerCommands(MAIN_COMMANDS);
});

describe("main window close keys", () => {
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

  it("hides the window on Escape when the project search is empty", async () => {
    render(
      <Host>
        <input type="search" aria-label="Search projects" />
      </Host>,
    );
    screen.getByRole("searchbox", { name: "Search projects" }).focus();

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
  });

  it("keeps Escape for the project search while it has text", async () => {
    render(
      <Host>
        <input type="search" aria-label="Search projects" />
      </Host>,
    );
    await userEvent.type(screen.getByRole("searchbox", { name: "Search projects" }), "demo");

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
  });

  it("keeps Escape for a focused text field", async () => {
    render(
      <Host>
        <input aria-label="Tag name" />
      </Host>,
    );
    screen.getByRole("textbox", { name: "Tag name" }).focus();

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
  });

  it.each([
    ["dialog", "dialog"],
    ["alertdialog", "alertdialog"],
    ["menu", "menu"],
  ])("keeps Escape for an open %s", async (_name, role) => {
    render(
      <Host>
        <div role={role} aria-label="Open layer" />
      </Host>,
    );

    await userEvent.keyboard("{Escape}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
  });

  it("still hides the window on ⌘Q while a dialog is open", async () => {
    render(
      <Host>
        <div role="dialog" aria-label="Open layer" />
      </Host>,
    );

    await userEvent.keyboard("{Meta>}q{/Meta}");

    expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
  });

  it("shows an error toast when the window cannot be hidden", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "io", message: "No window", code: null });
    render(<Host />);

    await userEvent.keyboard("{Meta>}q{/Meta}");

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not close the window: No window");
  });
});
