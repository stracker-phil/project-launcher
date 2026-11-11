import { beforeEach, describe, expect, it } from "vitest";
import { resetLibrary, useLibrary } from "./library";

beforeEach(() => resetLibrary());

describe("requestNewAction", () => {
  it("holds no request at first", () => {
    expect(useLibrary.getState().newAction).toBeNull();
  });

  it("stores the draft as the first request", () => {
    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "dev", command: "npm run dev" });

    expect(useLibrary.getState().newAction).toEqual({ draft: { runsAs: "terminal", label: "dev", command: "npm run dev" }, nonce: 1 });
  });

  it("counts the nonce up when the same draft is requested again", () => {
    const draft = { runsAs: "terminal", label: "dev", command: "npm run dev" } as const;

    useLibrary.getState().requestNewAction(draft);
    useLibrary.getState().requestNewAction(draft);

    expect(useLibrary.getState().newAction).toEqual({ draft, nonce: 2 });
  });

  it("is cleared by takeNewAction", () => {
    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "dev" });

    useLibrary.getState().takeNewAction();

    expect(useLibrary.getState().newAction).toBeNull();
  });

  it("accepts a new request after the last one was taken", () => {
    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "dev" });
    useLibrary.getState().takeNewAction();

    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "build" });

    expect(useLibrary.getState().newAction?.draft.label).toBe("build");
  });

  it("starts counting from the beginning after a reset", () => {
    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "dev" });

    resetLibrary();
    useLibrary.getState().requestNewAction({ runsAs: "terminal", label: "build" });

    expect(useLibrary.getState().newAction?.nonce).toBe(1);
  });
});
