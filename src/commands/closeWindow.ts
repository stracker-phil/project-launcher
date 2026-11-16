import { api, toAppError } from "../api";
import { useStore } from "../state/store";

/** Hides the window in front; the app keeps running in the menu bar. */
export function closeWindow() {
  api.runCommand("window:close").catch((e: unknown) => useStore.getState().toast("error", `Could not close the window: ${toAppError(e).message}`));
}
