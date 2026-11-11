import { create } from "zustand";
import type { Filter, Project } from "../api";

/** The detail field being edited in place, from a menu or a command. */
export type EditField = "name" | "notes" | "tag" | "icon";

/** The sidebar's tag row being edited: a new tag at the end of the list, or a rename. */
export type TagEdit = { mode: "new" } | { mode: "rename"; tag: string };

export interface LibraryState {
  filter: Filter;
  search: string;
  selectedId: string | null;
  /** The project last selected under each filter, by `filterKey`. */
  lastSelected: Record<string, string>;
  editing: { id: string; field: EditField } | null;
  tagEdit: TagEdit | null;
  /** Counts up to ask the search field for the focus. */
  searchFocus: number;
  /** The project the detail shows, for menu commands; set by the list. */
  current: Project | null;

  /** Clears the search and selects the project last selected there. */
  setFilter: (filter: Filter) => void;
  setSearch: (search: string) => void;
  select: (id: string | null) => void;
  edit: (id: string, field: EditField) => void;
  stopEditing: () => void;
  setTagEdit: (edit: TagEdit | null) => void;
  focusSearch: () => void;
  setCurrent: (project: Project | null) => void;
}

export const ALL: Filter = { kind: "all" };

export function filterKey(filter: Filter): string {
  return filter.kind === "tag" ? `tag:${filter.tag}` : filter.kind;
}

export const sameFilter = (a: Filter, b: Filter) => filterKey(a) === filterKey(b);

export const useLibrary = create<LibraryState>()((set, get) => ({
  filter: ALL,
  search: "",
  selectedId: null,
  lastSelected: {},
  editing: null,
  tagEdit: null,
  searchFocus: 0,
  current: null,

  setFilter: (filter) => set({ filter, search: "", selectedId: get().lastSelected[filterKey(filter)] ?? null }),
  setSearch: (search) => set({ search }),
  select: (id) =>
    set((s) => ({
      selectedId: id,
      lastSelected: id ? { ...s.lastSelected, [filterKey(s.filter)]: id } : s.lastSelected,
      editing: s.editing && s.editing.id !== id ? null : s.editing,
    })),
  edit: (id, field) => set({ editing: { id, field } }),
  stopEditing: () => set({ editing: null }),
  setTagEdit: (tagEdit) => set({ tagEdit }),
  focusSearch: () => set((s) => ({ searchFocus: s.searchFocus + 1 })),
  setCurrent: (current) => set({ current }),
}));

const initialState = useLibrary.getState();

/** Restores the initial state, then applies `patch`; for tests. */
export function resetLibrary(patch: Partial<LibraryState> = {}) {
  useLibrary.setState({ ...initialState, ...patch }, true);
}
