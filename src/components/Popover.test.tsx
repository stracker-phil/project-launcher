import { render, screen } from "@testing-library/react";
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

  it("ignores Escape and presses while closed", async () => {
    const onClose = vi.fn();
    render(<Harness open={false} onClose={onClose} />);

    await userEvent.keyboard("{Escape}");
    await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

    expect(onClose).not.toHaveBeenCalled();
  });
});
