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

const commands = new Map<string, PageCommand>();

/** Adds commands; a later registration of the same id replaces the earlier one. */
export function registerCommands(list: PageCommand[]) {
  for (const c of list) commands.set(c.id, c);
}

export function clearCommands() {
  commands.clear();
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

export function bindingFor(id: string): string | null {
  const binding = commands.get(id)?.defaultBinding;
  return binding ? normalizeBinding(binding) : null;
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

/** `Mod+Shift+K` → `⌘⇧K`. */
export function bindingLabel(binding: string | null): string {
  if (!binding) return "";
  const parts = splitBinding(binding);
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1) as Modifier[];
  return mods.map((m) => MOD_LABELS[m] ?? m).join("") + (KEY_LABELS[key] ?? key);
}

export function isTextInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") return !["checkbox", "radio", "button", "submit", "range"].includes((target as HTMLInputElement).type);
  return false;
}

/** The command for a keydown. In text fields only ⌘/⌃ combinations (and Escape) fire, so typing never runs one. */
export function resolveKeydown(e: KeyLike & { target?: EventTarget | null }): PageCommand | null {
  const binding = bindingFromEvent(e);
  if (!binding) return null;
  const bare = !e.metaKey && !e.ctrlKey && binding !== "Escape";
  if (bare && isTextInput(e.target ?? null)) return null;
  return allCommands().find((c) => bindingFor(c.id) === binding && isEnabled(c)) ?? null;
}
