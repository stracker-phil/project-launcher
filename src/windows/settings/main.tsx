import React from "react";
import ReactDOM from "react-dom/client";
import { applyInitialLook } from "../../lib/theme";
import { registerCoreSettingsPages } from "../../settings/pages";
import { useStore } from "../../state/store";
import "../../styles";
import { SettingsApp } from "./SettingsApp";

registerCoreSettingsPages();
applyInitialLook();

if (import.meta.env.VITE_DEV_HOOKS === "1") window.launcher = useStore;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SettingsApp />
  </React.StrictMode>,
);
