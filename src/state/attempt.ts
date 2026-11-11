import { toAppError } from "../api";
import { useStore } from "./store";

/** Waits for a request; a failure shows as an error toast, `failure` first. Resolves whether it went through. */
export function attempt(request: Promise<unknown>, failure: string): Promise<boolean> {
  return request.then(
    () => true,
    (e: unknown) => {
      useStore.getState().toast("error", `${failure}: ${toAppError(e).message}`);
      return false;
    },
  );
}
