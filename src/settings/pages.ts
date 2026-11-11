import { Activity, AppWindow, Info, Keyboard, SunMoon } from "lucide-react";
import { api, toAppError, type MenuShortcut } from "../api";
import { registerSettingsPage } from "../registry/settingsPages";
import { AboutPage } from "./AboutPage";
import { AppearancePage } from "./AppearancePage";
import { GeneralPage } from "./GeneralPage";
import { rowAnchor, shortcutRows } from "./shortcuts/rows";
import { ShortcutsPage } from "./shortcuts/ShortcutsPage";
import { StatusPage } from "./StatusPage";

/** Registered again once the menu's items arrive, so the search finds every command by its title. */
function registerShortcutsPage(menu: MenuShortcut[]) {
  registerSettingsPage({
    id: "shortcuts",
    title: "Shortcuts",
    icon: Keyboard,
    order: 6,
    keywords: ["keys", "keyboard"],
    rows: shortcutRows(menu, {}).map((row) => ({ id: rowAnchor(row.id), title: row.title })),
    component: ShortcutsPage,
  });
}

/** The core pages; ids are what `app:settings:<page>` and `open_settings` name. */
export function registerCoreSettingsPages() {
  registerSettingsPage({
    id: "general",
    title: "General",
    icon: AppWindow,
    order: 2,
    keywords: ["editor", "git", "terminal", "login", "startup"],
    rows: [
      { id: "defaultEditor", title: "Default editor" },
      { id: "gitClient", title: "Git client" },
      { id: "terminalApp", title: "Terminal" },
      { id: "configEditor", title: "Project file editor" },
      { id: "login", title: "Open at login" },
    ],
    component: GeneralPage,
  });
  registerSettingsPage({
    id: "appearance",
    title: "Appearance",
    icon: SunMoon,
    order: 4,
    keywords: ["dark", "light", "theme"],
    component: AppearancePage,
  });
  registerShortcutsPage([]);
  // The page itself shows a failed read; the search keeps the page without the menu's rows.
  void api
    .menuShortcuts()
    .then(registerShortcutsPage)
    .catch((e: unknown) => api.logError(`Reading the menu's shortcuts failed: ${toAppError(e).message}`))
    .catch(() => undefined);
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
