import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Action, type Project } from "../../api";
import { resetRuns } from "../../state/runs";
import { resetStore, useStore } from "../../state/store";
import { action, project } from "../../test/library";
import { useActionShortcuts } from "./useActionShortcuts";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const editor = action("Open in Editor", { shortcut: "Mod+O" });
const build = action("Build site", { command: "npm run build", shortcut: "Mod+Shift+B" }, "scripts", 0);
const bare = action("Notes", { shortcut: "G" }, "apps", 1);
const actions = [editor, build, bare];

function Detail({ shown, list = actions }: { shown: Project | null; list?: Action[] }) {
  useActionShortcuts(shown, list);
  return <input aria-label="Search" />;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetRuns();
});

describe("useActionShortcuts", () => {
  it("launches the action whose shortcut is pressed", async () => {
    render(<Detail shown={alpha} />);

    await userEvent.keyboard("{Meta>}o{/Meta}");

    expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 });
  });

  it("matches a shortcut with several modifiers and launches a script", async () => {
    render(<Detail shown={alpha} />);

    await userEvent.keyboard("{Meta>}{Shift>}b{/Shift}{/Meta}");

    expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "scripts", index: 0 });
  });

  it("ignores a key that no action has", async () => {
    render(<Detail shown={alpha} />);

    await userEvent.keyboard("{Meta>}j{/Meta}");

    expect(mocked.launchAction).not.toHaveBeenCalled();
  });

  it("does nothing while no project is shown", async () => {
    render(<Detail shown={null} />);

    await userEvent.keyboard("{Meta>}o{/Meta}");

    expect(mocked.launchAction).not.toHaveBeenCalled();
  });

  it("follows the actions it is given", async () => {
    const { rerender } = render(<Detail shown={alpha} />);
    rerender(<Detail shown={alpha} list={[]} />);

    await userEvent.keyboard("{Meta>}o{/Meta}");

    expect(mocked.launchAction).not.toHaveBeenCalled();
  });

  describe("in a text field", () => {
    it("does not launch on a bare key", async () => {
      render(<Detail shown={alpha} />);
      await userEvent.click(screen.getByRole("textbox", { name: "Search" }));

      await userEvent.keyboard("g");

      expect(screen.getByRole("textbox", { name: "Search" })).toHaveValue("g");
      expect(mocked.launchAction).not.toHaveBeenCalled();
    });

    it("launches on a ⌘ combination", async () => {
      render(<Detail shown={alpha} />);
      await userEvent.click(screen.getByRole("textbox", { name: "Search" }));

      await userEvent.keyboard("{Meta>}o{/Meta}");

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 });
    });

    it("launches a bare-key action outside a text field", async () => {
      render(<Detail shown={alpha} />);

      await userEvent.keyboard("g");

      expect(mocked.launchAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 1 });
    });
  });

  describe("while something else has the keys", () => {
    it("ignores the shortcut while a menu is open", async () => {
      render(<Detail shown={alpha} />);
      useStore.getState().openMenu([{ text: "Open" }], { x: 1, y: 1 });

      await userEvent.keyboard("{Meta>}o{/Meta}");

      expect(mocked.launchAction).not.toHaveBeenCalled();
    });

    it("ignores the shortcut while a confirmation is open", async () => {
      render(<Detail shown={alpha} />);
      void useStore.getState().confirm({ title: "Remove?", text: "This removes it.", action: "Remove" });

      await userEvent.keyboard("{Meta>}o{/Meta}");

      expect(mocked.launchAction).not.toHaveBeenCalled();
    });
  });
});
