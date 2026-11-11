import { useEffect } from "react";
import { api, type DevAction, type Project } from "../api";
import { attempt } from "../state/attempt";
import { useRuns } from "../state/runs";
import { useStore } from "../state/store";

export function devAction(project: Project, action: DevAction) {
  return attempt(api.devAction(project.id, action), "Couldn’t change the dev environment");
}

/** Opens the database in DataBee; while its server starts first, a toast says so. */
export function openDatabase(project: Project) {
  const opening = api.openDatabase(project.id).then((opened) => {
    if (opened === "starting") useStore.getState().toast("info", "Starting the dev server first; DataBee opens once it runs.");
  });
  return attempt(opening, "Couldn’t open the database");
}

/** A failed step shows as a toast and its output under the project's Scripts, as a script's would. */
export function useDevFailures() {
  useEffect(() => {
    const stop = api.on("dev_failed", ({ projectId, label, output }) => {
      useStore.getState().toast("error", `${label} failed`);
      useRuns.getState().recordFailure(projectId, label, output);
    });
    return () => void stop.then((unlisten) => unlisten());
  }, []);
}
