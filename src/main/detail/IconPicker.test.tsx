import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Project } from "../../api";
import { PROJECT_COLORS } from "../../lib/icons";
import { resetStore, useStore } from "../../state/store";
import { project } from "../../test/library";
import { IconPicker } from "./IconPicker";

const mocked = vi.mocked(api, true);

function Harness({ subject, readOnly = false, startOpen = false }: { subject: Project; readOnly?: boolean; startOpen?: boolean }) {
  const [open, setOpen] = useState(startOpen);
  return <IconPicker project={subject} open={open} onOpenChange={setOpen} readOnly={readOnly} />;
}

const picker = () => screen.getByRole("dialog", { name: "Icon and Color" });

beforeEach(() => {
  vi.clearAllMocks();
  mocked.setProjectMeta.mockResolvedValue(undefined);
  resetStore();
});

describe("IconPicker", () => {
  it("opens colours and icons when the tile is clicked", async () => {
    render(<Harness subject={project()} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Change Icon & Color" }));

    expect(picker()).toBeInTheDocument();
    expect(within(picker()).getByRole("button", { name: "Rocket" })).toBeInTheDocument();
  });

  it("closes again when the tile is clicked a second time", async () => {
    render(<Harness subject={project()} />);
    const tile = screen.getByRole("button", { name: "Change Icon & Color" });

    await userEvent.click(tile);
    await userEvent.click(tile);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("sets the colour when a swatch is clicked", async () => {
    const subject = project();
    render(<Harness subject={subject} startOpen />);

    await userEvent.click(within(picker()).getByRole("button", { name: "Green" }));

    expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "color", value: "Green" });
  });

  it("sets the icon when an icon is clicked", async () => {
    const subject = project();
    render(<Harness subject={subject} startOpen />);

    await userEvent.click(within(picker()).getByRole("button", { name: "Rocket" }));

    expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "icon", value: "Rocket" });
  });

  it("shows an error toast when the change is rejected", async () => {
    mocked.setProjectMeta.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    render(<Harness subject={project()} startOpen />);

    await userEvent.click(within(picker()).getByRole("button", { name: "Rocket" }));

    await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Couldn’t update project: Disk full"));
  });

  it("marks the current colour and icon as pressed", () => {
    render(<Harness subject={project({ meta: { icon: "Rocket", color: "Green" } })} startOpen />);

    expect(within(picker()).getByRole("button", { name: "Green", pressed: true })).toBeInTheDocument();
    expect(within(picker()).getByRole("button", { name: "Blue", pressed: false })).toBeInTheDocument();
    expect(within(picker()).getByRole("button", { name: "Rocket", pressed: true })).toBeInTheDocument();
    expect(within(picker()).getByRole("button", { name: "Folder", pressed: false })).toBeInTheDocument();
  });

  it("lists a current icon the picker does not offer first", () => {
    render(<Harness subject={project({ meta: { icon: "Unlisted" } })} startOpen />);

    const icons = within(picker()).getAllByRole("button").slice(PROJECT_COLORS.length);
    expect(icons[0]).toHaveAccessibleName("Unlisted");
    expect(icons[0]).toHaveAttribute("aria-pressed", "true");
    expect(icons[1]).toHaveAccessibleName("Folder");
  });

  it("starts the icon list with the first offered icon when the current one is offered", () => {
    render(<Harness subject={project({ meta: { icon: "Rocket" } })} startOpen />);

    const icons = within(picker()).getAllByRole("button").slice(PROJECT_COLORS.length);
    expect(icons[0]).toHaveAccessibleName("Folder");
  });

  it("offers no button when read-only", () => {
    render(<Harness subject={project({ locked: true })} readOnly />);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
