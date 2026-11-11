import { useEffect } from "react";
import { api } from "../api";
import { sameFilter, useLibrary } from "../state/library";

/** Tells Rust which project the detail shows; the palette and the tray act on it. A failed report only means they
 *  see no current project. */
export function useReportCurrentProject() {
  const currentId = useLibrary((s) => s.current?.id ?? null);
  useEffect(() => {
    void api.setCurrentProject(currentId).catch(() => {});
  }, [currentId]);
}

/** Selects the project the palette, the tray or the command line asks for, under the filter Rust names. */
export function useSelectProjectRequests() {
  useEffect(() => {
    const stop = api.on("select_project", ({ id, filter }) => {
      const library = useLibrary.getState();
      if (!sameFilter(library.filter, filter)) library.setFilter(filter);
      library.select(id);
    });
    return () => void stop.then((unlisten) => unlisten());
  }, []);
}
