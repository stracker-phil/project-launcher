//! Tag list edits; renaming or deleting a tag also changes every project file that carries it.

use std::path::Path;

use serde_json::Value;

use super::{edit, invalid_field, required};
use crate::data::ops::Op;
use crate::data::{Changed, Data};
use crate::error::{AppError, AppResult};
use crate::projects::file;
use crate::projects::view::text;
use crate::projects::{FileKey, FileRead, MetaKey};

impl Data {
    pub fn add_tag(&self, name: &str) -> AppResult<Changed> {
        let name = tag_name(name)?;
        self.change(|data| {
            let mut tags = data.settings().tags;
            if tags.contains(&name) {
                return Err(invalid_field("tag", format!("“{name}” already exists")));
            }
            tags.push(name);
            edit("Add Tag", Op::SetTags(tags))
        })
    }

    /// Renames the tag in the list and in every project file that carries it; renaming onto an existing tag
    /// merges the two.
    pub fn rename_tag(&self, from: &str, to: &str) -> AppResult<Changed> {
        let to = tag_name(to)?;
        self.change(|data| {
            let carriers = data.projects_tagged(from);
            let tags = data.settings().tags;
            if !tags.iter().any(|t| t == from) && carriers.is_empty() {
                return Err(no_tag(from));
            }
            if from == to {
                return Ok(None);
            }
            let mut ops = vec![Op::SetTags(renamed_tags(tags, from, &to))];
            ops.extend(retag_carriers(carriers, Some(Value::String(to))));
            edit("Rename Tag", Op::Batch(ops))
        })
    }

    /// Removes the tag from the list and from every project file that carries it.
    pub fn delete_tag(&self, name: &str) -> AppResult<Changed> {
        self.change(|data| {
            let carriers = data.projects_tagged(name);
            let mut tags = data.settings().tags;
            let listed = tags.len();
            tags.retain(|t| t != name);
            if tags.len() == listed && carriers.is_empty() {
                return Err(no_tag(name));
            }
            let mut ops = vec![Op::SetTags(tags)];
            ops.extend(retag_carriers(carriers, None));
            edit("Delete Tag", Op::Batch(ops))
        })
    }

    /// The same tags in a new order.
    pub fn set_tag_order(&self, order: Vec<String>) -> AppResult<Changed> {
        self.change(|data| {
            let tags = data.settings().tags;
            let mut sorted_new = order.clone();
            let mut sorted_old = tags.clone();
            sorted_new.sort();
            sorted_old.sort();
            if sorted_new != sorted_old {
                return Err(AppError::InvalidInput(
                    "The new order must list every tag once".into(),
                ));
            }
            if order == tags {
                return Ok(None);
            }
            edit("Reorder Tags", Op::SetTags(order))
        })
    }

    /// The ids of the projects whose file, read fresh, carries `tag`.
    fn projects_tagged(&self, tag: &str) -> Vec<String> {
        self.registry
            .read(|r| r.projects.clone())
            .into_iter()
            .filter(|p| match file::read(Path::new(&p.path)) {
                FileRead::Found(object) => {
                    text(FileKey::Meta(MetaKey::Tag).get(&object).as_ref()).as_deref() == Some(tag)
                }
                _ => false,
            })
            .map(|p| p.id)
            .collect()
    }
}

/// The tag list with `from` renamed to `to`; when `to` is listed already, `from` merges into it.
fn renamed_tags(mut tags: Vec<String>, from: &str, to: &str) -> Vec<String> {
    match tags.iter().position(|t| t == from) {
        Some(i) if tags.iter().any(|t| t == to) => {
            tags.remove(i);
        }
        Some(i) => tags[i] = to.to_string(),
        None if !tags.iter().any(|t| t == to) => tags.push(to.to_string()),
        None => {}
    }
    tags
}

/// One op per carrier that sets its tag to `value` (`None` removes it).
fn retag_carriers(carriers: Vec<String>, value: Option<Value>) -> impl Iterator<Item = Op> {
    carriers.into_iter().map(move |id| Op::SetFileKey {
        id,
        key: FileKey::Meta(MetaKey::Tag),
        value: value.clone(),
    })
}

fn tag_name(raw: &str) -> AppResult<String> {
    required(raw, || invalid_field("tag", "A tag needs a name"))
}

fn no_tag(name: &str) -> AppError {
    AppError::NotFound(format!("No tag “{name}”"))
}
