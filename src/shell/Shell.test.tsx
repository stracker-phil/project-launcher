import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { resetStore, useStore } from "../state/store";
import { Shell } from "./Shell";

const sidebar = () => screen.queryByRole("navigation", { name: "Sidebar" });
const list = () => screen.queryByRole("region", { name: "Projects" });
const detail = () => screen.queryByRole("main", { name: "Project" });
const inspector = () => screen.queryByRole("complementary", { name: "Inspector" });

beforeEach(() => {
  resetStore();
});

describe("Shell", () => {
  it("shows the sidebar, the project list and the detail, without the inspector, at first", () => {
    render(<Shell />);

    expect(sidebar()).toBeInTheDocument();
    expect(list()).toBeInTheDocument();
    expect(detail()).toBeInTheDocument();
    expect(inspector()).not.toBeInTheDocument();
  });

  it("shows the inspector once it is toggled open, and hides it when toggled again", () => {
    render(<Shell />);

    act(() => useStore.getState().toggleInspector());
    expect(inspector()).toBeInTheDocument();

    act(() => useStore.getState().toggleInspector());
    expect(inspector()).not.toBeInTheDocument();
  });

  it("shows only the detail and the inspector when the sidebar is hidden", () => {
    render(<Shell />);

    act(() => useStore.getState().toggleSidebar());

    expect(sidebar()).not.toBeInTheDocument();
    expect(list()).not.toBeInTheDocument();
    expect(detail()).toBeInTheDocument();
    expect(inspector()).toBeInTheDocument();
  });

  it("brings the sidebar and the list back when the sidebar is shown again", () => {
    resetStore({ sidebarHidden: true });
    render(<Shell />);

    act(() => useStore.getState().toggleSidebar());

    expect(sidebar()).toBeInTheDocument();
    expect(list()).toBeInTheDocument();
  });
});
