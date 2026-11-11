import { create } from "zustand";
import { api } from "../api";
import type { MenuAt, MenuEntry, MenuOptions, OpenMenu } from "./menuTypes";

export interface Toast {
  id: number;
  kind: "error" | "info" | "success";
  message: string;
  /** How many times the same message was reported while it was showing. */
  count: number;
  /** A button in the toast, e.g. Undo; running it dismisses the toast. */
  action?: ToastAction;
}

export interface ToastAction {
  label: string;
  run: () => void;
  /** Overrides how long the toast stays. */
  ttlMs?: number;
}

/** The one question shown at a time; asking another answers this one with false. */
export interface Confirmation {
  title: string;
  text: string;
  /** The destructive button's label, e.g. "Remove". */
  action: string;
  answer: (confirmed: boolean) => void;
}

/** Open or closed for each layout: the inspector starts closed beside the list and open without it. */
interface InspectorOpen {
  withSidebar: boolean;
  withoutSidebar: boolean;
}

export interface Store {
  sidebarHidden: boolean;
  inspectorOpen: InspectorOpen;
  toggleSidebar: () => void;
  toggleInspector: () => void;

  toasts: Toast[];
  toast: (kind: Toast["kind"], message: string, action?: ToastAction) => void;
  dismissToast: (id: number) => void;

  confirmation: Confirmation | null;
  /** Shows the sheet; resolves true when the owner confirms, false on Cancel or Escape. */
  confirm: (question: Omit<Confirmation, "answer">) => Promise<boolean>;

  /** The command palette (⌘K) is open. */
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;

  menu: OpenMenu | null;
  /** Shows a menu, replacing an open one; opening it again on the same element closes it. */
  openMenu: (entries: MenuEntry[], at: MenuAt, label?: string, options?: MenuOptions) => void;
  closeMenu: () => void;
}

const MAX_TOASTS = 5;
let nextToastId = 1;
let nextMenuId = 1;

function focusedElement(): HTMLElement | null {
  const focused = document.activeElement;
  return focused instanceof HTMLElement && focused !== document.body ? focused : null;
}

const layoutOf = (s: Pick<Store, "sidebarHidden">): keyof InspectorOpen => (s.sidebarHidden ? "withoutSidebar" : "withSidebar");

export const inspectorShown = (s: Pick<Store, "sidebarHidden" | "inspectorOpen">) => s.inspectorOpen[layoutOf(s)];

export const useStore = create<Store>()((set, get) => ({
  sidebarHidden: false,
  inspectorOpen: { withSidebar: false, withoutSidebar: true },
  toggleSidebar: () => set((s) => ({ sidebarHidden: !s.sidebarHidden })),
  toggleInspector: () => set((s) => ({ inspectorOpen: { ...s.inspectorOpen, [layoutOf(s)]: !inspectorShown(s) } })),

  toasts: [],
  toast(kind, message, action) {
    if (kind === "error") void api.logError(message).catch(() => {});
    const same = get().toasts.find((t) => t.kind === kind && t.message === message);
    if (same) {
      set((s) => ({ toasts: s.toasts.map((t) => (t.id === same.id ? { ...t, count: t.count + 1 } : t)) }));
      return;
    }
    const id = nextToastId++;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message, count: 1, action }].slice(-MAX_TOASTS) }));
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  confirmation: null,
  confirm: (question) =>
    new Promise<boolean>((resolve) => {
      get().confirmation?.answer(false);
      set({
        confirmation: {
          ...question,
          answer: (confirmed) => {
            set({ confirmation: null });
            resolve(confirmed);
          },
        },
      });
    }),

  paletteOpen: false,
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),

  menu: null,
  openMenu: (entries, at, label = "Menu", options = {}) => {
    if (at instanceof HTMLElement && get().menu?.at === at) {
      set({ menu: null });
      return;
    }
    const opener = at instanceof HTMLElement ? at : focusedElement();
    set({ menu: { id: nextMenuId++, entries, at, label, opener, search: options.search } });
  },
  closeMenu: () => set({ menu: null }),
}));

const initialState = useStore.getState();

/** Restores the initial state, then applies `patch`; for tests. */
export function resetStore(patch: Partial<Store> = {}) {
  useStore.setState({ ...initialState, ...patch }, true);
}
