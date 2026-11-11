import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { clearCommands, registerCommands } from "./registry";
import { useShortcuts } from "./useShortcuts";

function Host({ children }: { children?: React.ReactNode }) {
  useShortcuts();
  return <>{children}</>;
}

describe("useShortcuts", () => {
  const run = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    clearCommands();
    registerCommands([{ id: "demo.run", title: "Run demo", defaultBinding: "Mod+K", run }]);
  });

  it("runs the command bound to the pressed key", async () => {
    render(<Host />);

    await userEvent.keyboard("{Meta>}k{/Meta}");

    expect(run).toHaveBeenCalled();
  });

  it("prevents the default action of a key that ran a command", () => {
    render(<Host />);

    const notPrevented = fireEvent.keyDown(document.body, { key: "k", metaKey: true });

    expect(notPrevented).toBe(false);
  });

  it("leaves a key without a command alone", () => {
    render(<Host />);

    const notPrevented = fireEvent.keyDown(document.body, { key: "j", metaKey: true });

    expect(notPrevented).toBe(true);
    expect(run).not.toHaveBeenCalled();
  });

  it("swallows a held key without running the command again", () => {
    render(<Host />);

    const notPrevented = fireEvent.keyDown(document.body, { key: "k", metaKey: true, repeat: true });

    expect(notPrevented).toBe(false);
    expect(run).not.toHaveBeenCalled();
  });

  it("does not run a command when Enter activates a focused button", async () => {
    registerCommands([{ id: "demo.enter", title: "Enter command", defaultBinding: "Enter", run }]);
    const onClick = vi.fn();
    render(
      <Host>
        <button onClick={onClick}>Apply</button>
      </Host>,
    );
    screen.getByRole("button", { name: "Apply" }).focus();

    await userEvent.keyboard("{Enter}");

    expect(run).not.toHaveBeenCalled();
    expect(onClick).toHaveBeenCalled();
  });

  it("ignores keys pressed inside a context menu", async () => {
    render(
      <Host>
        <div role="menu">
          <button role="menuitem">Rename</button>
        </div>
      </Host>,
    );
    screen.getByRole("menuitem", { name: "Rename" }).focus();

    await userEvent.keyboard("{Meta>}k{/Meta}");

    expect(run).not.toHaveBeenCalled();
  });

  it("stops routing keys once unmounted", async () => {
    const { unmount } = render(<Host />);
    unmount();

    await userEvent.keyboard("{Meta>}k{/Meta}");

    expect(run).not.toHaveBeenCalled();
  });
});
