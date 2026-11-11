import { Activity, Info } from "lucide-react";
import { registerSettingsPage } from "../registry/settingsPages";
import { AboutPage } from "./AboutPage";
import { StatusPage } from "./StatusPage";

/** The core pages; ids are what `app:settings:<page>` and `open_settings` name. */
export function registerCoreSettingsPages() {
  registerSettingsPage({
    id: "status",
    title: "Status",
    icon: Activity,
    order: 10,
    keywords: ["diagnostics"],
    rows: [
      { id: "version", title: "Version" },
      { id: "build", title: "Build" },
      { id: "data", title: "Data folder" },
      { id: "logs", title: "Log folder" },
      { id: "cli", title: "Command line" },
    ],
    component: StatusPage,
  });
  registerSettingsPage({ id: "about", title: "About", icon: Info, order: 20, component: AboutPage });
}
