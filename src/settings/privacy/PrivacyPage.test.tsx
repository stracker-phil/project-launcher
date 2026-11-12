import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { Toasts } from "../../components/Toasts";
import { resetStore } from "../../state/store";
import { PrivacyPage } from "./PrivacyPage";

const mocked = vi.mocked(api, true);

function renderPage() {
  render(
    <>
      <PrivacyPage />
      <Toasts />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("PrivacyPage", () => {
  it("lists what Launcher keeps on this Mac and where", () => {
    renderPage();

    const storage = screen.getByRole("region", { name: "On this Mac" });
    expect(storage).toHaveTextContent("Settings and tags");
    expect(storage).toHaveTextContent("Project list");
    expect(storage).toHaveTextContent("Window and sidebar");
    expect(storage).toHaveTextContent("Project details");
    expect(storage).toHaveTextContent(".project-launcher.json");
  });

  it("says Launcher has no network code", () => {
    renderPage();

    const network = screen.getByRole("region", { name: "Network" });
    expect(network).toHaveTextContent("No network code");
    expect(network).toHaveTextContent("links open in your browser");
  });

  it("runs app:data when Show Data Folder is clicked", async () => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Show Data Folder" }));

    expect(mocked.runCommand).toHaveBeenCalledWith("app:data");
  });

  it("shows an error toast when Show Data Folder fails", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "not_found", message: "No such folder", code: null });
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Show Data Folder" }));

    expect(await screen.findByText("Could not show the data folder: No such folder")).toBeInTheDocument();
  });
});
