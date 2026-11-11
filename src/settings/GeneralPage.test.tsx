import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { DEFAULT_SETTINGS } from "../test/apiMock";
import { GeneralPage } from "./GeneralPage";

const mocked = vi.mocked(api, true);
const diskFull = { kind: "io", message: "Disk full", code: null };

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <GeneralPage />
      <Toasts />
    </QueryClientProvider>,
  );
}

async function renderLoaded() {
  renderPage();
  await screen.findByDisplayValue("Demo Editor");
}

const loginSwitch =() => screen.getByRole("switch", { name: "Open at login" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, defaultEditor: "Demo Editor", gitClient: "", terminalApp: "Terminal", configEditor: "TextEdit" });
  mocked.setAppSetting.mockResolvedValue(undefined);
  mocked.autostartEnabled.mockResolvedValue(false);
  mocked.setAutostart.mockImplementation(async (on) => on);
});

describe("GeneralPage", () => {
  describe("apps", () => {
    it("shows the stored app of each setting", async () => {
      await renderLoaded();

      expect(screen.getByRole("textbox", { name: "Default editor" })).toHaveValue("Demo Editor");
      expect(screen.getByRole("textbox", { name: "Git client" })).toHaveValue("");
      expect(screen.getByRole("textbox", { name: "Terminal" })).toHaveValue("Terminal");
      expect(screen.getByRole("textbox", { name: "Project file editor" })).toHaveValue("TextEdit");
    });

    it.each([
      ["Default editor", "defaultEditor"],
      ["Git client", "gitClient"],
      ["Terminal", "terminalApp"],
      ["Project file editor", "configEditor"],
    ] as const)("saves the %s as %s when the field is left", async (label, setting) => {
      await renderLoaded();
      const field = screen.getByRole("textbox", { name: label });

      await userEvent.clear(field);
      await userEvent.type(field, "Demo App");
      await userEvent.tab();

      expect(mocked.setAppSetting).toHaveBeenCalledWith(setting, "Demo App");
    });

    it("saves on Enter", async () => {
      await renderLoaded();
      const field = screen.getByRole("textbox", { name: "Git client" });

      await userEvent.type(field, "Demo Git{Enter}");

      expect(mocked.setAppSetting).toHaveBeenCalledWith("gitClient", "Demo Git");
    });

    it("saves an emptied field so the setting is cleared", async () => {
      await renderLoaded();
      const field = screen.getByRole("textbox", { name: "Project file editor" });

      await userEvent.clear(field);
      await userEvent.tab();

      expect(mocked.setAppSetting).toHaveBeenCalledWith("configEditor", "");
    });

    it("saves nothing when the field is left unchanged", async () => {
      await renderLoaded();
      const field = screen.getByRole("textbox", { name: "Default editor" });

      await userEvent.click(field);
      await userEvent.tab();

      expect(mocked.setAppSetting).not.toHaveBeenCalled();
    });

    it("shows an error toast when saving fails", async () => {
      mocked.setAppSetting.mockRejectedValue(diskFull);
      await renderLoaded();
      const field = screen.getByRole("textbox", { name: "Git client" });

      await userEvent.type(field, "Demo Git{Enter}");

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change the git client: Disk full");
    });
  });

  describe("open at login", () => {
    it("is off when the login item is not installed", async () => {
      renderPage();

      expect(await screen.findByRole("switch", { name: "Open at login" })).toHaveAttribute("aria-checked", "false");
    });

    it("is on when the login item is installed, and a click turns it off", async () => {
      mocked.autostartEnabled.mockResolvedValue(true);
      renderPage();
      expect(await screen.findByRole("switch", { name: "Open at login", checked: true })).toBeInTheDocument();

      await userEvent.click(loginSwitch());

      expect(mocked.setAutostart).toHaveBeenCalledWith(false);
      expect(loginSwitch()).toHaveAttribute("aria-checked", "false");
    });

    it("turns on with a click and follows the state Rust reports", async () => {
      renderPage();
      expect(loginSwitch()).toHaveAttribute("aria-checked", "false");

      await userEvent.click(loginSwitch());

      expect(mocked.setAutostart).toHaveBeenCalledWith(true);
      expect(loginSwitch()).toHaveAttribute("aria-checked", "true");
    });

    it("stays as Rust reports it when the request is refused", async () => {
      mocked.setAutostart.mockResolvedValue(false);
      renderPage();

      await userEvent.click(loginSwitch());

      expect(loginSwitch()).toHaveAttribute("aria-checked", "false");
    });

    it("keeps its state and shows an error toast when the change fails", async () => {
      mocked.setAutostart.mockRejectedValue(diskFull);
      renderPage();

      await userEvent.click(loginSwitch());

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change Open at Login: Disk full");
      expect(loginSwitch()).toHaveAttribute("aria-checked", "false");
    });
  });
});
