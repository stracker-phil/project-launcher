import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  allCommands,
  bindingFromEvent,
  bindingFor,
  bindingKeys,
  bindingLabel,
  clearCommands,
  getCommand,
  hasCommandOrControl,
  normalizeBinding,
  registerCommands,
  resolveBinding,
  resolveKeydown,
  setOverrides,
  runPageCommand,
  type KeyLike,
  type PageCommand,
} from "./registry";

const command = (patch: Partial<PageCommand> = {}): PageCommand => ({ id: "demo.run", title: "Run demo", run: vi.fn(), ...patch });

const key = (k: string, patch: Partial<KeyLike> = {}): KeyLike => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...patch,
});

beforeEach(() => {
  clearCommands();
});

describe("command registry", () => {
  it("returns registered commands by id and lists them all", () => {
    const a = command({ id: "a" });
    const b = command({ id: "b" });

    registerCommands([a, b]);

    expect(getCommand("a")).toBe(a);
    expect(allCommands()).toEqual([a, b]);
  });

  it("replaces a command registered again under the same id", () => {
    registerCommands([command({ id: "a", title: "Old" })]);
    registerCommands([command({ id: "a", title: "New" })]);

    expect(allCommands().map((c) => c.title)).toEqual(["New"]);
  });

  it("forgets every command once cleared", () => {
    registerCommands([command({ id: "a" })]);

    clearCommands();

    expect(allCommands()).toEqual([]);
    expect(getCommand("a")).toBeUndefined();
  });
});

describe("runPageCommand", () => {
  it("runs a registered command and reports true", () => {
    const run = vi.fn();
    registerCommands([command({ run })]);

    expect(runPageCommand("demo.run")).toBe(true);
    expect(run).toHaveBeenCalled();
  });

  it("reports false for an unknown id", () => {
    expect(runPageCommand("nope")).toBe(false);
  });

  it("reports false and does not run a command whose when() is false", () => {
    const run = vi.fn();
    registerCommands([command({ run, when: () => false })]);

    expect(runPageCommand("demo.run")).toBe(false);
    expect(run).not.toHaveBeenCalled();
  });
});

describe("normalizeBinding", () => {
  it.each([
    ["shift+mod+k", "Mod+Shift+K"],
    ["Mod+Shift+K", "Mod+Shift+K"],
    ["alt+ctrl+cmd+j", "Mod+Ctrl+Alt+J"],
    ["cmd+k", "Mod+K"],
    ["meta+k", "Mod+K"],
    ["cmdorctrl+k", "Mod+K"],
    ["escape", "Escape"],
    ["mod+arrowup", "Mod+ArrowUp"],
    ["Mod+ArrowUp", "Mod+ArrowUp"],
    ["Mod+Shift++", "Mod+Shift++"],
    ["+", "+"],
    [" mod + k ", "Mod+K"],
  ])("normalizes %j to %j", (input, expected) => {
    expect(normalizeBinding(input)).toBe(expected);
  });

  it.each([[""], ["Mod"], ["Mod+"], ["Mod+K+L"], ["Shift+Shift"]])("returns null for the invalid binding %j", (input) => {
    expect(normalizeBinding(input)).toBeNull();
  });
});

describe("bindingFromEvent", () => {
  it("lists modifiers in the order Mod, Ctrl, Alt, Shift and upper-cases letters", () => {
    expect(bindingFromEvent(key("k", { metaKey: true, ctrlKey: true, altKey: true, shiftKey: true }))).toBe("Mod+Ctrl+Alt+Shift+K");
    expect(bindingFromEvent(key("k", { shiftKey: true, metaKey: true }))).toBe("Mod+Shift+K");
  });

  it.each(["Meta", "Control", "Alt", "Shift", "CapsLock"])("returns null for a bare %s press", (modifier) => {
    expect(bindingFromEvent(key(modifier, { metaKey: true }))).toBeNull();
  });

  it("names the space bar Space", () => {
    expect(bindingFromEvent(key(" ", { metaKey: true }))).toBe("Mod+Space");
  });

  it("keeps named keys as they are", () => {
    expect(bindingFromEvent(key("Escape"))).toBe("Escape");
    expect(bindingFromEvent(key("ArrowDown", { ctrlKey: true }))).toBe("Ctrl+ArrowDown");
  });

  it("falls back to the physical key when Option composed another character", () => {
    expect(bindingFromEvent(key("˚", { code: "KeyK", altKey: true, metaKey: true }))).toBe("Mod+Alt+K");
  });

  it("falls back to the physical key for a non-Latin layout", () => {
    expect(bindingFromEvent(key("л", { code: "KeyK", metaKey: true }))).toBe("Mod+K");
  });

  it("reads Shift with a digit and Command as the digit, not the layout's symbol", () => {
    expect(bindingFromEvent(key("!", { code: "Digit1", metaKey: true, shiftKey: true }))).toBe("Mod+Shift+1");
  });

  it("keeps the typed symbol for Shift with a digit and no Command or Control", () => {
    expect(bindingFromEvent(key("!", { code: "Digit1", shiftKey: true }))).toBe("Shift+!");
  });
});

describe("bindingLabel", () => {
  it("shows modifiers as their Mac symbols in the menu bar's order, ⌘ last", () => {
    expect(bindingLabel("Mod+Ctrl+Alt+Shift+K")).toBe("⌃⌥⇧⌘K");
  });

  it.each([
    ["Mod+Shift+Z", "⇧⌘Z"],
    ["Mod+Ctrl+S", "⌃⌘S"],
  ])("puts ⌘ after the other modifiers in %j", (binding, label) => {
    expect(bindingLabel(binding)).toBe(label);
  });

  it.each([
    ["Mod+Enter", "⌘↵"],
    ["Escape", "⎋"],
    ["Mod+ArrowUp", "⌘↑"],
    ["Mod+ArrowDown", "⌘↓"],
    ["Mod+ArrowLeft", "⌘←"],
    ["Mod+ArrowRight", "⌘→"],
    ["Mod+Space", "⌘␣"],
    ["Mod+Shift++", "⇧⌘+"],
  ])("labels %j as %j", (binding, label) => {
    expect(bindingLabel(binding)).toBe(label);
  });

  it("is empty for no binding", () => {
    expect(bindingLabel(null)).toBe("");
  });
});

describe("bindingKeys", () => {
  it.each([
    ["Shift+Mod+T", ["⇧", "⌘", "T"]],
    ["Mod+Ctrl+Alt+Shift+K", ["⌃", "⌥", "⇧", "⌘", "K"]],
    ["Mod+Enter", ["⌘", "↵"]],
    ["Escape", ["⎋"]],
    ["Mod+Shift++", ["⇧", "⌘", "+"]],
  ])("splits %j into its symbols, modifiers first", (binding, keys) => {
    expect(bindingKeys(binding)).toEqual(keys);
  });

  it("is empty for no binding", () => {
    expect(bindingKeys(null)).toEqual([]);
  });

  it("joins to the binding's label", () => {
    expect(bindingKeys("Mod+Shift+Z").join("")).toBe(bindingLabel("Mod+Shift+Z"));
  });
});

describe("owner's keys", () => {
  describe("resolveBinding", () => {
    it("uses the default when the owner set nothing", () => {
      expect(resolveBinding("a", "mod+o", {})).toBe("Mod+O");
    });

    it("prefers the owner's key and normalizes it", () => {
      expect(resolveBinding("a", "Mod+O", { a: "shift+mod+p" })).toBe("Mod+Shift+P");
    });

    it("is null when the owner removed the key", () => {
      expect(resolveBinding("a", "Mod+O", { a: null })).toBeNull();
    });

    it("is null without an owner's key or a default", () => {
      expect(resolveBinding("a", undefined, {})).toBeNull();
    });

    it("is null for an owner's key that cannot be parsed", () => {
      expect(resolveBinding("a", "Mod+O", { a: "Mod" })).toBeNull();
    });
  });

  describe("setOverrides", () => {
    it("changes the key a command is found under", () => {
      registerCommands([command({ defaultBinding: "Mod+O" })]);

      setOverrides({ "demo.run": "Mod+P" });

      expect(bindingFor("demo.run")).toBe("Mod+P");
    });

    it("moves the keydown to the owner's key", () => {
      const open = command({ defaultBinding: "Mod+O" });
      registerCommands([open]);

      setOverrides({ "demo.run": "Mod+P" });

      expect(resolveKeydown(key("p", { metaKey: true }))).toBe(open);
      expect(resolveKeydown(key("o", { metaKey: true }))).toBeNull();
    });

    it("unbinds a command set to null", () => {
      registerCommands([command({ defaultBinding: "Mod+O" })]);

      setOverrides({ "demo.run": null });

      expect(bindingFor("demo.run")).toBeNull();
      expect(resolveKeydown(key("o", { metaKey: true }))).toBeNull();
    });

    it("replaces the earlier overrides", () => {
      registerCommands([command({ defaultBinding: "Mod+O" })]);
      setOverrides({ "demo.run": "Mod+P" });

      setOverrides({});

      expect(bindingFor("demo.run")).toBe("Mod+O");
    });

    it("is forgotten when the commands are cleared", () => {
      setOverrides({ "demo.run": "Mod+P" });

      clearCommands();
      registerCommands([command({ defaultBinding: "Mod+O" })]);

      expect(bindingFor("demo.run")).toBe("Mod+O");
    });
  });

  describe("bindingFor", () => {
    it("is null for an unknown command", () => {
      expect(bindingFor("nope")).toBeNull();
    });
  });
});

describe("hasCommandOrControl", () => {
  it.each(["Mod+K", "Ctrl+K", "Mod+Shift+K", "Ctrl+Alt+Space", "Mod++"])("accepts %j", (binding) => {
    expect(hasCommandOrControl(binding)).toBe(true);
  });

  it.each(["K", "Shift+K", "Alt+K", "Alt+Shift+ArrowUp", "Escape", "+"])("rejects %j", (binding) => {
    expect(hasCommandOrControl(binding)).toBe(false);
  });
});

describe("resolveKeydown", () => {
  it("finds the enabled command whose default binding matches the key", () => {
    const open = command({ id: "open", defaultBinding: "mod+o" });
    registerCommands([command({ id: "other", defaultBinding: "Mod+P" }), open]);

    expect(resolveKeydown(key("o", { metaKey: true }))).toBe(open);
  });

  it("finds nothing for a key no command is bound to", () => {
    registerCommands([command({ defaultBinding: "Mod+O" })]);

    expect(resolveKeydown(key("x", { metaKey: true }))).toBeNull();
  });

  it("skips a command whose when() is false", () => {
    registerCommands([command({ defaultBinding: "Mod+O", when: () => false })]);

    expect(resolveKeydown(key("o", { metaKey: true }))).toBeNull();
  });

  it("finds nothing for a bare modifier press", () => {
    registerCommands([command({ defaultBinding: "Mod+O" })]);

    expect(resolveKeydown(key("Meta", { metaKey: true }))).toBeNull();
  });

  describe("in text fields", () => {
    const contentEditable = () => {
      const div = document.createElement("div");
      Object.defineProperty(div, "isContentEditable", { value: true });
      return div;
    };
    const fields: [string, () => HTMLElement][] = [
      ["an input", () => document.createElement("input")],
      ["a textarea", () => document.createElement("textarea")],
      ["a contenteditable element", contentEditable],
    ];

    it.each(fields)("ignores a bare key typed into %s", (_name, make) => {
      registerCommands([command({ defaultBinding: "K" })]);

      expect(resolveKeydown({ ...key("k"), target: make() })).toBeNull();
    });

    it.each(fields)("still fires a Command combination in %s", (_name, make) => {
      const run = command({ defaultBinding: "Mod+K" });
      registerCommands([run]);

      expect(resolveKeydown({ ...key("k", { metaKey: true }), target: make() })).toBe(run);
    });

    it("still fires a Control combination in an input", () => {
      const run = command({ defaultBinding: "Ctrl+K" });
      registerCommands([run]);

      expect(resolveKeydown({ ...key("k", { ctrlKey: true }), target: document.createElement("input") })).toBe(run);
    });

    it("still fires Escape in an input", () => {
      const close = command({ defaultBinding: "Escape" });
      registerCommands([close]);

      expect(resolveKeydown({ ...key("Escape"), target: document.createElement("input") })).toBe(close);
    });

    it("fires a bare key on a checkbox, which is not a text field", () => {
      const toggle = command({ defaultBinding: "K" });
      registerCommands([toggle]);
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";

      expect(resolveKeydown({ ...key("k"), target: checkbox })).toBe(toggle);
    });
  });
});
