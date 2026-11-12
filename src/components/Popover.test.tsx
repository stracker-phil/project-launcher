import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Popover } from "./Popover";

function Harness({ open, onClose }: { open: boolean; onClose: () => void }) {
  const anchor = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button ref={anchor} type="button">Opener</button>
      <button type="button">Elsewhere</button>
      <Popover open={open} anchor={anchor} label="Options" onClose={onClose}>
        <button type="button">Inside</button>
      </Popover>
    </>
  );
}

describe("Popover", () => {
  it("shows its children in a labelled dialog while open", () => {
    render(<Harness open onClose={vi.fn()} />);

    expect(screen.getByRole("dialog", { name: "Options" })).toContainElement(screen.getByRole("button", { name: "Inside" }));
  });

  it("renders nothing while closed", () => {
    render(<Harness open={false} onClose={vi.fn()} />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inside" })).not.toBeInTheDocument();
  });

  it("closes on Escape and gives the focus back to the anchor", async () => {
    const onClose = vi.fn();
    render(<Harness open onClose={onClose} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "Inside" })).toHaveFocus());

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Opener" })).toHaveFocus();
  });

  it("stays open on Escape inside an element that is recording a key", async () => {
    const onClose = vi.fn();
    render(
      <Popover open anchor={{ current: null }} label="Options" onClose={onClose}>
        <button type="button" data-recording>Recording</button>
        <button type="button">Inside</button>
      </Popover>,
    );
    await waitFor(() => expect(screen.getByRole("button", { name: "Recording" })).toHaveFocus());

    screen.getByRole("button", { name: "Recording" }).focus();
    await userEvent.keyboard("{Escape}");
    expect(onClose).not.toHaveBeenCalled();

    screen.getByRole("button", { name: "Inside" }).focus();
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
  });

  it("closes on a press outside", async () => {
    const onClose = vi.fn();
    render(<Harness open onClose={onClose} />);

    await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("stays open on a press inside", async () => {
    const onClose = vi.fn();
    render(<Harness open onClose={onClose} />);

    await userEvent.click(screen.getByRole("button", { name: "Inside" }));

    expect(onClose).not.toHaveBeenCalled();
  });

  it("leaves a press on the anchor to the anchor", async () => {
    const onClose = vi.fn();
    render(<Harness open onClose={onClose} />);

    await userEvent.click(screen.getByRole("button", { name: "Opener" }));

    expect(onClose).not.toHaveBeenCalled();
  });

  describe("focus", () => {
    const nextFrame = () => act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));

    function Choices({ open, anchorIsButton = true }: { open: boolean; anchorIsButton?: boolean }) {
      const anchor = useRef<HTMLButtonElement & HTMLSpanElement>(null);
      return (
        <>
          {anchorIsButton ? <button ref={anchor} type="button">Opener</button> : <span ref={anchor}>Label</span>}
          <div role="listbox" aria-label="Projects" tabIndex={0} />
          <Popover open={open} anchor={anchor} label="Options" onClose={vi.fn()}>
            <button type="button">First</button>
            <button type="button" aria-pressed="true">Chosen</button>
          </Popover>
        </>
      );
    }

    it("puts the focus on the pressed choice when it opens", async () => {
      render(<Choices open />);
      await nextFrame();

      expect(screen.getByRole("button", { name: "Chosen" })).toHaveFocus();
    });

    it("puts the focus on the first focusable element when nothing is pressed", async () => {
      render(
        <Popover open anchor={{ current: null }} label="Options" onClose={vi.fn()}>
          <button type="button">First</button>
          <button type="button" aria-pressed="false">Second</button>
        </Popover>,
      );
      await nextFrame();

      expect(screen.getByRole("button", { name: "First" })).toHaveFocus();
    });

    it("gives the focus back to the anchor when it closes with the focus inside", async () => {
      const { rerender } = render(<Choices open />);
      await nextFrame();

      rerender(<Choices open={false} />);

      expect(screen.getByRole("button", { name: "Opener" })).toHaveFocus();
    });

    it("gives the focus to the project list when the anchor cannot take it", async () => {
      const { rerender } = render(<Choices open anchorIsButton={false} />);
      await nextFrame();

      rerender(<Choices open={false} anchorIsButton={false} />);
      await nextFrame();

      expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
    });

    it("leaves the focus where it is when it closes with the focus outside", async () => {
      const { rerender } = render(<Choices open />);
      await nextFrame();
      screen.getByRole("listbox", { name: "Projects" }).focus();

      rerender(<Choices open={false} />);

      expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
    });
  });

  it("ignores Escape and presses while closed", async () => {
    const onClose = vi.fn();
    render(<Harness open={false} onClose={onClose} />);

    await userEvent.keyboard("{Escape}");
    await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

    expect(onClose).not.toHaveBeenCalled();
  });
});
