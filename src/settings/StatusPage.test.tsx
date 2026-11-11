import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type AppStatus } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { StatusPage } from "./StatusPage";

const mocked = vi.mocked(api, true);

const status = (patch: Partial<AppStatus> = {}): AppStatus => ({
  name: "Launcher",
  version: "1.2.3",
  debug: false,
  dataFolder: "~/Library/Application Support/com.example.launcher",
  logFolder: "~/Library/Logs/com.example.launcher",
  commandLine: { path: "~/.local/bin/launcher", link: "installed" },
  ...patch,
});

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.appStatus.mockResolvedValue(status());
});

describe("StatusPage", () => {
  it("shows the version, the release build and the folders with a ~ home", async () => {
    renderPage();

    expect(await screen.findByText("1.2.3")).toBeInTheDocument();
    expect(screen.getByText("Release")).toBeInTheDocument();
    expect(screen.getByText("~/Library/Application Support/com.example.launcher")).toBeInTheDocument();
    expect(screen.getByText("~/Library/Logs/com.example.launcher")).toBeInTheDocument();
  });

  it("shows a debug build as Debug", async () => {
    mocked.appStatus.mockResolvedValue(status({ debug: true }));
    renderPage();

    expect(await screen.findByText("Debug")).toBeInTheDocument();
  });

  it.each([
    [{ path: "~/.local/bin/launcher", link: "installed" as const }, "Installed at ~/.local/bin/launcher"],
    [{ path: "~/.local/bin/launcher", link: "notInstalled" as const }, "Not installed (~/.local/bin/launcher)"],
    [{ path: "~/.local/bin/launcher", link: "notApplicable" as const }, "Not linked for debug builds"],
  ])("describes the command line link %j", async (commandLine, text) => {
    mocked.appStatus.mockResolvedValue(status({ commandLine }));
    renderPage();

    expect(await screen.findByText(text)).toBeInTheDocument();
  });

  it("shows an alert when the status cannot be read", async () => {
    mocked.appStatus.mockRejectedValue({ kind: "io", message: "Permission denied", code: null });
    renderPage();

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not read the status: Permission denied");
  });
});
