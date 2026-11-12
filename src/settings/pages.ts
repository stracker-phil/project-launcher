import { Activity, AppWindow, Info, Keyboard, ShieldCheck, SunMoon } from "lucide-react";
import { api, toAppError, type MenuShortcut } from "../api";
import { registerSettingsPage } from "../registry/settingsPages";
import { ABOUT_ROWS, AboutPage } from "./AboutPage";
import { APPEARANCE_ROWS, AppearancePage } from "./AppearancePage";
import { GENERAL_ROWS, GeneralPage } from "./GeneralPage";
import { PRIVACY_ROWS, PrivacyPage } from "./privacy/PrivacyPage";
import { rowAnchor, shortcutRows } from "./shortcuts/rows";
import { ShortcutsPage } from "./shortcuts/ShortcutsPage";
import { STATUS_ROWS, StatusPage } from "./StatusPage";

/** Registered again once the menu's items arrive, so the search finds every command by its title. */
function registerShortcutsPage(menu: MenuShortcut[]) {
  registerSettingsPage({
    id: "shortcuts",
    title: "Shortcuts",
    group: "App",
    intro: "The key for each command. Click a key to record a new one.",
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
    group: "App",
    intro: "The apps projects open with, Docker, and how Launcher starts.",
    icon: AppWindow,
    order: 2,
    keywords: ["apps"],
    rows: GENERAL_ROWS,
    component: GeneralPage,
  });
  registerSettingsPage({
    id: "appearance",
    title: "Appearance",
    group: "App",
    intro: "How every Launcher window looks.",
    icon: SunMoon,
    order: 4,
    keywords: ["look"],
    rows: APPEARANCE_ROWS,
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
    group: "System",
    intro: "What this copy of Launcher is and where it keeps its files.",
    icon: Activity,
    order: 10,
    keywords: ["diagnostics"],
    rows: STATUS_ROWS,
    component: StatusPage,
  });
  registerSettingsPage({
    id: "privacy",
    title: "Privacy & Data",
    group: "System",
    intro: "What Launcher keeps, and where.",
    icon: ShieldCheck,
    order: 15,
    keywords: ["data"],
    rows: PRIVACY_ROWS,
    component: PrivacyPage,
  });
  registerSettingsPage({
    id: "about",
    title: "About",
    group: "System",
    intro: "The version, and the folders a bug report needs.",
    icon: Info,
    order: 20,
    rows: ABOUT_ROWS,
    component: AboutPage,
  });
}
