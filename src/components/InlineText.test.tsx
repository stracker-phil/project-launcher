import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { InlineText } from "./InlineText";

function renderText(props: Partial<React.ComponentProps<typeof InlineText>> = {}) {
  const onSave = vi.fn();
  render(<InlineText value="Demo Site" placeholder="Add a name" label="Name" onSave={onSave} {...props} />);
  return { onSave };
}

const nextFrame = () => act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));

const startEditing = async () => {
  await userEvent.click(screen.getByRole("button", { name: "Name" }));
  await nextFrame();
  return screen.getByRole<HTMLInputElement>("textbox", { name: "Name" });
};

describe("InlineText", () => {
  describe("reading", () => {
    it("shows the value as a button", () => {
      renderText();

      expect(screen.getByRole("button", { name: "Name" })).toHaveTextContent("Demo Site");
    });

    it("shows the placeholder while the value is empty", () => {
      renderText({ value: "" });

      expect(screen.getByRole("button", { name: "Name" })).toHaveTextContent("Add a name");
    });

    it("shows plain text without a button when read-only", () => {
      renderText({ readOnly: true });

      expect(screen.getByText("Demo Site")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });

    it("shows a dash for an empty read-only value", () => {
      renderText({ value: "", readOnly: true });

      expect(screen.getByText("—")).toBeInTheDocument();
      expect(screen.queryByText("Add a name")).not.toBeInTheDocument();
    });
  });

  describe("single line", () => {
    it("turns into a field with the value selected on click", async () => {
      renderText();

      const field = await startEditing();

      expect(field).toHaveValue("Demo Site");
      await waitFor(() => {
        expect(field.selectionStart).toBe(0);
        expect(field.selectionEnd).toBe("Demo Site".length);
      });
    });

    it("saves the text without trailing whitespace on Enter", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.clear(field);
      await userEvent.type(field, "Sample App   {Enter}");

      expect(onSave).toHaveBeenCalledWith("Sample App");
      expect(screen.getByRole("button", { name: "Name" })).toBeInTheDocument();
    });

    it("saves when the field loses focus", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.clear(field);
      await userEvent.type(field, "Sample App");
      const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(true);
      await userEvent.tab();
      hasFocus.mockRestore();

      expect(onSave).toHaveBeenCalledWith("Sample App");
    });

    it("saves an emptied value", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.clear(field);
      await userEvent.keyboard("{Enter}");

      expect(onSave).toHaveBeenCalledWith("");
    });

    it("does not save when the text is unchanged", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.type(field, "{Enter}");

      expect(onSave).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Name" })).toBeInTheDocument();
    });

    it("does not save when only trailing whitespace was added", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.type(field, "   {Enter}");

      expect(onSave).not.toHaveBeenCalled();
    });

    it("keeps the old value and saves nothing on Escape", async () => {
      const { onSave } = renderText();
      const field = await startEditing();

      await userEvent.clear(field);
      await userEvent.type(field, "Sample App{Escape}");

      expect(onSave).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Name" })).toHaveTextContent("Demo Site");
    });

    it("starts from the saved value again after a cancelled edit", async () => {
      renderText();
      const field = await startEditing();
      await userEvent.type(field, " draft{Escape}");

      const again = await startEditing();

      expect(again).toHaveValue("Demo Site");
    });
  });

  describe("focus", () => {
    const withList = () => render(<div role="listbox" aria-label="Projects" tabIndex={-1} />);

    it.each([
      { key: "{Enter}", why: "Enter" },
      { key: "{Escape}", why: "Escape" },
    ])("hands the keys to the project list after $why", async ({ key }) => {
      withList();
      renderText();
      await startEditing();

      await userEvent.keyboard(key);
      await nextFrame();

      expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
    });

    it("keeps the field open and saves nothing when it blurs while the document has no focus", async () => {
      const { onSave } = renderText();
      const field = await startEditing();
      await userEvent.type(field, " draft");
      const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(false);

      fireEvent.blur(field);

      expect(onSave).not.toHaveBeenCalled();
      expect(screen.getByRole("textbox", { name: "Name" })).toHaveValue("Demo Site draft");
      hasFocus.mockRestore();
    });

    it("does not save on Enter while an input method is composing", async () => {
      const { onSave } = renderText();
      const field = await startEditing();
      fireEvent.change(field, { target: { value: "Sample" } });

      fireEvent.keyDown(field, { key: "Enter", isComposing: true });

      expect(onSave).not.toHaveBeenCalled();
      expect(screen.getByRole("textbox", { name: "Name" })).toBeInTheDocument();
    });
  });

  describe("multiline", () => {
    const startNotes = async () => {
      await userEvent.click(screen.getByRole("button", { name: "Notes" }));
      await nextFrame();
      return screen.getByRole("textbox", { name: "Notes" });
    };

    it("adds a line on Enter without saving", async () => {
      const { onSave } = renderText({ multiline: true, label: "Notes", value: "First" });
      const field = await startNotes();

      await userEvent.type(field, "{Enter}Second");

      expect(field).toHaveValue("First\nSecond");
      expect(onSave).not.toHaveBeenCalled();
    });

    it("saves on ⌘Enter", async () => {
      const { onSave } = renderText({ multiline: true, label: "Notes", value: "First" });
      const field = await startNotes();

      await userEvent.type(field, "{Enter}Second");
      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      expect(onSave).toHaveBeenCalledWith("First\nSecond");
      expect(screen.getByRole("button", { name: "Notes" })).toBeInTheDocument();
    });

    it("keeps the old value on Escape", async () => {
      const { onSave } = renderText({ multiline: true, label: "Notes", value: "First" });
      const field = await startNotes();

      await userEvent.type(field, " more{Escape}");

      expect(onSave).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Notes" })).toHaveTextContent("First");
    });
  });

  describe("controlled editing", () => {
    it("shows the field while editing is on and reports Escape as the end of editing", async () => {
      const onEditingChange = vi.fn();
      renderText({ editing: true, onEditingChange });

      const field = screen.getByRole("textbox", { name: "Name" });
      await userEvent.type(field, "{Escape}");

      expect(onEditingChange).toHaveBeenLastCalledWith(false);
    });

    it("asks to start editing on click and stays a button until editing is turned on", async () => {
      const onEditingChange = vi.fn();
      const { onSave } = renderText({ editing: false, onEditingChange });

      await userEvent.click(screen.getByRole("button", { name: "Name" }));

      expect(onEditingChange).toHaveBeenCalledWith(true);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(onSave).not.toHaveBeenCalled();
    });

    it("switches to the field when the parent turns editing on", () => {
      const onSave = vi.fn();
      const { rerender } = render(<InlineText value="Demo Site" placeholder="Add a name" label="Name" onSave={onSave} editing={false} />);

      rerender(<InlineText value="Demo Site" placeholder="Add a name" label="Name" onSave={onSave} editing />);

      expect(screen.getByRole("textbox", { name: "Name" })).toHaveValue("Demo Site");
    });

    it("never edits when read-only, even if editing is on", () => {
      renderText({ editing: true, readOnly: true });

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });
  });
});
