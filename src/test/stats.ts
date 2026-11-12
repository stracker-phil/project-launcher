import type { FileStats, GitStats, QuickStats } from "../api";
import { EMPTY_FILE_STATS, EMPTY_QUICK_STATS } from "./apiMock";

/** Quick stats of a folder with nothing found; `patch` overrides. */
export const quickStats = (patch: Partial<QuickStats> = {}): QuickStats => ({ ...EMPTY_QUICK_STATS, ...patch });

/** File stats of an empty folder; `patch` overrides. */
export const fileStats = (patch: Partial<FileStats> = {}): FileStats => ({ ...EMPTY_FILE_STATS, ...patch });

/** A clean checkout on `main` with an upstream it is level with and no tags; `patch` overrides. */
export const gitStats = (patch: Partial<GitStats> = {}): GitStats => ({
  branch: "main",
  hasUpstream: true,
  ahead: 0,
  behind: 0,
  modified: 0,
  untracked: 0,
  lastCommit: null,
  commits: 0,
  started: null,
  latestTag: null,
  weekly: [],
  ...patch,
});
