/** A command of the page's own: menus reach it as `page:<id>`, its key runs it while the window has focus. */
export interface PageCommand {
  id: string;
  title: string;
  /** `Mod+Shift+K` notation; commands whose key is a menu accelerator leave it out. */
  defaultBinding?: string;
  /** Enabled-state check; a disabled command neither fires nor runs from a menu. */
  when?: () => boolean;
  run: () => void | Promise<void>;
}

const MODIFIERS = ["Mod", "Ctrl", "Alt", "Shift"] as const;
type Modifier = (typeof MODIFIERS)[number];
const MOD_LABELS: Record<Modifier, string> = { Mod: "⌘", Ctrl: "⌃", Alt: "⌥", Shift: "⇧" };
/** macOS shows modifiers as ⌃⌥⇧⌘, as the menu bar does. */
const LABEL_ORDER: Modifier[] = ["Ctrl", "Alt", "Shift", "Mod"];
const KEY_LABELS: Record<string, string> = {
  Enter: "↵",
  Escape: "⎋",
  Tab: "⇥",
  Backspace: "⌫",
  Delete: "⌦",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Space: "␣",
};

/** The owner's keys by command id (`settings.json` `shortcuts`); `null` leaves a command without one. */
export type Overrides = Record<string, string | null>;

const commands = new Map<string, PageCommand>();
let overrides: Overrides = {};

/** Adds commands; a later registration of the same id replaces the earlier one. */
export function registerCommands(list: PageCommand[]) {
  for (const c of list) commands.set(c.id, c);
}

export function clearCommands() {
  commands.clear();
  overrides = {};
}

export function allCommands(): PageCommand[] {
  return [...commands.values()];
}

export function getCommand(id: string): PageCommand | undefined {
  return commands.get(id);
}

/** Runs a registered, enabled command; false when there is none to run. */
export function runPageCommand(id: string): boolean {
  const command = commands.get(id);
  if (!command || !isEnabled(command)) return false;
  void command.run();
  return true;
}

const isEnabled = (command: PageCommand) => command.when?.() ?? true;

function canonicalKey(key: string): string | null {
  if (key === " ") return "Space";
  if (key.length === 1) return key.toUpperCase();
  if (["Meta", "Control", "Alt", "Shift", "CapsLock", "Fn", "Dead", "Unidentified"].includes(key)) return null;
  return NAMED_KEYS.find((named) => named.toLowerCase() === key.toLowerCase()) ?? key;
}

/** Named keys in the spelling `KeyboardEvent.key` uses, so a hand-written `escape` matches. */
const NAMED_KEYS = ["Enter", "Escape", "Tab", "Backspace", "Delete", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space", "Home", "End", "PageUp", "PageDown", ...Array.from({ length: 12 }, (_, i) => `F${i + 1}`)];

/** Splits on `+`; a trailing `++` (or a bare `+`) is the `+` key, as in `Mod+Shift++`. */
function splitBinding(binding: string): string[] {
  if (binding === "+") return ["+"];
  if (binding.endsWith("++")) return [...binding.slice(0, -2).split("+"), "+"];
  return binding.split("+");
}

/** Normalizes `shift+mod+k` to `Mod+Shift+K`; null for an unparsable string. */
export function normalizeBinding(binding: string): string | null {
  const mods = new Set<Modifier>();
  let key: string | null = null;
  for (const part of splitBinding(binding).map((p) => p.trim())) {
    const lower = part.toLowerCase();
    const mod = MODIFIERS.find((m) => m.toLowerCase() === lower || (m === "Mod" && ["cmd", "meta", "cmdorctrl"].includes(lower)));
    if (mod) mods.add(mod);
    else if (part && key === null) key = canonicalKey(part);
    else return null;
  }
  if (!key) return null;
  return [...MODIFIERS.filter((m) => mods.has(m)), key].join("+");
}

export function setOverrides(next: Overrides) {
  overrides = { ...next };
}

/** The owner's key for `id` when set (`null`: none), else `defaultBinding`. */
export function resolveBinding(id: string, defaultBinding: string | undefined, custom: Overrides = overrides): string | null {
  const binding = Object.hasOwn(custom, id) ? custom[id] : defaultBinding;
  return binding ? normalizeBinding(binding) : null;
}

export function bindingFor(id: string): string | null {
  return resolveBinding(id, commands.get(id)?.defaultBinding);
}

/** Menu keys need ⌘ or ⌃, so a typed letter never reaches a menu item. */
export function hasCommandOrControl(binding: string): boolean {
  const mods = splitBinding(binding).slice(0, -1);
  return mods.includes("Mod") || mods.includes("Ctrl");
}

export interface KeyLike {
  key: string;
  /** Physical key (`KeyK`, `Digit1`); used when ⌥ composed `key` or the layout is not Latin. */
  code?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

const PUNCTUATION_CODES: Record<string, string> = {
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backquote: "`",
};

function keyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  return PUNCTUATION_CODES[code] ?? null;
}

function isNonLatinChar(key: string) {
  return key.length === 1 && !/^[\x20-\x7e]$|^\p{Script=Latin}$/u.test(key);
}

/** ⌥ composes characters (⌥K is `˚`) and other scripts type other letters, so those use the physical key. */
function eventKey(e: KeyLike): string | null {
  const physical = e.code ? keyFromCode(e.code) : null;
  if (physical && (e.altKey || e.key === "Dead" || e.key === "Unidentified" || isNonLatinChar(e.key))) return physical;
  // ⇧ turns a digit into a symbol that differs by layout; with ⌘ or ⌃ it stays the digit.
  if (physical && e.shiftKey && (e.metaKey || e.ctrlKey) && /^Digit/.test(e.code ?? "")) return physical;
  return canonicalKey(e.key);
}

/** The binding a keyboard event represents, or null for a bare modifier press. */
export function bindingFromEvent(e: KeyLike): string | null {
  const key = eventKey(e);
  if (!key) return null;
  const mods: Modifier[] = [];
  if (e.metaKey) mods.push("Mod");
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  return [...mods, key].join("+");
}

/** The binding's symbols one by one, modifiers first: `["⇧", "⌘", "T"]`. */
export function bindingKeys(binding: string | null): string[] {
  if (!binding) return [];
  const parts = splitBinding(binding);
  const key = parts[parts.length - 1];
  const mods = new Set(parts.slice(0, -1));
  return [...LABEL_ORDER.filter((m) => mods.has(m)).map((m) => MOD_LABELS[m]), KEY_LABELS[key] ?? key];
}

export function bindingLabel(binding: string | null): string {
  return bindingKeys(binding).join("");
}

export function isTextInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") return !["checkbox", "radio", "button", "submit", "range"].includes((target as HTMLInputElement).type);
  return false;
}

/** An open sheet, popover or menu takes Escape first. */
export const layerIsOpen = () => !!document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]');

/** The command for a keydown. In text fields only ⌘/⌃ combinations (and Escape) fire, so typing never runs one. */
export function resolveKeydown(e: KeyLike & { target?: EventTarget | null }): PageCommand | null {
  const binding = bindingFromEvent(e);
  if (!binding) return null;
  const bare = !e.metaKey && !e.ctrlKey && binding !== "Escape";
  if (bare && isTextInput(e.target ?? null)) return null;
  return allCommands().find((c) => bindingFor(c.id) === binding && isEnabled(c)) ?? null;
}
