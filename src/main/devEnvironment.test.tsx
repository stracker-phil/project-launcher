import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { resetRuns } from "../state/runs";
import { resetStore } from "../state/store";
import { EMPTY_PROJECT_CONFIG, EMPTY_STATUS, type createApiMock } from "../test/apiMock";
import { project } from "../test/library";
import { ActionsSection } from "./detail/ActionsSection";
import { devAction, openDatabase, useDevFailures } from "./devEnvironment";

const mocked = vi.mocked(api, true);
const emit = (api as ReturnType<typeof createApiMock>).emit;

const alpha = project({ id: "alpha", name: "Alpha" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetRuns();
});

describe("devAction", () => {
  it("sends the action for the project", async () => {
    await devAction(alpha, "serverRestart");

    expect(mocked.devAction).toHaveBeenCalledWith("alpha", "serverRestart");
  });

  it("shows an error toast when Rust refuses", async () => {
    mocked.devAction.mockRejectedValue({ kind: "io", message: "Docker is not installed", code: null });
    render(<Toasts />);

    await devAction(alpha, "dockerStart");

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change the dev environment: Docker is not installed");
  });
});

describe("openDatabase", () => {
  it("opens the project's database without a toast when DataBee opens at once", async () => {
    render(<Toasts />);

    await openDatabase(alpha);

    expect(mocked.openDatabase).toHaveBeenCalledWith("alpha");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("says the dev server starts first when DataBee waits for it", async () => {
    mocked.openDatabase.mockResolvedValue("starting");
    render(<Toasts />);

    await openDatabase(alpha);

    expect(await screen.findByRole("status")).toHaveTextContent("Starting the dev server first; DataBee opens once it runs.");
  });

  it("shows an error toast when the database can't be opened", async () => {
    mocked.openDatabase.mockRejectedValue({ kind: "invalid_input", message: "DATABASE_URL is not set", code: null });
    render(<Toasts />);

    await openDatabase(alpha);

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t open the database: DATABASE_URL is not set");
  });
});

describe("useDevFailures", () => {
  function Host() {
    useDevFailures();
    return (
      <>
        <ActionsSection project={alpha} config={EMPTY_PROJECT_CONFIG} status={EMPTY_STATUS} editable />
        <Toasts />
      </>
    );
  }

  it("shows a toast naming the failed step", () => {
    render(<Host />);

    act(() => emit("dev_failed", { projectId: "alpha", label: "Start DDEV", output: "port 80 is in use" }));

    expect(screen.getByRole("alert")).toHaveTextContent("Start DDEV failed");
  });

  it("shows the step's output as the project's Last Run", () => {
    render(<Host />);

    act(() => emit("dev_failed", { projectId: "alpha", label: "Start DDEV", output: "port 80 is in use" }));

    const region = screen.getByRole("region", { name: "Last Run" });
    expect(region).toHaveTextContent("Start DDEV");
    expect(region).toHaveTextContent("Failed");
    expect(region).toHaveTextContent("port 80 is in use");
  });

  it("keeps the output of another project out of this project's Last Run", () => {
    render(<Host />);

    act(() => emit("dev_failed", { projectId: "beta", label: "Start DDEV", output: "port 80 is in use" }));

    expect(screen.queryByRole("region", { name: "Last Run" })).not.toBeInTheDocument();
  });
});
