import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { resetStore, useStore } from "../state/store";
import { ConfirmHost, ConfirmSheet } from "./ConfirmSheet";

const question = { title: "Remove the project “Demo Site”?", text: "It stays on disk.", action: "Remove Project" };

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("ConfirmSheet", () => {
  function renderSheet() {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(<ConfirmSheet {...question} onCancel={onCancel} onConfirm={onConfirm} />);
    return { onCancel, onConfirm };
  }

  it("shows the title and text as an alert dialog", () => {
    renderSheet();

    const dialog = screen.getByRole("alertdialog", { name: question.title });
    expect(dialog).toHaveAccessibleDescription(question.text);
    expect(screen.getByRole("button", { name: "Remove Project" })).toBeInTheDocument();
  });

  it("cancels from the Cancel button", async () => {
    const { onCancel, onConfirm } = renderSheet();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onCancel).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("cancels on Escape", async () => {
    const { onCancel, onConfirm } = renderSheet();

    await userEvent.keyboard("{Escape}");

    expect(onCancel).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("confirms from the action button", async () => {
    const { onCancel, onConfirm } = renderSheet();

    await userEvent.click(screen.getByRole("button", { name: "Remove Project" }));

    expect(onConfirm).toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });
});

describe("ConfirmHost", () => {
  const ask = () => {
    let answer!: Promise<boolean>;
    act(() => {
      answer = useStore.getState().confirm(question);
    });
    return answer;
  };

  it("shows nothing while no question is open", () => {
    render(<ConfirmHost />);

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("shows the asked question and resolves true when the action button is clicked", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    expect(await screen.findByRole("alertdialog", { name: question.title })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Remove Project" }));

    await expect(answer).resolves.toBe(true);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("resolves false and closes when Cancel is clicked", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    await expect(answer).resolves.toBe(false);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("resolves false and closes on Escape", async () => {
    render(<ConfirmHost />);
    const answer = ask();
    await screen.findByRole("alertdialog");

    await userEvent.keyboard("{Escape}");

    await expect(answer).resolves.toBe(false);
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("answers the first question false when a second is asked, and shows the second", async () => {
    render(<ConfirmHost />);
    const first = ask();
    let second!: Promise<boolean>;
    act(() => {
      second = useStore.getState().confirm({ title: "Remove the project “Sample App”?", text: "Gone.", action: "Remove Sample" });
    });

    await expect(first).resolves.toBe(false);
    expect(screen.getByRole("alertdialog", { name: "Remove the project “Sample App”?" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Remove Sample" }));
    await expect(second).resolves.toBe(true);
  });
});
