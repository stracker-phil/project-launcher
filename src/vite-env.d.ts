/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" in the debug app bundle (`bun run build:app`), which the dev scripts drive. */
  readonly VITE_DEV_HOOKS?: string;
}

interface Window {
  launcher?: typeof import("./state/store").useStore;
}
