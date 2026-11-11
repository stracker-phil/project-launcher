import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { Toasts } from "../components/Toasts";
import { AboutPage } from "./AboutPage";

const mocked = vi.mocked(api, true);

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AboutPage />
      <Toasts />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("AboutPage", () => {
  it("shows the app name and version", async () => {
    renderPage();

    expect(await screen.findByText("Version 1.2.3")).toBeInTheDocument();
    expect(screen.getByText("Launcher")).toBeInTheDocument();
  });

  it.each([
    ["Show Data Folder", "app:data"],
    ["Open Log Folder", "app:logs"],
  ])("runs %s as the command %s", async (name, command) => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name }));

    expect(mocked.runCommand).toHaveBeenCalledWith(command);
  });

  it.each([
    ["Show Data Folder", "Could not show the data folder: No such folder"],
    ["Open Log Folder", "Could not open the log folder: No such folder"],
  ])("shows an error toast when %s fails", async (name, message) => {
    mocked.runCommand.mockRejectedValue({ kind: "not_found", message: "No such folder", code: null });
    renderPage();

    await userEvent.click(screen.getByRole("button", { name }));

    expect(await screen.findByText(message)).toBeInTheDocument();
  });
});
