import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resetStore, useStore } from "../state/store";
import type { createApiMock } from "../test/apiMock";
import { useEditRequests } from "./useEditRequests";

const mocked = vi.mocked(api, true);
const emit = (api as ReturnType<typeof createApiMock>).emit;

const execCommand = vi.fn();
const original = document.execCommand;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.undo.mockResolvedValue(null);
  mocked.redo.mockResolvedValue(null);
  document.execCommand = execCommand;
  renderHook(() => useEditRequests());
});

afterEach(() => {
  document.execCommand = original;
});

describe("useEditRequests", () => {
  describe("with a text field focused", () => {
    beforeEach(async () => {
      render(<input aria-label="Project name" />);
      await userEvent.click(screen.getByRole("textbox", { name: "Project name" }));
    });

    it.each(["undo", "redo"] as const)("runs %s in the field and leaves the data alone", (kind) => {
      act(() => emit("edit_request", kind));

      expect(execCommand).toHaveBeenCalledWith(kind);
      expect(mocked.undo).not.toHaveBeenCalled();
      expect(mocked.redo).not.toHaveBeenCalled();
    });
  });

  describe("without a text field focused", () => {
    it("undoes the last change of the data on an undo request", () => {
      act(() => emit("edit_request", "undo"));

      expect(mocked.undo).toHaveBeenCalled();
      expect(mocked.redo).not.toHaveBeenCalled();
      expect(execCommand).not.toHaveBeenCalled();
    });

    it("redoes the last undone change on a redo request", () => {
      act(() => emit("edit_request", "redo"));

      expect(mocked.redo).toHaveBeenCalled();
      expect(mocked.undo).not.toHaveBeenCalled();
      expect(execCommand).not.toHaveBeenCalled();
    });

    it("treats a focused checkbox like no text field", async () => {
      render(<input type="checkbox" aria-label="Starred" />);
      await userEvent.click(screen.getByRole("checkbox", { name: "Starred" }));

      act(() => emit("edit_request", "undo"));

      expect(mocked.undo).toHaveBeenCalled();
      expect(execCommand).not.toHaveBeenCalled();
    });

    it.each([
      ["undo", "undo"],
      ["redo", "redo"],
    ] as const)("shows an error toast when the %s fails", async (kind, method) => {
      mocked[method].mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      act(() => emit("edit_request", kind));

      await waitFor(() =>
        expect(useStore.getState().toasts).toEqual([expect.objectContaining({ kind: "error", message: `Could not ${kind}: Disk full` })]),
      );
    });
  });
});
