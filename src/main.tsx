import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyInitialLook } from "./lib/theme";
import { useStore } from "./state/store";
import "./styles";

applyInitialLook();

// Lets scripts/dev/js.sh drive the debug build; release builds don't hand the whole store to the page.
if (import.meta.env.VITE_DEV_HOOKS === "1") window.launcher = useStore;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
