import { beforeEach, describe, expect, it } from "vitest";
import type { ScriptResult } from "../api";
import { resetRuns, sourceKey, useRuns } from "./runs";

const result: ScriptResult = { ok: true, exitCode: 0, timedOut: false, output: "done", durationMs: 1200 };
const build = { list: "scripts", index: 0 } as const;
const lint = { list: "scripts", index: 1 } as const;

beforeEach(() => resetRuns());

describe("sourceKey", () => {
  it("tells actions apart by list and index", () => {
    expect(sourceKey({ list: "scripts", index: 1 })).toBe("scripts:1");
    expect(sourceKey({ list: "apps", index: 1 })).not.toBe(sourceKey({ list: "scripts", index: 1 }));
  });
});

describe("useRuns", () => {
  it("lists the scripts running in a project", () => {
    useRuns.getState().start("alpha", build);
    useRuns.getState().start("alpha", lint);
    useRuns.getState().start("beta", build);

    expect(useRuns.getState().running).toEqual({ alpha: ["scripts:0", "scripts:1"], beta: ["scripts:0"] });
  });

  it("finishing a script removes only its mark and keeps the last run", () => {
    useRuns.getState().start("alpha", build);
    useRuns.getState().start("alpha", lint);

    useRuns.getState().finish("alpha", build, { label: "Build site", result });

    const { running, last } = useRuns.getState();
    expect(running.alpha).toEqual(["scripts:1"]);
    expect(last.alpha).toEqual({ label: "Build site", result });
  });

  it("finishing without a run leaves the last run alone", () => {
    useRuns.getState().start("alpha", build);
    useRuns.getState().finish("alpha", build, { label: "Build site", result });
    useRuns.getState().start("alpha", lint);

    useRuns.getState().finish("alpha", lint);

    expect(useRuns.getState().running.alpha).toEqual([]);
    expect(useRuns.getState().last.alpha.label).toBe("Build site");
  });

  it("a newer run replaces the project's last run", () => {
    useRuns.getState().finish("alpha", build, { label: "Build site", result });
    useRuns.getState().finish("alpha", lint, { label: "Lint", result });

    expect(useRuns.getState().last.alpha.label).toBe("Lint");
  });

  it("dismissing removes one project's last run only", () => {
    useRuns.getState().finish("alpha", build, { label: "Build site", result });
    useRuns.getState().finish("beta", build, { label: "Build site", result });

    useRuns.getState().dismiss("alpha");

    expect(Object.keys(useRuns.getState().last)).toEqual(["beta"]);
  });

  it("resetRuns clears everything", () => {
    useRuns.getState().start("alpha", build);
    useRuns.getState().finish("alpha", lint, { label: "Lint", result });

    resetRuns();

    expect(useRuns.getState().running).toEqual({});
    expect(useRuns.getState().last).toEqual({});
  });
});
