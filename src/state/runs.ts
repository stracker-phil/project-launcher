import { create } from "zustand";
import type { ActionSource, ScriptResult } from "../api";

/** A script's last run in a project, shown under its scripts until dismissed. */
export interface LastRun {
  label: string;
  result: ScriptResult;
}

interface Runs {
  /** Per project: the scripts running now, as `sourceKey`s. */
  running: Record<string, string[]>;
  last: Record<string, LastRun>;
  start: (projectId: string, source: ActionSource) => void;
  finish: (projectId: string, source: ActionSource, run?: LastRun) => void;
  dismiss: (projectId: string) => void;
}

export const sourceKey = (source: ActionSource) => `${source.list}:${source.index}`;

/** Scripts that run and the last one's output: view state, so it lives in the page (ADR 0001). */
export const useRuns = create<Runs>()((set) => ({
  running: {},
  last: {},
  start: (projectId, source) =>
    set((s) => ({ running: { ...s.running, [projectId]: [...(s.running[projectId] ?? []), sourceKey(source)] } })),
  finish: (projectId, source, run) =>
    set((s) => {
      const running = { ...s.running, [projectId]: (s.running[projectId] ?? []).filter((k) => k !== sourceKey(source)) };
      if (!run) return { running };
      return { running, last: { ...s.last, [projectId]: run } };
    }),
  dismiss: (projectId) =>
    set((s) => {
      const last = { ...s.last };
      delete last[projectId];
      return { last };
    }),
}));

export function resetRuns() {
  useRuns.setState({ running: {}, last: {} });
}
