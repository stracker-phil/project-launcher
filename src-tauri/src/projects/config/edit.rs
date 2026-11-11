//! Changes to a project file's `apps` and `scripts`, as the action editor makes them. They edit copies of both
//! lists; `Lists::changes` returns the ones that differ, for the writer to apply as undoable key changes. Keys
//! Launcher doesn't edit (`hiddenStates`, the owner's own) stay in the entry, in their place.

use serde_json::{Map, Value};

use super::model::{
    ActionDraft, ActionList, ActionSource, Builtin, DRAFT_FIELDS, RunsAs, file_key,
};
use super::resolve::entries;
use crate::error::{AppError, AppResult};
use crate::projects::file::{FileKey, FileObject};

/// The two action lists of a file, as they will be written.
#[derive(Debug)]
pub struct Lists {
    apps: Vec<Value>,
    scripts: Vec<Value>,
    apps_before: Vec<Value>,
    scripts_before: Vec<Value>,
}

impl Lists {
    pub fn of(file: &FileObject) -> Self {
        let apps = entries(file.get("apps")).to_vec();
        let scripts = entries(file.get("scripts")).to_vec();
        Lists {
            apps_before: apps.clone(),
            scripts_before: scripts.clone(),
            apps,
            scripts,
        }
    }

    fn list(&mut self, list: ActionList) -> &mut Vec<Value> {
        match list {
            ActionList::Apps => &mut self.apps,
            ActionList::Scripts => &mut self.scripts,
        }
    }

    /// The keys to write: each list that differs from the file, as an array.
    pub fn changes(self) -> Vec<(FileKey, Value)> {
        let mut changes = Vec::new();
        if self.apps != self.apps_before {
            changes.push((FileKey::Apps, Value::Array(self.apps)));
        }
        if self.scripts != self.scripts_before {
            changes.push((FileKey::Scripts, Value::Array(self.scripts)));
        }
        changes
    }

    /// Creates (`source: None`) or updates an action; changing between a script and anything else moves it to
    /// the other list, at its end.
    pub fn save(
        &mut self,
        source: Option<ActionSource>,
        draft: &ActionDraft,
        preset: Option<Builtin>,
    ) -> AppResult<()> {
        let existing = match source {
            Some(source) => Some(self.entry(source)?.clone()),
            None => None,
        };
        if let (Some(preset), Some(entry)) = (preset, &existing)
            && Builtin::of(entry) != Some(preset)
        {
            return Err(changed_meanwhile());
        }
        let entry = match preset {
            Some(preset) => builtin_entry(preset, draft, existing.as_ref())?,
            None => custom_entry(draft, existing.as_ref())?,
        };
        let target = if preset.is_none() && draft.runs_as == RunsAs::Script {
            ActionList::Scripts
        } else {
            ActionList::Apps
        };
        self.place(source, target, entry);
        Ok(())
    }

    /// In place when it stays in its list, else last in `target`.
    fn place(&mut self, source: Option<ActionSource>, target: ActionList, entry: Value) {
        match source {
            Some(source) if source.list == target => self.list(target)[source.index] = entry,
            Some(source) => {
                self.list(source.list).remove(source.index);
                self.list(target).push(entry);
            }
            None => self.list(target).push(entry),
        }
    }

    pub fn remove(&mut self, source: ActionSource) -> AppResult<()> {
        self.entry(source)?;
        self.list(source.list).remove(source.index);
        Ok(())
    }

    /// Moves the action to `to` in its list; past the end means last.
    pub fn reorder(&mut self, source: ActionSource, to: usize) -> AppResult<()> {
        self.entry(source)?;
        let list = self.list(source.list);
        let entry = list.remove(source.index);
        list.insert(to.min(list.len()), entry);
        Ok(())
    }

    /// Enabling adds the built-in's name last unless it is listed; disabling removes every entry of it,
    /// overrides included.
    pub fn set_builtin(&mut self, preset: Builtin, enabled: bool) {
        let apps = self.list(ActionList::Apps);
        let listed = |e: &Value| Builtin::of(e) == Some(preset);
        if !enabled {
            apps.retain(|e| !listed(e));
        } else if !apps.iter().any(listed) {
            apps.push(Value::String(preset.name().into()));
        }
    }

    fn entry(&mut self, source: ActionSource) -> AppResult<&Value> {
        self.list(source.list)
            .get(source.index)
            .ok_or_else(changed_meanwhile)
    }
}

fn changed_meanwhile() -> AppError {
    AppError::InvalidInput(
        "The project file changed in the meantime; select the project again and retry".into(),
    )
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidField {
        field: field.into(),
        message: message.into(),
    }
}

/// The field's trimmed text, if any; a shortcut in the file's notation.
fn filled(draft: &ActionDraft, field: &str) -> AppResult<Option<String>> {
    let Some(value) = draft.field(field).map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if field == "shortcut" {
        return file_key(value)
            .map(Some)
            .ok_or_else(|| invalid("shortcut", "Not a key combination"));
    }
    Ok(Some(value.to_string()))
}

/// The target fields each way of running writes.
fn target_fields(runs_as: RunsAs) -> &'static [&'static str] {
    match runs_as {
        RunsAs::App => &["app", "args"],
        RunsAs::Url => &["url"],
        RunsAs::Terminal | RunsAs::Script => &["command"],
    }
}

const APPEARANCE_FIELDS: [&str; 3] = ["icon", "color", "shortcut"];

/// A built-in stays its bare name until something overrides it, then becomes `{ "preset": …, …overrides }`.
fn builtin_entry(
    preset: Builtin,
    draft: &ActionDraft,
    existing: Option<&Value>,
) -> AppResult<Value> {
    let mut entry = existing
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entry.shift_remove("preset");
    for field in std::iter::once("label").chain(DRAFT_FIELDS) {
        set(&mut entry, field, filled(draft, field)?);
    }
    if entry.is_empty() {
        return Ok(Value::String(preset.name().into()));
    }
    let mut with_preset = Map::new();
    with_preset.insert("preset".into(), Value::String(preset.name().into()));
    with_preset.extend(entry);
    Ok(Value::Object(with_preset))
}

/// The field each way of running can't do without, with the message asking for it.
fn required_field(runs_as: RunsAs) -> Option<(&'static str, &'static str)> {
    match runs_as {
        RunsAs::App => Some(("app", "Choose an app")),
        RunsAs::Url => Some(("url", "Enter a URL")),
        RunsAs::Script => Some(("command", "Enter a command")),
        RunsAs::Terminal => None,
    }
}

/// An entry of its own: a label, the target fields of how it runs and its appearance; the other target
/// fields are dropped. Saving over a built-in starts a fresh entry. An empty terminal command opens a shell in
/// the project folder.
fn custom_entry(draft: &ActionDraft, existing: Option<&Value>) -> AppResult<Value> {
    let label = draft.label.trim();
    if label.is_empty() {
        return Err(invalid("label", "Give the action a name"));
    }
    if let Some((field, message)) = required_field(draft.runs_as)
        && filled(draft, field)?.is_none()
    {
        return Err(invalid(field, message));
    }
    let mut entry = existing
        .filter(|e| Builtin::of(e).is_none())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entry.insert("label".into(), Value::String(label.into()));
    let targets = target_fields(draft.runs_as);
    for field in ["app", "args", "url", "command"] {
        let value = if targets.contains(&field) {
            filled(draft, field)?
        } else {
            None
        };
        set(&mut entry, field, value);
    }
    for field in APPEARANCE_FIELDS {
        set(&mut entry, field, filled(draft, field)?);
    }
    if draft.runs_as == RunsAs::Terminal && !entry.contains_key("command") {
        entry.insert("command".into(), Value::String(String::new()));
    }
    Ok(Value::Object(entry))
}

/// An existing key keeps its position; `None` removes the key.
fn set(entry: &mut Map<String, Value>, field: &str, value: Option<String>) {
    match value {
        Some(value) => {
            entry.insert(field.into(), Value::String(value));
        }
        None => {
            entry.shift_remove(field);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn lists(file: Value) -> Lists {
        Lists::of(file.as_object().unwrap())
    }

    fn app_at(index: usize) -> ActionSource {
        ActionSource {
            list: ActionList::Apps,
            index,
        }
    }

    fn script_at(index: usize) -> ActionSource {
        ActionSource {
            list: ActionList::Scripts,
            index,
        }
    }

    fn draft(runs_as: RunsAs, label: &str) -> ActionDraft {
        ActionDraft {
            runs_as,
            label: label.into(),
            ..ActionDraft::default()
        }
    }

    fn some(text: &str) -> Option<String> {
        Some(text.into())
    }

    fn keys(entry: &Value) -> Vec<&str> {
        entry
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect()
    }

    fn apps_after(lists: Lists) -> Value {
        lists
            .changes()
            .into_iter()
            .find(|(key, _)| *key == FileKey::Apps)
            .map(|(_, value)| value)
            .expect("apps changed")
    }

    fn field_of(result: AppResult<()>) -> String {
        match result {
            Err(AppError::InvalidField { field, .. }) => field,
            other => panic!("expected an invalid field, got {other:?}"),
        }
    }

    fn assert_changed_meanwhile(result: AppResult<()>) {
        assert!(
            matches!(result, Err(AppError::InvalidInput(_))),
            "{result:?}"
        );
    }

    #[test]
    fn nothing_changed_means_no_changes() {
        let file =
            json!({ "apps": ["editor"], "scripts": [{ "label": "Build", "command": "make" }] });
        assert_eq!(lists(file).changes(), vec![]);
    }

    #[test]
    fn only_the_changed_lists_are_returned() {
        let mut only_scripts = lists(json!({ "apps": ["editor"] }));
        only_scripts
            .save(
                None,
                &ActionDraft {
                    command: some("make"),
                    ..draft(RunsAs::Script, "Build")
                },
                None,
            )
            .unwrap();
        assert_eq!(
            only_scripts.changes(),
            vec![(
                FileKey::Scripts,
                json!([{ "label": "Build", "command": "make" }])
            )]
        );

        let mut only_apps = lists(json!({ "scripts": [] }));
        only_apps.set_builtin(Builtin::Git, true);
        assert_eq!(only_apps.changes(), vec![(FileKey::Apps, json!(["git"]))]);
    }

    #[test]
    fn a_list_that_is_not_an_array_reads_as_empty() {
        let mut lists = lists(json!({ "apps": "editor", "scripts": { "a": 1 } }));
        lists.set_builtin(Builtin::Editor, true);
        assert_eq!(lists.changes(), vec![(FileKey::Apps, json!(["editor"]))]);
    }

    #[test]
    fn a_new_app_is_appended_with_a_trimmed_label_and_only_its_app_fields() {
        let mut lists = lists(json!({ "apps": ["editor"] }));
        let app = ActionDraft {
            app: some(" Figma "),
            args: some("${dir}"),
            url: some("https://example.com"),
            command: some("ls"),
            icon: some("Palette"),
            color: some("blue"),
            shortcut: some("Mod+Shift+L"),
            ..draft(RunsAs::App, "  Figma  ")
        };
        lists.save(None, &app, None).unwrap();
        assert_eq!(
            apps_after(lists),
            json!([
                "editor",
                {
                    "label": "Figma",
                    "app": "Figma",
                    "args": "${dir}",
                    "icon": "Palette",
                    "color": "blue",
                    "shortcut": "cmd+shift+l",
                },
            ])
        );
    }

    #[test]
    fn a_new_url_drops_the_other_target_fields() {
        let mut lists = lists(json!({}));
        let url = ActionDraft {
            url: some("https://example.com/docs"),
            app: some("Figma"),
            args: some("x"),
            command: some("ls"),
            ..draft(RunsAs::Url, "Docs")
        };
        lists.save(None, &url, None).unwrap();
        assert_eq!(
            apps_after(lists),
            json!([{ "label": "Docs", "url": "https://example.com/docs" }])
        );
    }

    #[test]
    fn a_terminal_without_a_command_writes_an_empty_one() {
        let mut lists = lists(json!({}));
        lists
            .save(None, &draft(RunsAs::Terminal, "Shell"), None)
            .unwrap();
        assert_eq!(
            apps_after(lists),
            json!([{ "label": "Shell", "command": "" }])
        );
    }

    #[test]
    fn a_terminal_with_a_command_writes_it_and_nothing_else_of_the_target() {
        let mut lists = lists(json!({}));
        let terminal = ActionDraft {
            command: some("npm run dev"),
            url: some("https://example.com"),
            ..draft(RunsAs::Terminal, "Dev")
        };
        lists.save(None, &terminal, None).unwrap();
        assert_eq!(
            apps_after(lists),
            json!([{ "label": "Dev", "command": "npm run dev" }])
        );
    }

    #[test]
    fn a_new_script_goes_to_the_scripts_list() {
        let mut lists = lists(json!({ "scripts": [{ "label": "Build", "command": "make" }] }));
        let script = ActionDraft {
            command: some("make test"),
            app: some("stray"),
            ..draft(RunsAs::Script, "Test")
        };
        lists.save(None, &script, None).unwrap();
        assert_eq!(
            lists.changes(),
            vec![(
                FileKey::Scripts,
                json!([{ "label": "Build", "command": "make" }, { "label": "Test", "command": "make test" }])
            )]
        );
    }

    #[test]
    fn a_shortcut_in_either_notation_is_written_in_the_files() {
        for (input, expected) in [
            ("Mod+Shift+L", "cmd+shift+l"),
            ("cmd+shift+l", "cmd+shift+l"),
            ("Ctrl+Alt+J", "ctrl+alt+j"),
        ] {
            let mut lists = lists(json!({}));
            let terminal = ActionDraft {
                shortcut: some(input),
                ..draft(RunsAs::Terminal, "Shell")
            };
            lists.save(None, &terminal, None).unwrap();
            assert_eq!(apps_after(lists)[0]["shortcut"], json!(expected), "{input}");
        }
    }

    #[test]
    fn an_invalid_shortcut_is_rejected_on_its_field() {
        let mut lists = lists(json!({}));
        let terminal = ActionDraft {
            shortcut: some("not a key"),
            ..draft(RunsAs::Terminal, "Shell")
        };
        assert_eq!(field_of(lists.save(None, &terminal, None)), "shortcut");
        assert_eq!(lists.changes(), vec![]);
    }

    #[test]
    fn missing_required_fields_are_rejected_on_their_field() {
        let cases = [
            (draft(RunsAs::App, "  "), "label"),
            (draft(RunsAs::Terminal, ""), "label"),
            (draft(RunsAs::App, "Figma"), "app"),
            (
                ActionDraft {
                    app: some("  "),
                    ..draft(RunsAs::App, "Figma")
                },
                "app",
            ),
            (draft(RunsAs::Url, "Docs"), "url"),
            (
                ActionDraft {
                    app: some("Figma"),
                    ..draft(RunsAs::Url, "Docs")
                },
                "url",
            ),
            (draft(RunsAs::Script, "Build"), "command"),
            (
                ActionDraft {
                    command: some(" "),
                    ..draft(RunsAs::Script, "Build")
                },
                "command",
            ),
        ];
        for (draft, field) in cases {
            let mut lists = lists(json!({}));
            assert_eq!(field_of(lists.save(None, &draft, None)), field, "{draft:?}");
            assert_eq!(lists.changes(), vec![], "{draft:?}");
        }
    }

    #[test]
    fn updating_in_place_keeps_unknown_keys_and_the_key_order() {
        let mut lists = lists(json!({
            "apps": [{
                "label": "Old",
                "app": "Figma",
                "hiddenStates": ["live"],
                "custom": 1,
                "icon": "Palette",
                "shortcut": "cmd+f",
            }],
        }));
        let updated = ActionDraft {
            app: some("Sketch"),
            args: some("--new"),
            ..draft(RunsAs::App, "New")
        };
        lists.save(Some(app_at(0)), &updated, None).unwrap();
        let apps = apps_after(lists);
        assert_eq!(
            apps[0],
            json!({ "label": "New", "app": "Sketch", "hiddenStates": ["live"], "custom": 1, "args": "--new" })
        );
        assert_eq!(
            keys(&apps[0]),
            ["label", "app", "hiddenStates", "custom", "args"]
        );
    }

    #[test]
    fn clearing_a_field_removes_its_key() {
        let mut lists = lists(json!({
            "apps": [{ "label": "Docs", "url": "https://example.com", "icon": "Book", "color": "blue" }],
        }));
        let cleared = ActionDraft {
            url: some("https://example.com"),
            icon: some("  "),
            ..draft(RunsAs::Url, "Docs")
        };
        lists.save(Some(app_at(0)), &cleared, None).unwrap();
        assert_eq!(
            apps_after(lists),
            json!([{ "label": "Docs", "url": "https://example.com" }])
        );
    }

    #[test]
    fn updating_an_entry_leaves_the_other_entries_alone() {
        let mut lists =
            lists(json!({ "apps": ["editor", { "label": "A", "command": "a" }, "git"] }));
        let terminal = ActionDraft {
            command: some("b"),
            ..draft(RunsAs::Terminal, "B")
        };
        lists.save(Some(app_at(1)), &terminal, None).unwrap();
        assert_eq!(
            apps_after(lists),
            json!(["editor", { "label": "B", "command": "b" }, "git"])
        );
    }

    #[test]
    fn changing_a_script_to_an_app_moves_it_to_the_end_of_apps() {
        let mut lists = lists(json!({
            "apps": ["editor"],
            "scripts": [
                { "label": "Build", "command": "make", "custom": true },
                { "label": "Test", "command": "make test" },
            ],
        }));
        let app = ActionDraft {
            app: some("Terminal"),
            args: some("make"),
            ..draft(RunsAs::App, "Build")
        };
        lists.save(Some(script_at(0)), &app, None).unwrap();
        assert_eq!(
            lists.changes(),
            vec![
                (
                    FileKey::Apps,
                    json!(["editor", { "label": "Build", "custom": true, "app": "Terminal", "args": "make" }])
                ),
                (
                    FileKey::Scripts,
                    json!([{ "label": "Test", "command": "make test" }])
                ),
            ]
        );
    }

    #[test]
    fn changing_an_app_to_a_script_moves_it_to_the_end_of_scripts() {
        let mut lists = lists(json!({
            "apps": [{ "label": "Shell", "command": "ls" }, "editor"],
            "scripts": [{ "label": "Build", "command": "make" }],
        }));
        let script = ActionDraft {
            command: some("ls -la"),
            ..draft(RunsAs::Script, "Shell")
        };
        lists.save(Some(app_at(0)), &script, None).unwrap();
        assert_eq!(
            lists.changes(),
            vec![
                (FileKey::Apps, json!(["editor"])),
                (
                    FileKey::Scripts,
                    json!([{ "label": "Build", "command": "make" }, { "label": "Shell", "command": "ls -la" }])
                ),
            ]
        );
    }

    #[test]
    fn a_builtin_without_overrides_is_its_bare_name() {
        let mut lists =
            lists(json!({ "apps": [{ "preset": "editor", "label": "Code" }, "terminal"] }));
        lists
            .save(
                Some(app_at(0)),
                &ActionDraft::default(),
                Some(Builtin::Editor),
            )
            .unwrap();
        assert_eq!(apps_after(lists), json!(["editor", "terminal"]));
    }

    #[test]
    fn a_builtin_with_overrides_becomes_an_object_with_the_preset_first() {
        let mut lists = lists(json!({ "apps": ["editor"] }));
        let overrides = ActionDraft {
            label: " Code ".into(),
            app: some("Zed"),
            shortcut: some("Mod+E"),
            ..ActionDraft::default()
        };
        lists
            .save(Some(app_at(0)), &overrides, Some(Builtin::Editor))
            .unwrap();
        let apps = apps_after(lists);
        assert_eq!(
            apps,
            json!([{ "preset": "editor", "label": "Code", "app": "Zed", "shortcut": "cmd+e" }])
        );
        assert_eq!(keys(&apps[0]), ["preset", "label", "app", "shortcut"]);
    }

    #[test]
    fn a_builtin_keeps_its_hidden_states_when_its_overrides_are_cleared() {
        let mut lists = lists(json!({
            "apps": [{ "label": "Code", "preset": "editor", "hiddenStates": ["live"] }],
        }));
        lists
            .save(
                Some(app_at(0)),
                &ActionDraft::default(),
                Some(Builtin::Editor),
            )
            .unwrap();
        let apps = apps_after(lists);
        assert_eq!(
            apps,
            json!([{ "preset": "editor", "hiddenStates": ["live"] }])
        );
        assert_eq!(keys(&apps[0]), ["preset", "hiddenStates"]);
    }

    #[test]
    fn an_invalid_builtin_shortcut_is_rejected_on_its_field() {
        let mut lists = lists(json!({ "apps": ["editor"] }));
        let overrides = ActionDraft {
            shortcut: some("nope"),
            ..ActionDraft::default()
        };
        assert_eq!(
            field_of(lists.save(Some(app_at(0)), &overrides, Some(Builtin::Editor))),
            "shortcut"
        );
    }

    #[test]
    fn saving_a_builtin_over_a_different_entry_says_the_file_changed() {
        for file in [
            json!({ "apps": ["terminal"] }),
            json!({ "apps": [{ "preset": "terminal" }] }),
            json!({ "apps": [{ "label": "Editor", "app": "Zed" }] }),
        ] {
            let mut lists = lists(file.clone());
            assert_changed_meanwhile(lists.save(
                Some(app_at(0)),
                &ActionDraft::default(),
                Some(Builtin::Editor),
            ));
            assert_eq!(lists.changes(), vec![], "{file}");
        }
    }

    #[test]
    fn a_new_builtin_is_appended() {
        let mut lists = lists(json!({ "apps": ["editor"] }));
        let overrides = ActionDraft {
            label: "Repo".into(),
            ..ActionDraft::default()
        };
        lists.save(None, &overrides, Some(Builtin::Git)).unwrap();
        assert_eq!(
            apps_after(lists),
            json!(["editor", { "preset": "git", "label": "Repo" }])
        );
    }

    #[test]
    fn a_source_out_of_range_is_rejected() {
        let file = json!({ "apps": ["editor"], "scripts": [] });
        let terminal = ActionDraft {
            command: some("ls"),
            ..draft(RunsAs::Terminal, "Shell")
        };
        for source in [app_at(1), app_at(9), script_at(0)] {
            assert_changed_meanwhile(lists(file.clone()).save(Some(source), &terminal, None));
            assert_changed_meanwhile(lists(file.clone()).save(
                Some(source),
                &ActionDraft::default(),
                Some(Builtin::Editor),
            ));
            assert_changed_meanwhile(lists(file.clone()).remove(source));
            assert_changed_meanwhile(lists(file.clone()).reorder(source, 0));
        }
    }

    #[test]
    fn removing_deletes_only_that_entry() {
        let mut lists = lists(
            json!({ "apps": ["editor", "terminal", "git"], "scripts": [{ "label": "A", "command": "a" }] }),
        );
        lists.remove(app_at(1)).unwrap();
        assert_eq!(
            lists.changes(),
            vec![(FileKey::Apps, json!(["editor", "git"]))]
        );

        let mut lists = self::lists(json!({ "scripts": [{ "label": "A", "command": "a" }] }));
        lists.remove(script_at(0)).unwrap();
        assert_eq!(lists.changes(), vec![(FileKey::Scripts, json!([]))]);
    }

    #[test]
    fn reordering_moves_an_entry_and_past_the_end_means_last() {
        for (from, to, expected) in [
            (0, 2, json!(["terminal", "git", "editor"])),
            (0, 99, json!(["terminal", "git", "editor"])),
            (2, 0, json!(["git", "editor", "terminal"])),
            (1, 0, json!(["terminal", "editor", "git"])),
            (2, 1, json!(["editor", "git", "terminal"])),
        ] {
            let mut lists = lists(json!({ "apps": ["editor", "terminal", "git"] }));
            lists.reorder(app_at(from), to).unwrap();
            assert_eq!(apps_after(lists), expected, "{from} -> {to}");
        }
    }

    #[test]
    fn reordering_to_the_same_place_changes_nothing() {
        let mut lists = lists(json!({ "apps": ["editor", "terminal"] }));
        lists.reorder(app_at(1), 1).unwrap();
        assert_eq!(lists.changes(), vec![]);
    }

    #[test]
    fn reordering_scripts_changes_only_scripts() {
        let mut lists = lists(json!({
            "apps": ["editor"],
            "scripts": [{ "label": "A", "command": "a" }, { "label": "B", "command": "b" }],
        }));
        lists.reorder(script_at(0), 5).unwrap();
        assert_eq!(
            lists.changes(),
            vec![(
                FileKey::Scripts,
                json!([{ "label": "B", "command": "b" }, { "label": "A", "command": "a" }])
            )]
        );
    }

    #[test]
    fn enabling_a_builtin_appends_its_bare_name_once() {
        let mut lists = lists(json!({ "apps": ["terminal"] }));
        lists.set_builtin(Builtin::RepoBrowser, true);
        lists.set_builtin(Builtin::RepoBrowser, true);
        assert_eq!(apps_after(lists), json!(["terminal", "repoBrowser"]));
    }

    #[test]
    fn enabling_a_builtin_listed_as_an_object_changes_nothing() {
        let mut lists = lists(json!({ "apps": [{ "preset": "editor", "label": "Code" }] }));
        lists.set_builtin(Builtin::Editor, true);
        assert_eq!(lists.changes(), vec![]);
    }

    #[test]
    fn disabling_a_builtin_removes_every_entry_of_it() {
        let mut lists = lists(json!({
            "apps": [
                "editor",
                { "preset": "editor", "label": "Code" },
                "terminal",
                { "label": "Editor", "app": "Zed" },
                { "preset": "git" },
            ],
        }));
        lists.set_builtin(Builtin::Editor, false);
        assert_eq!(
            apps_after(lists),
            json!(["terminal", { "label": "Editor", "app": "Zed" }, { "preset": "git" }])
        );
    }

    #[test]
    fn disabling_an_unlisted_builtin_changes_nothing() {
        let mut lists = lists(json!({ "apps": ["terminal"] }));
        lists.set_builtin(Builtin::Claude, false);
        assert_eq!(lists.changes(), vec![]);
    }
}
