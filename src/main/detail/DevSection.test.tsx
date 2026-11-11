import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type DatabaseInfo, type DevStatus, type DockerStatus, type ServerStatus } from "../../api";
import { Toasts } from "../../components/Toasts";
import { resetStore } from "../../state/store";
import { EMPTY_DEV_STATUS } from "../../test/apiMock";
import { project } from "../../test/library";
import { DevSection } from "./DevSection";

const mocked = vi.mocked(api, true);

const alpha = project({ id: "alpha", name: "Alpha" });

const docker = (patch: Partial<DockerStatus> = {}): DockerStatus => ({ running: true, activity: null, manager: "OrbStack", ...patch });
const server = (patch: Partial<ServerStatus> = {}): ServerStatus => ({
  server: "ddev",
  label: "DDEV",
  state: "running",
  url: "https://alpha.ddev.site",
  detail: null,
  activity: null,
  ...patch,
});
const database = (patch: Partial<DatabaseInfo> = {}): DatabaseInfo => ({ source: "ddev", engine: "MariaDB", ...patch });

function renderSection(status: Partial<DevStatus> | undefined) {
  return render(
    <>
      <DevSection project={alpha} status={status && { ...EMPTY_DEV_STATUS, ...status }} />
      <Toasts />
    </>,
  );
}

const button = (name: string) => screen.getByRole("button", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("DevSection", () => {
  it.each([
    { why: "the status has not loaded", status: undefined },
    { why: "the project has no Docker, server or database", status: {} },
  ])("renders nothing when $why", ({ status }) => {
    renderSection(status);

    expect(screen.queryByRole("heading", { name: "Dev Environment" })).not.toBeInTheDocument();
  });

  describe("Docker row", () => {
    it("shows the manager's name and that Docker is running, offering Stop and Restart", () => {
      renderSection({ docker: docker() });

      expect(screen.getByRole("heading", { name: "Dev Environment" })).toBeInTheDocument();
      expect(screen.getByText("OrbStack")).toBeInTheDocument();
      expect(screen.getByText("Running")).toBeInTheDocument();
      expect(button("Stop")).toBeEnabled();
      expect(button("Restart")).toBeEnabled();
    });

    it("shows that Docker is stopped, offering Start and a disabled Restart", () => {
      renderSection({ docker: docker({ running: false }) });

      expect(screen.getByText("Stopped")).toBeInTheDocument();
      expect(button("Start")).toBeEnabled();
      expect(button("Restart")).toBeDisabled();
    });

    it.each([
      { running: false, name: "Start", action: "dockerStart" },
      { running: true, name: "Stop", action: "dockerStop" },
    ] as const)("sends $action when $name is clicked", async ({ running, name, action }) => {
      renderSection({ docker: docker({ running }) });

      await userEvent.click(button(name));

      expect(mocked.devAction).toHaveBeenCalledWith("alpha", action);
    });

    it("sends dockerRestart when Restart is clicked", async () => {
      renderSection({ docker: docker() });

      await userEvent.click(button("Restart"));

      expect(mocked.devAction).toHaveBeenCalledWith("alpha", "dockerRestart");
    });

    it.each([
      { activity: "starting", text: "Starting…", running: false },
      { activity: "stopping", text: "Stopping…", running: true },
      { activity: "restarting", text: "Restarting…", running: true },
    ] as const)("shows $text and disables both buttons while $activity", ({ activity, text, running }) => {
      renderSection({ docker: docker({ activity, running }) });

      expect(screen.getByText(text)).toBeInTheDocument();
      expect(screen.getAllByRole("button").every((b) => b.hasAttribute("disabled"))).toBe(true);
    });

    it("shows an error toast when the action is refused", async () => {
      mocked.devAction.mockRejectedValue({ kind: "io", message: "Docker is not installed", code: null });
      renderSection({ docker: docker({ running: false }) });

      await userEvent.click(button("Start"));

      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change the dev environment: Docker is not installed");
    });
  });

  describe("server row", () => {
    it("shows the server's label, state and URL", () => {
      renderSection({ server: server() });

      expect(screen.getByText("DDEV")).toBeInTheDocument();
      expect(screen.getByText("Running")).toBeInTheDocument();
      expect(button("https://alpha.ddev.site")).toBeInTheDocument();
    });

    it("opens the URL through api.openUrl", async () => {
      renderSection({ server: server() });

      await userEvent.click(button("https://alpha.ddev.site"));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://alpha.ddev.site");
    });

    it.each([
      { state: "paused", text: "Paused", toggle: "Start" },
      { state: "stopped", text: "Stopped", toggle: "Start" },
      { state: "unknown", text: "Unknown", toggle: "Start" },
    ] as const)("shows $text for a $state server with Start enabled", ({ state, text, toggle }) => {
      renderSection({ server: server({ state, url: null }) });

      expect(screen.getByText(text)).toBeInTheDocument();
      expect(button(toggle)).toBeEnabled();
      expect(button("Restart")).toBeDisabled();
    });

    it("shows no URL button for a server without a URL", () => {
      renderSection({ server: server({ state: "stopped", url: null }) });

      expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Start", "Restart"]);
    });

    it.each([
      { state: "stopped", name: "Start", action: "serverStart" },
      { state: "running", name: "Stop", action: "serverStop" },
    ] as const)("sends $action when $name is clicked", async ({ state, name, action }) => {
      renderSection({ server: server({ state }) });

      await userEvent.click(button(name));

      expect(mocked.devAction).toHaveBeenCalledWith("alpha", action);
    });

    it("sends serverRestart when Restart is clicked", async () => {
      renderSection({ server: server() });

      await userEvent.click(button("Restart"));

      expect(mocked.devAction).toHaveBeenCalledWith("alpha", "serverRestart");
    });

    it("shows what is missing for a server that is not set up and disables Start", () => {
      renderSection({ server: server({ state: "notConfigured", url: null, detail: "Run ddev config first" }) });

      expect(screen.getByText("Run ddev config first")).toBeInTheDocument();
      expect(button("Start")).toBeDisabled();
    });

    it("says Not set up for a server that is not set up and gives no reason", () => {
      renderSection({ server: server({ state: "notConfigured", url: null, detail: null }) });

      expect(screen.getByText("Not set up")).toBeInTheDocument();
    });

    it("shows Starting… and disables its buttons while the server starts", () => {
      renderSection({ server: server({ state: "stopped", activity: "starting" }) });

      expect(screen.getByText("Starting…")).toBeInTheDocument();
      expect(button("Start")).toBeDisabled();
      expect(button("Restart")).toBeDisabled();
    });

    it("keeps the Docker row's buttons apart from the server's", async () => {
      renderSection({ docker: docker({ running: false }), server: server({ state: "stopped", url: null }) });

      await userEvent.click(screen.getAllByRole("button", { name: "Start" })[1]);

      expect(mocked.devAction).toHaveBeenCalledWith("alpha", "serverStart");
    });
  });

  describe("database row", () => {
    it("shows the engine and its detail", () => {
      renderSection({ database: database({ detail: "db@127.0.0.1/alpha" }) });

      expect(screen.getByText("Database")).toBeInTheDocument();
      expect(screen.getByText("MariaDB")).toBeInTheDocument();
      expect(screen.getByText("db@127.0.0.1/alpha")).toBeInTheDocument();
    });

    it("opens the database in DataBee", async () => {
      renderSection({ database: database() });

      await userEvent.click(button("Open in DataBee"));

      expect(mocked.openDatabase).toHaveBeenCalledWith("alpha");
    });

    it("shows why the database can't be opened and disables the button", () => {
      renderSection({ database: database({ error: "DATABASE_URL is not set" }) });

      expect(screen.getByText("DATABASE_URL is not set")).toBeInTheDocument();
      expect(button("Open in DataBee")).toBeDisabled();
    });
  });
});
