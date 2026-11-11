import React from "react";
import ReactDOM from "react-dom/client";
import { applyInitialLook } from "../../lib/theme";
import { registerCoreSettingsPages } from "../../settings/pages";
import "../../styles";
import { PaletteApp } from "./PaletteApp";

// Its Settings pages are searchable here too.
registerCoreSettingsPages();
applyInitialLook();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <PaletteApp />
  </React.StrictMode>,
);
