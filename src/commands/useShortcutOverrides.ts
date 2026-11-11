import { useEffect } from "react";
import { useSettings } from "../queries/useSettings";
import { setOverrides } from "./registry";

/** Keeps the page keys on the owner's overrides from `settings.json`. */
export function useShortcutOverrides() {
  const shortcuts = useSettings().data?.shortcuts;
  useEffect(() => {
    if (shortcuts) setOverrides(shortcuts);
  }, [shortcuts]);
}
