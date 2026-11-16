//! Key combinations in the page's notation (`Mod+Shift+K`, how `settings.json` stores them) and Tauri's
//! accelerator notation (`CmdOrCtrl+Shift+K`, what menu items take). Either parses; each prints both.

use crate::store::Shortcuts;

use super::Item;

/// `(binding, accelerator)` for the punctuation keys the menu takes.
const PUNCTUATION: &[(&str, &str)] = &[
    (",", "Comma"),
    (".", "Period"),
    ("[", "BracketLeft"),
    ("]", "BracketRight"),
    ("-", "Minus"),
    ("=", "Equal"),
    ("\\", "Backslash"),
    (";", "Semicolon"),
    ("'", "Quote"),
    ("/", "Slash"),
    ("`", "Backquote"),
];

/// Other spellings of named keys, `(alias, binding)`.
const ALIASES: &[(&str, &str)] = &[
    ("Up", "ArrowUp"),
    ("Down", "ArrowDown"),
    ("Left", "ArrowLeft"),
    ("Right", "ArrowRight"),
    ("Esc", "Escape"),
    ("Return", "Enter"),
];

const NAMED: &[&str] = &[
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "Delete",
    "Space",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Home",
    "End",
    "PageUp",
    "PageDown",
];

/// A key with its modifiers; `key` is in the page's spelling (`K`, `,`, `ArrowUp`, `F5`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Combo {
    pub command: bool,
    pub control: bool,
    pub option: bool,
    pub shift: bool,
    pub key: String,
}

impl Combo {
    /// Either notation, any case; `None` without exactly one known key. An unknown modifier reads as a key,
    /// so it fails as an unknown or second key.
    pub fn parse(text: &str) -> Option<Self> {
        let mut combo = Combo::default();
        for part in text.trim().split('+').map(str::trim) {
            if let Some(flag) = combo.modifier(part) {
                *flag = true;
            } else if combo.key.is_empty() {
                combo.key = key_name(part)?;
            } else {
                return None;
            }
        }
        (!combo.key.is_empty()).then_some(combo)
    }

    /// The flag `part` names, if it is a modifier.
    fn modifier(&mut self, part: &str) -> Option<&mut bool> {
        match part.to_ascii_lowercase().as_str() {
            "mod" | "cmd" | "command" | "meta" | "super" | "cmdorctrl" | "commandorcontrol" => {
                Some(&mut self.command)
            }
            "ctrl" | "control" => Some(&mut self.control),
            "alt" | "option" => Some(&mut self.option),
            "shift" => Some(&mut self.shift),
            _ => None,
        }
    }

    /// Menu keys need ⌘ or ⌃, so a typed letter never reaches a menu item.
    pub fn has_command_or_control(&self) -> bool {
        self.command || self.control
    }

    /// `Mod+Ctrl+Alt+Shift+K`.
    pub fn binding(&self) -> String {
        self.join(["Mod", "Ctrl", "Alt", "Shift"], &self.key)
    }

    /// `CmdOrCtrl+Ctrl+Alt+Shift+K`.
    pub fn accelerator(&self) -> String {
        let key = PUNCTUATION
            .iter()
            .find(|(binding, _)| *binding == self.key)
            .map_or(self.key.as_str(), |(_, name)| name);
        self.join(["CmdOrCtrl", "Ctrl", "Alt", "Shift"], key)
    }

    fn join(&self, names: [&str; 4], key: &str) -> String {
        let flags = [self.command, self.control, self.option, self.shift];
        let mut parts: Vec<&str> = names
            .into_iter()
            .zip(flags)
            .filter_map(|(name, on)| on.then_some(name))
            .collect();
        parts.push(key);
        parts.join("+")
    }
}

/// The page's spelling of a key, from either notation.
fn key_name(part: &str) -> Option<String> {
    if part.len() == 1 {
        let c = part.chars().next()?;
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
        return PUNCTUATION
            .iter()
            .find(|(binding, _)| *binding == part)
            .map(|(binding, _)| binding.to_string());
    }
    let same = |name: &&str| name.eq_ignore_ascii_case(part);
    if let Some((binding, _)) = PUNCTUATION.iter().find(|(_, name)| same(name)) {
        return Some(binding.to_string());
    }
    if let Some((_, binding)) = ALIASES.iter().find(|(alias, _)| same(alias)) {
        return Some(binding.to_string());
    }
    if let Some(name) = NAMED.iter().find(|name| same(name)) {
        return Some(name.to_string());
    }
    let function_key = part
        .strip_prefix(['F', 'f'])
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=12).contains(n));
    function_key.map(|n| format!("F{n}"))
}

/// The key a menu item has with the owner's overrides: an override that menus can't take (no ⌘ or ⌃, an
/// unknown key) leaves the default.
pub fn effective_accelerator(item: &Item, shortcuts: &Shortcuts) -> Option<String> {
    if !item.takes_a_key() {
        return None;
    }
    let default = || item.accelerator.map(str::to_string);
    match shortcuts.get(item.command) {
        Some(None) => None,
        Some(Some(binding)) => menu_combo(binding)
            .map(|combo| combo.accelerator())
            .or_else(default),
        None => default(),
    }
}

/// A combination a menu item can take.
pub fn menu_combo(text: &str) -> Option<Combo> {
    Combo::parse(text).filter(Combo::has_command_or_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOGGLE: Item = Item {
        label: "Toggle",
        command: "test:toggle",
        accelerator: Some("CmdOrCtrl+Shift+T"),
    };

    const UNBOUND: Item = Item {
        label: "Unbound",
        command: "test:unbound",
        accelerator: None,
    };

    fn combo(command: bool, control: bool, option: bool, shift: bool, key: &str) -> Combo {
        Combo {
            command,
            control,
            option,
            shift,
            key: key.to_string(),
        }
    }

    fn overrides(command: &str, key: Option<&str>) -> Shortcuts {
        let mut shortcuts = Shortcuts::new();
        shortcuts.insert(command.to_string(), key.map(str::to_string));
        shortcuts
    }

    #[test]
    fn quit_has_no_accelerator_whatever_the_overrides_say() {
        for shortcuts in [
            Shortcuts::new(),
            overrides("app:quit", Some("Mod+Q")),
            overrides("app:quit", None),
        ] {
            assert_eq!(
                effective_accelerator(&crate::menu::items::QUIT, &shortcuts),
                None
            );
        }
    }

    #[test]
    fn both_notations_parse_to_the_same_combination() {
        let expected = combo(true, false, false, true, "K");
        for text in [
            "Mod+Shift+K",
            "CmdOrCtrl+Shift+K",
            "CommandOrControl+Shift+K",
        ] {
            assert_eq!(Combo::parse(text), Some(expected.clone()), "{text}");
        }
    }

    #[test]
    fn parsing_ignores_case_and_surrounding_spaces() {
        let expected = combo(true, true, true, true, "K");
        for text in [
            "mod+ctrl+alt+shift+k",
            "MOD+CTRL+ALT+SHIFT+K",
            "  Mod + Ctrl + Alt + Shift + k  ",
            "cmdorctrl+control+option+shift+K",
        ] {
            assert_eq!(Combo::parse(text), Some(expected.clone()), "{text}");
        }
    }

    #[test]
    fn modifier_aliases_parse_to_their_modifier() {
        for (text, expected) in [
            ("cmd+K", combo(true, false, false, false, "K")),
            ("Command+K", combo(true, false, false, false, "K")),
            ("meta+K", combo(true, false, false, false, "K")),
            ("super+K", combo(true, false, false, false, "K")),
            ("control+K", combo(false, true, false, false, "K")),
            ("option+K", combo(false, false, true, false, "K")),
            ("alt+K", combo(false, false, true, false, "K")),
        ] {
            assert_eq!(Combo::parse(text), Some(expected), "{text}");
        }
    }

    #[test]
    fn key_aliases_and_names_parse_to_the_pages_spelling() {
        for (text, key) in [
            ("Mod+Up", "ArrowUp"),
            ("Mod+Down", "ArrowDown"),
            ("Mod+Left", "ArrowLeft"),
            ("Mod+Right", "ArrowRight"),
            ("Mod+ArrowUp", "ArrowUp"),
            ("Mod+arrowdown", "ArrowDown"),
            ("Mod+Esc", "Escape"),
            ("Mod+Return", "Enter"),
            ("Mod+enter", "Enter"),
            ("Mod+space", "Space"),
            ("Mod+tab", "Tab"),
            ("Mod+backspace", "Backspace"),
            ("Mod+delete", "Delete"),
            ("Mod+home", "Home"),
            ("Mod+end", "End"),
            ("Mod+pageup", "PageUp"),
            ("Mod+PageDown", "PageDown"),
            ("Mod+k", "K"),
            ("Mod+5", "5"),
        ] {
            assert_eq!(
                Combo::parse(text).map(|c| c.key),
                Some(key.to_string()),
                "{text}"
            );
        }
    }

    #[test]
    fn punctuation_parses_as_a_character_or_by_name() {
        for (character, name) in [
            (",", "Comma"),
            (".", "Period"),
            ("[", "BracketLeft"),
            ("]", "BracketRight"),
            ("-", "Minus"),
            ("=", "Equal"),
            ("\\", "Backslash"),
            (";", "Semicolon"),
            ("'", "Quote"),
            ("/", "Slash"),
            ("`", "Backquote"),
        ] {
            let by_character = Combo::parse(&format!("Mod+{character}"));
            let by_name = Combo::parse(&format!("CmdOrCtrl+{}", name.to_lowercase()));
            assert_eq!(
                by_character.as_ref().map(|c| c.key.as_str()),
                Some(character),
                "{character}"
            );
            assert_eq!(by_character, by_name, "{name}");
        }
    }

    #[test]
    fn function_keys_one_to_twelve_parse_in_any_case() {
        for n in 1..=12 {
            for text in [format!("Mod+F{n}"), format!("Mod+f{n}")] {
                assert_eq!(
                    Combo::parse(&text).map(|c| c.key),
                    Some(format!("F{n}")),
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn function_keys_outside_one_to_twelve_are_rejected() {
        for text in ["Mod+F0", "Mod+F13", "Mod+F100"] {
            assert_eq!(Combo::parse(text), None, "{text}");
        }
    }

    #[test]
    fn a_combination_without_exactly_one_known_key_is_rejected() {
        for text in [
            "",
            "   ",
            "Mod",
            "Mod+Shift",
            "Mod++",
            "+",
            "Mod+Plus",
            "Mod+K+L",
            "Mod+K+Shift+L",
            "Mod+Nope",
            "Hyper+K",
            "Mod+@",
            "Mod+Ä",
        ] {
            assert_eq!(Combo::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_binding_puts_modifiers_in_a_fixed_order() {
        for (text, expected) in [
            ("CmdOrCtrl+Ctrl+S", "Mod+Ctrl+S"),
            ("Shift+Alt+Ctrl+Cmd+X", "Mod+Ctrl+Alt+Shift+X"),
            ("shift+mod+k", "Mod+Shift+K"),
            ("Mod+,", "Mod+,"),
            ("Ctrl+Up", "Ctrl+ArrowUp"),
            ("F5", "F5"),
        ] {
            let combo = Combo::parse(text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(combo.binding(), expected, "{text}");
        }
    }

    #[test]
    fn the_accelerator_puts_modifiers_in_a_fixed_order_and_names_punctuation() {
        for (text, expected) in [
            ("Mod+Ctrl+S", "CmdOrCtrl+Ctrl+S"),
            ("Shift+Alt+Ctrl+Cmd+X", "CmdOrCtrl+Ctrl+Alt+Shift+X"),
            ("Mod+,", "CmdOrCtrl+Comma"),
            ("Mod+.", "CmdOrCtrl+Period"),
            ("Mod+[", "CmdOrCtrl+BracketLeft"),
            ("Mod+]", "CmdOrCtrl+BracketRight"),
            ("Mod+-", "CmdOrCtrl+Minus"),
            ("Mod+=", "CmdOrCtrl+Equal"),
            ("Mod+\\", "CmdOrCtrl+Backslash"),
            ("Mod+;", "CmdOrCtrl+Semicolon"),
            ("Mod+'", "CmdOrCtrl+Quote"),
            ("Mod+/", "CmdOrCtrl+Slash"),
            ("Mod+`", "CmdOrCtrl+Backquote"),
            ("Mod+Esc", "CmdOrCtrl+Escape"),
            ("Mod+Down", "CmdOrCtrl+ArrowDown"),
            ("Alt+F12", "Alt+F12"),
        ] {
            let combo = Combo::parse(text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(combo.accelerator(), expected, "{text}");
        }
    }

    #[test]
    fn a_binding_survives_a_round_trip_through_the_accelerator() {
        for binding in [
            "Mod+Shift+K",
            "Mod+Ctrl+Alt+Shift+K",
            "Ctrl+J",
            "Mod+,",
            "Mod+ArrowUp",
            "Mod+Enter",
            "Mod+F5",
            "Mod+5",
        ] {
            let combo = Combo::parse(binding).unwrap();
            assert_eq!(combo.binding(), binding, "{binding}");
            let back = Combo::parse(&combo.accelerator()).unwrap();
            assert_eq!(back, combo, "{binding}");
            assert_eq!(back.binding(), binding, "{binding}");
        }
    }

    #[test]
    fn an_accelerator_survives_a_round_trip_through_the_binding() {
        for accelerator in [
            "CmdOrCtrl+Shift+K",
            "CmdOrCtrl+Ctrl+Alt+Shift+K",
            "Ctrl+J",
            "CmdOrCtrl+Comma",
            "CmdOrCtrl+ArrowDown",
            "CmdOrCtrl+Escape",
            "CmdOrCtrl+F12",
        ] {
            let combo = Combo::parse(accelerator).unwrap();
            assert_eq!(combo.accelerator(), accelerator, "{accelerator}");
            let back = Combo::parse(&combo.binding()).unwrap();
            assert_eq!(back, combo, "{accelerator}");
            assert_eq!(back.accelerator(), accelerator, "{accelerator}");
        }
    }

    #[test]
    fn menu_combos_need_command_or_control() {
        for (text, accepted) in [
            ("Mod+K", true),
            ("Cmd+K", true),
            ("Ctrl+K", true),
            ("Shift+Ctrl+K", true),
            ("Mod+Alt+Shift+K", true),
            ("K", false),
            ("Shift+K", false),
            ("Alt+K", false),
            ("Alt+Shift+K", false),
            ("Mod+Nope", false),
            ("", false),
        ] {
            assert_eq!(menu_combo(text).is_some(), accepted, "{text:?}");
        }
    }

    #[test]
    fn without_an_override_the_default_accelerator_applies() {
        assert_eq!(
            effective_accelerator(&TOGGLE, &Shortcuts::new()).as_deref(),
            Some("CmdOrCtrl+Shift+T")
        );
        assert_eq!(effective_accelerator(&UNBOUND, &Shortcuts::new()), None);
    }

    #[test]
    fn an_override_in_either_notation_replaces_the_default() {
        for key in ["Mod+Alt+J", "cmd+option+j", "CmdOrCtrl+Alt+J"] {
            assert_eq!(
                effective_accelerator(&TOGGLE, &overrides("test:toggle", Some(key))).as_deref(),
                Some("CmdOrCtrl+Alt+J"),
                "{key}"
            );
        }
    }

    #[test]
    fn an_override_can_give_a_key_to_an_item_without_a_default() {
        assert_eq!(
            effective_accelerator(&UNBOUND, &overrides("test:unbound", Some("Mod+,"))).as_deref(),
            Some("CmdOrCtrl+Comma")
        );
    }

    #[test]
    fn a_null_override_leaves_the_item_without_a_key() {
        assert_eq!(
            effective_accelerator(&TOGGLE, &overrides("test:toggle", None)),
            None
        );
    }

    #[test]
    fn an_unusable_override_falls_back_to_the_default() {
        for key in ["K", "Shift+K", "Mod+Nope", "Mod+K+L", ""] {
            assert_eq!(
                effective_accelerator(&TOGGLE, &overrides("test:toggle", Some(key))).as_deref(),
                Some("CmdOrCtrl+Shift+T"),
                "{key:?}"
            );
            assert_eq!(
                effective_accelerator(&UNBOUND, &overrides("test:unbound", Some(key))),
                None,
                "{key:?}"
            );
        }
    }

    #[test]
    fn overrides_of_other_commands_do_not_apply() {
        assert_eq!(
            effective_accelerator(&TOGGLE, &overrides("test:other", Some("Mod+J"))).as_deref(),
            Some("CmdOrCtrl+Shift+T")
        );
    }
}
