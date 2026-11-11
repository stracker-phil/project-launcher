import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Filter } from "../api";
import { resetLibrary, useLibrary } from "../state/library";
import type { createApiMock } from "../test/apiMock";
import { project } from "../test/library";
import { useFocusOnOpen, useReportCurrentProject, useSelectProjectRequests } from "./useCurrentProject";

const mocked = vi.mocked(api, true);
const select = (payload: { id: string; filter: Filter }) => act(() => (api as ReturnType<typeof createApiMock>).emit("select_project", payload));

const nextFrame = () => act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));

const alpha = project({ id: "alpha", name: "Alpha" });
const beta = project({ id: "beta", name: "Beta Shop" });

beforeEach(() => {
  vi.clearAllMocks();
  resetLibrary();
  mocked.setCurrentProject.mockResolvedValue(undefined);
});

describe("useReportCurrentProject", () => {
  it("tells Rust there is none at first", async () => {
    renderHook(() => useReportCurrentProject());

    await waitFor(() => expect(mocked.setCurrentProject).toHaveBeenCalledWith(null));
  });

  it("tells Rust which project the detail shows, as it changes", async () => {
    renderHook(() => useReportCurrentProject());

    act(() => useLibrary.getState().setCurrent(alpha));
    await waitFor(() => expect(mocked.setCurrentProject).toHaveBeenLastCalledWith("alpha"));

    act(() => useLibrary.getState().setCurrent(beta));
    await waitFor(() => expect(mocked.setCurrentProject).toHaveBeenLastCalledWith("beta"));

    act(() => useLibrary.getState().setCurrent(null));
    await waitFor(() => expect(mocked.setCurrentProject).toHaveBeenLastCalledWith(null));
  });

  it("keeps working when Rust rejects the report", async () => {
    mocked.setCurrentProject.mockRejectedValue({ kind: "io", message: "Unavailable", code: null });
    renderHook(() => useReportCurrentProject());

    act(() => useLibrary.getState().setCurrent(alpha));

    await waitFor(() => expect(mocked.setCurrentProject).toHaveBeenLastCalledWith("alpha"));
  });
});

describe("useSelectProjectRequests", () => {
  async function renderListening() {
    const view = renderHook(() => useSelectProjectRequests());
    await waitFor(() => expect(mocked.on).toHaveBeenCalledWith("select_project", expect.any(Function)));
    return view;
  }

  it.each<[string, Filter]>([
    ["all", { kind: "all" }],
    ["archived", { kind: "archived" }],
    ["tag", { kind: "tag", tag: "Client" }],
  ])("shows the %s filter Rust names and selects the project", async (_name, filter) => {
    useLibrary.setState({ filter: { kind: "starred" } });
    await renderListening();

    select({ id: "beta", filter });

    expect(useLibrary.getState().filter).toEqual(filter);
    expect(useLibrary.getState().selectedId).toBe("beta");
  });

  it("keeps the search while the named filter is already shown", async () => {
    useLibrary.setState({ filter: { kind: "tag", tag: "Client" }, search: "shop" });
    await renderListening();

    select({ id: "beta", filter: { kind: "tag", tag: "Client" } });

    expect(useLibrary.getState().search).toBe("shop");
    expect(useLibrary.getState().selectedId).toBe("beta");
  });

  it("clears the search when it switches the filter", async () => {
    useLibrary.setState({ search: "shop" });
    await renderListening();

    select({ id: "old", filter: { kind: "archived" } });

    expect(useLibrary.getState().search).toBe("");
  });

  it("stops listening when unmounted", async () => {
    const { unmount } = await renderListening();
    unmount();
    await act(async () => {});

    select({ id: "beta", filter: { kind: "all" } });

    expect(useLibrary.getState().selectedId).toBeNull();
  });

  it("gives the project list the focus when nothing has it", async () => {
    render(<div role="listbox" aria-label="Projects" tabIndex={0} />);
    await renderListening();

    select({ id: "beta", filter: { kind: "all" } });
    await nextFrame();

    expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
  });
});

describe("useFocusOnOpen", () => {
  const opened = () => act(() => (api as ReturnType<typeof createApiMock>).emit("window_opened", null));

  async function renderListening() {
    const view = renderHook(() => useFocusOnOpen());
    await waitFor(() => expect(mocked.on).toHaveBeenCalledWith("window_opened", expect.any(Function)));
    return view;
  }

  it("gives the project list the focus when the window opens and nothing has it", async () => {
    render(<div role="listbox" aria-label="Projects" tabIndex={0} />);
    await renderListening();

    opened();
    await nextFrame();

    expect(screen.getByRole("listbox", { name: "Projects" })).toHaveFocus();
  });

  it("leaves the focus with a field that has it", async () => {
    render(
      <>
        <div role="listbox" aria-label="Projects" tabIndex={0} />
        <input aria-label="Name" />
      </>,
    );
    screen.getByRole("textbox", { name: "Name" }).focus();
    await renderListening();

    opened();
    await nextFrame();

    expect(screen.getByRole("textbox", { name: "Name" })).toHaveFocus();
  });
});
