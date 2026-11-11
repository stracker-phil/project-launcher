import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { resetStore } from "../../state/store";
import { project } from "../../test/library";
import { ActionEditor, type EditorTarget } from "./ActionEditor";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });
const newAction: EditorTarget = { source: null, draft: { runsAs: "app", label: "" }, preset: null, defaults: null };
const existing: EditorTarget = { source: { list: "apps", index: 0 }, draft: { runsAs: "app", label: "Open in Editor", app: "Demo Editor" }, preset: null, defaults: null };
const builtin = (draft: EditorTarget["draft"]): EditorTarget => ({
  source: { list: "apps", index: 0 },
  draft,
  preset: "editor",
  defaults: { runsAs: "app", label: "Editor", app: "Zed" },
});

function Harness({ target, onClose }: { target: EditorTarget | null; onClose: () => void }) {
  const anchor = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button ref={anchor} type="button">Anchor</button>
      <ActionEditor project={alpha} target={target} anchor={anchor} onClose={onClose} />
    </>
  );
}

const renderEditor = (target: EditorTarget | null, onClose = vi.fn()) => {
  render(<Harness target={target} onClose={onClose} />);
  return onClose;
};
const field = (name: string) => screen.getByRole("textbox", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("ActionEditor", () => {
  it("shows nothing without a target", () => {
    renderEditor(null);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("opens a New Action form with an Add button for a target without a source", () => {
    renderEditor(newAction);

    const dialog = screen.getByRole("dialog", { name: "New Action" });
    expect(within(dialog).getByRole("button", { name: "Add" })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("opens an Edit Action form with the action's values and a Save button", () => {
    renderEditor(existing);

    expect(screen.getByRole("dialog", { name: "Edit Action" })).toBeInTheDocument();
    expect(field("Title")).toHaveValue("Open in Editor");
    expect(field("App")).toHaveValue("Demo Editor");
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });

  describe("Runs as", () => {
    it("shows App and Arguments fields for an app", () => {
      renderEditor(newAction);

      expect(screen.getByRole("button", { name: "App" })).toHaveAttribute("aria-pressed", "true");
      expect(field("App")).toBeInTheDocument();
      expect(field("Arguments")).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "URL" })).not.toBeInTheDocument();
    });

    it.each([
      { choice: "Link", shown: "URL", hidden: ["App", "Command"] },
      { choice: "Terminal", shown: "Command", hidden: ["App", "URL"] },
      { choice: "Script", shown: "Command", hidden: ["App", "URL"] },
    ])("shows only the $shown field for $choice", async ({ choice, shown, hidden }) => {
      renderEditor(newAction);

      await userEvent.click(screen.getByRole("button", { name: choice }));

      expect(screen.getByRole("button", { name: choice })).toHaveAttribute("aria-pressed", "true");
      expect(field(shown)).toBeInTheDocument();
      for (const name of hidden) expect(screen.queryByRole("textbox", { name })).not.toBeInTheDocument();
    });

    it("explains the chosen kind", async () => {
      renderEditor(newAction);

      await userEvent.click(screen.getByRole("button", { name: "Script" }));

      expect(screen.getByText("Runs in the background; the output shows under Scripts.")).toBeInTheDocument();
    });
  });

  describe("saving", () => {
    it("adds a new action with the draft and a null source, then closes", async () => {
      const onClose = renderEditor(newAction);
      await userEvent.type(field("Title"), "Staging");
      await userEvent.click(screen.getByRole("button", { name: "Link" }));
      await userEvent.type(field("URL"), "https://staging.example.com");

      await userEvent.click(screen.getByRole("button", { name: "Add" }));

      expect(mocked.saveAction).toHaveBeenCalledWith("alpha", null, { runsAs: "url", label: "Staging", url: "https://staging.example.com" }, null);
      expect(onClose).toHaveBeenCalled();
    });

    it("saves a changed action at its source", async () => {
      const onClose = renderEditor(existing);
      await userEvent.clear(field("Title"));
      await userEvent.type(field("Title"), "Editor Now");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.saveAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 }, { runsAs: "app", label: "Editor Now", app: "Demo Editor" }, null);
      expect(onClose).toHaveBeenCalled();
    });

    it("submits with Enter in a field", async () => {
      renderEditor(newAction);

      await userEvent.type(field("Title"), "Quick{Enter}");

      expect(mocked.saveAction).toHaveBeenCalledWith("alpha", null, expect.objectContaining({ label: "Quick" }), null);
    });

    it("shows the refusal and keeps the form open", async () => {
      mocked.saveAction.mockRejectedValueOnce({ kind: "invalid_input", message: "Give the action a title", code: null });
      const onClose = renderEditor(newAction);

      await userEvent.click(screen.getByRole("button", { name: "Add" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Give the action a title");
      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog", { name: "New Action" })).toBeInTheDocument();
    });

    it("closes without saving on Cancel", async () => {
      const onClose = renderEditor(existing);

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(onClose).toHaveBeenCalled();
      expect(mocked.saveAction).not.toHaveBeenCalled();
    });
  });

  describe("a built-in", () => {
    it("explains that empty fields use the defaults, shown as placeholders, and hides Runs as", () => {
      renderEditor(builtin({ runsAs: "app", label: "" }));

      expect(screen.getByText("A built-in: empty fields use the default shown in gray.")).toBeInTheDocument();
      expect(screen.queryByRole("group", { name: "Runs as" })).not.toBeInTheDocument();
      expect(field("App")).toHaveAttribute("placeholder", "Zed");
      expect(field("Title")).toHaveAttribute("placeholder", "Editor");
    });

    it("offers no Reset to Default while nothing is overridden", () => {
      renderEditor(builtin({ runsAs: "app", label: "" }));

      expect(screen.queryByRole("button", { name: "Reset to Default" })).not.toBeInTheDocument();
    });

    it("clears the overrides with Reset to Default", async () => {
      renderEditor(builtin({ runsAs: "app", label: "", args: "--new-window" }));
      expect(field("Arguments")).toHaveValue("--new-window");

      await userEvent.click(screen.getByRole("button", { name: "Reset to Default" }));

      expect(field("Arguments")).toHaveValue("");
      expect(screen.queryByRole("button", { name: "Reset to Default" })).not.toBeInTheDocument();
    });

    it("saves the built-in with its preset", async () => {
      renderEditor(builtin({ runsAs: "app", label: "" }));
      await userEvent.type(field("Arguments"), "--new-window");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.saveAction).toHaveBeenCalledWith("alpha", { list: "apps", index: 0 }, { runsAs: "app", label: "", args: "--new-window" }, "editor");
    });
  });
});
