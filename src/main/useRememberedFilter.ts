import { useEffect } from "react";
import { api } from "../api";
import { sameFilter, useLibrary } from "../state/library";

/** Starts on the sidebar filter Rust kept from last time and tells Rust about every change. A filter chosen before
 *  the answer arrives wins; a failed read or report only means the next start shows another filter. */
export function useRememberedFilter() {
  useEffect(() => {
    let chosen = false;
    const stop = useLibrary.subscribe((state, previous) => {
      if (sameFilter(state.filter, previous.filter)) return;
      chosen = true;
      void api.setViewFilter(state.filter).catch(() => {});
    });
    void api
      .viewFilter()
      .then((filter) => {
        const library = useLibrary.getState();
        if (!chosen && !sameFilter(library.filter, filter)) library.setFilter(filter);
      })
      .catch(() => {});
    return stop;
  }, []);
}
