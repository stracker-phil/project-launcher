import { useEffect } from "react";
import { api, toAppError } from "../api";
import { useStore } from "../state/store";
import { isTextInput } from "./registry";

/** The Edit menu's Undo and Redo hold ⌘Z and ⇧⌘Z, so text fields never see them: a focused text field gets its
 *  typing undone here, anything else undoes the last change of the data. */
export function routeEditRequest(kind: "undo" | "redo") {
  if (isTextInput(document.activeElement)) {
    document.execCommand(kind);
    return;
  }
  (kind === "undo" ? api.undo() : api.redo()).catch((e: unknown) =>
    useStore.getState().toast("error", `Could not ${kind}: ${toAppError(e).message}`),
  );
}

/** Routes the Edit menu's Undo and Redo for this window while mounted. */
export function useEditRequests() {
  useEffect(() => {
    const stopListening = api.on("edit_request", routeEditRequest);
    return () => void stopListening.then((stop) => stop());
  }, []);
}
