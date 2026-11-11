import { useEffect } from "react";
import { api, toAppError, type LaunchEvent } from "../api";

/**
 * Takes what a launch left for the page, now and after every later launch, and hands each to `onEvent`.
 * Pass a stable `onEvent` (`useCallback`): a new one takes and subscribes again.
 */
export function useLaunchEvents(onEvent: (event: LaunchEvent) => void) {
  useEffect(() => {
    let live = true;
    const take = () => {
      void api
        .takeLaunchEvents()
        .then((events) => {
          if (live) events.forEach((event) => onEvent(event));
        })
        .catch((e: unknown) => void api.logError(`Taking launch events failed: ${toAppError(e).message}`));
    };
    take();
    const stopListening = api.on("launch", take);
    return () => {
      live = false;
      void stopListening.then((stop) => stop());
    };
  }, [onEvent]);
}
