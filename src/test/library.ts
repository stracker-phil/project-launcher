import type { Action, Library, Project, ProjectMeta, Row, Section } from "../api";
import { EMPTY_LIBRARY } from "./apiMock";

/** A project that is listed, available and unlocked; `meta` overrides merge into the default meta. */
export function project(patch: Partial<Omit<Project, "meta">> & { meta?: Partial<ProjectMeta> } = {}): Project {
  const { meta, ...rest } = patch;
  return {
    id: "alpha",
    path: "~/Projects/alpha",
    name: "Alpha",
    addedAt: 1,
    lastOpenedAt: null,
    available: true,
    locked: false,
    fileError: null,
    meta: { icon: "Folder", color: "Blue", tag: null, notes: [], starred: false, archived: false, url: null, repoUrl: null, editor: null, ...meta },
    ...rest,
  };
}

export const row = (p: Project, subtitle = p.path): Row => ({ project: p, subtitle });

export const section = (title: string | null, ...projects: Project[]): Section => ({ title, rows: projects.map((p) => row(p)) });

/** A library showing `sections`, with the sidebar counts of their projects. */
export function library(sections: Section[], patch: Partial<Library> = {}): Library {
  const count = sections.reduce((n, s) => n + s.rows.length, 0);
  return {
    ...EMPTY_LIBRARY,
    countLabel: `${count} projects`,
    sidebar: { ...EMPTY_LIBRARY.sidebar, all: count },
    sections,
    emptyText: null,
    ...patch,
  };
}

/** A library whose sidebar shows tags Client (2 projects), Internal (1) and Ideas (0). */
export function libraryWithTags(patch: Partial<Library["sidebar"]> = {}): Library {
  return {
    ...EMPTY_LIBRARY,
    title: "All Projects",
    countLabel: "5 projects",
    sidebar: {
      all: 5,
      starred: 0,
      untagged: 2,
      archived: 1,
      tags: [
        { name: "Client", count: 2 },
        { name: "Internal", count: 1 },
        { name: "Ideas", count: 0 },
      ],
      ...patch,
    },
  };
}

/** An app or script of a project config. */
export function action(label: string, patch: Partial<Action> = {}, list: "apps" | "scripts" = "apps", index = 0): Action {
  return {
    label,
    icon: "Code",
    color: null,
    shortcut: null,
    runsAs: list === "apps" ? "app" : "script",
    app: null,
    args: null,
    url: null,
    command: null,
    hiddenStates: [],
    edit: { source: { list, index }, draft: { runsAs: list === "apps" ? "app" : "script", label }, preset: null, defaults: null },
    ...patch,
  };
}
