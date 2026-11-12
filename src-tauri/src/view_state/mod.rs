//! `view.json`: the main window's frame and sidebar filter, restored on the next start. UI state, written
//! straight through its `JsonStore` and never undoable (ADR 0043).

pub mod commands;
pub mod frame;

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppResult;
use crate::library::Filter;
use crate::lock::MutexExt;
use crate::store::JsonStore;

pub const VIEW_FILE: &str = "view.json";

/// A rectangle in logical points: a window's outer position with its inner size, or a screen.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// `None` unless every number is finite and the size is positive.
    pub fn checked(self) -> Option<Self> {
        let finite = [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite());
        (finite && self.width > 0.0 && self.height > 0.0).then_some(self)
    }

    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn overlap(&self, other: &Frame) -> f64 {
        let width = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let height = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        width.max(0.0) * height.max(0.0)
    }
}

/// Whether at least half of `frame` is on the (non-overlapping) `screens`.
pub fn fits(frame: &Frame, screens: &[Frame]) -> bool {
    let visible: f64 = screens.iter().map(|screen| frame.overlap(screen)).sum();
    visible * 2.0 >= frame.area()
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewState {
    pub main_frame: Option<Frame>,
    pub filter: Option<Filter>,
}

impl ViewState {
    /// Each part that is missing or invalid repairs to `None`.
    pub fn repaired(raw: Value) -> Self {
        let part = |key: &str| raw.get(key).cloned().unwrap_or(Value::Null);
        Self {
            main_frame: serde_json::from_value::<Frame>(part("mainFrame"))
                .ok()
                .and_then(Frame::checked),
            filter: serde_json::from_value(part("filter")).ok(),
        }
    }
}

pub struct View {
    store: JsonStore<ViewState>,
    /// The main window's latest frame, not saved yet.
    pending_frame: Mutex<Option<Frame>>,
    save_scheduled: AtomicBool,
}

impl View {
    pub fn open(config_dir: &Path) -> Self {
        Self {
            store: JsonStore::open(config_dir.join(VIEW_FILE), ViewState::repaired),
            pending_frame: Mutex::new(None),
            save_scheduled: AtomicBool::new(false),
        }
    }

    pub fn saved_frame(&self) -> Option<Frame> {
        self.store.read(|view| view.main_frame)
    }

    pub fn filter(&self) -> Option<Filter> {
        self.store.read(|view| view.filter.clone())
    }

    pub fn set_filter(&self, filter: Filter) -> AppResult<()> {
        if self.filter().as_ref() == Some(&filter) {
            return Ok(());
        }
        self.store.update(|view| {
            view.filter = Some(filter);
            Ok(())
        })
    }

    /// Keeps the frame for the next `save_frame`; returns true when no save is scheduled yet, so the caller
    /// schedules one.
    pub fn note_frame(&self, frame: Frame) -> bool {
        *self.pending_frame.lock_unpoisoned() = Some(frame);
        !self.save_scheduled.swap(true, Ordering::SeqCst)
    }

    /// Writes the frame noted last, if it differs from the saved one.
    pub fn save_frame(&self) -> AppResult<()> {
        self.save_scheduled.store(false, Ordering::SeqCst);
        let Some(frame) = self.pending_frame.lock_unpoisoned().take() else {
            return Ok(());
        };
        if self.saved_frame() == Some(frame) {
            return Ok(());
        }
        self.store.update(|view| {
            view.main_frame = Some(frame);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;
    use crate::test_support::temp_config_dir;

    fn frame(x: f64, y: f64, width: f64, height: f64) -> Frame {
        Frame {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn valid_parts_of_the_file_round_trip() {
        let raw = json!({
            "mainFrame": { "x": 10.0, "y": 20.0, "width": 800.0, "height": 600.0 },
            "filter": { "kind": "tag", "tag": "Client" },
        });

        assert_eq!(
            ViewState::repaired(raw),
            ViewState {
                main_frame: Some(frame(10.0, 20.0, 800.0, 600.0)),
                filter: Some(Filter::Tag("Client".into())),
            }
        );
    }

    #[test]
    fn a_bad_frame_repairs_to_none_and_keeps_the_filter() {
        let frames = [
            json!({ "x": 0.0, "y": 0.0, "width": 0.0, "height": 600.0 }),
            json!({ "x": 0.0, "y": 0.0, "width": 800.0, "height": -1.0 }),
            json!({ "x": null, "y": 0.0, "width": 800.0, "height": 600.0 }),
            json!({ "x": "left", "y": 0.0, "width": 800.0, "height": 600.0 }),
            json!({ "x": 0.0, "y": 0.0, "width": 800.0 }),
            json!("big"),
        ];
        for bad in frames {
            let raw = json!({ "mainFrame": bad, "filter": { "kind": "starred" } });

            assert_eq!(
                ViewState::repaired(raw),
                ViewState {
                    main_frame: None,
                    filter: Some(Filter::Starred),
                },
                "{bad}"
            );
        }
    }

    #[test]
    fn an_unknown_filter_repairs_to_none_and_keeps_the_frame() {
        for bad in [json!({ "kind": "nope" }), json!("starred"), json!(7)] {
            let raw = json!({
                "mainFrame": { "x": 1.0, "y": 2.0, "width": 300.0, "height": 200.0 },
                "filter": bad,
            });

            assert_eq!(
                ViewState::repaired(raw),
                ViewState {
                    main_frame: Some(frame(1.0, 2.0, 300.0, 200.0)),
                    filter: None,
                },
                "{bad}"
            );
        }
    }

    #[test]
    fn missing_parts_and_a_non_object_repair_to_the_default() {
        for raw in [json!({}), json!([]), json!(null)] {
            assert_eq!(
                ViewState::repaired(raw.clone()),
                ViewState::default(),
                "{raw}"
            );
        }
    }

    #[test]
    fn a_missing_file_opens_with_the_default() {
        let dir = temp_config_dir();

        let view = View::open(dir.path());

        assert_eq!(view.saved_frame(), None);
        assert_eq!(view.filter(), None);
    }

    #[test]
    fn a_corrupt_file_opens_with_the_default() {
        let dir = temp_config_dir();
        fs::write(dir.path().join(VIEW_FILE), "{ not json").unwrap();

        let view = View::open(dir.path());

        assert_eq!(view.saved_frame(), None);
        assert_eq!(view.filter(), None);
    }

    #[test]
    fn a_checked_frame_needs_finite_numbers_and_a_positive_size() {
        let valid = frame(-50.0, -20.0, 800.0, 600.0);
        assert_eq!(valid.checked(), Some(valid));

        for bad in [
            frame(f64::NAN, 0.0, 800.0, 600.0),
            frame(0.0, f64::INFINITY, 800.0, 600.0),
            frame(0.0, 0.0, f64::NAN, 600.0),
            frame(0.0, 0.0, 800.0, f64::NEG_INFINITY),
            frame(0.0, 0.0, 0.0, 600.0),
            frame(0.0, 0.0, 800.0, 0.0),
            frame(0.0, 0.0, -800.0, 600.0),
            frame(0.0, 0.0, 800.0, -600.0),
        ] {
            assert_eq!(bad.checked(), None, "{bad:?}");
        }
    }

    #[test]
    fn a_frame_fits_when_at_least_half_of_it_is_on_the_screens() {
        let screen = frame(0.0, 0.0, 1000.0, 800.0);
        let cases = [
            (frame(100.0, 100.0, 400.0, 300.0), true),
            (frame(0.0, 0.0, 1000.0, 800.0), true),
            (frame(900.0, 0.0, 200.0, 100.0), true),
            (frame(950.0, 0.0, 100.0, 100.0), true),
            (frame(951.0, 0.0, 100.0, 100.0), false),
            (frame(-100.0, 0.0, 200.0, 100.0), true),
            (frame(0.0, 750.0, 100.0, 100.0), true),
            (frame(0.0, 751.0, 100.0, 100.0), false),
            (frame(900.0, 700.0, 200.0, 200.0), false),
            (frame(2000.0, 0.0, 400.0, 300.0), false),
            (frame(-500.0, -500.0, 400.0, 300.0), false),
        ];
        for (candidate, expected) in cases {
            assert_eq!(fits(&candidate, &[screen]), expected, "{candidate:?}");
        }
    }

    #[test]
    fn a_frame_spanning_two_screens_fits_by_their_combined_area() {
        let left = frame(0.0, 0.0, 1000.0, 800.0);
        let right = frame(1000.0, 0.0, 1000.0, 800.0);

        let spanning = frame(800.0, 100.0, 400.0, 300.0);
        let mostly_right = frame(1900.0, 100.0, 400.0, 300.0);

        assert!(fits(&spanning, &[left, right]));
        assert!(fits(&spanning, &[right, left]));
        assert!(!fits(&spanning, &[frame(5000.0, 0.0, 100.0, 100.0)]));
        assert!(!fits(&mostly_right, &[left, right]));
    }

    #[test]
    fn a_frame_fits_nowhere_without_screens() {
        assert!(!fits(&frame(0.0, 0.0, 800.0, 600.0), &[]));
    }

    #[test]
    fn a_set_filter_is_stored_and_survives_reopening() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());

        view.set_filter(Filter::Tag("Client".into())).unwrap();
        assert_eq!(view.filter(), Some(Filter::Tag("Client".into())));

        let reopened = View::open(dir.path());
        assert_eq!(reopened.filter(), Some(Filter::Tag("Client".into())));

        reopened.set_filter(Filter::Archived).unwrap();
        assert_eq!(View::open(dir.path()).filter(), Some(Filter::Archived));
    }

    #[test]
    fn setting_the_same_filter_again_succeeds_and_changes_nothing() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());
        view.set_filter(Filter::Starred).unwrap();

        view.set_filter(Filter::Starred).unwrap();

        assert_eq!(View::open(dir.path()).filter(), Some(Filter::Starred));
    }

    #[test]
    fn only_the_first_noted_frame_asks_for_a_save_until_it_is_saved() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());

        assert!(view.note_frame(frame(0.0, 0.0, 800.0, 600.0)));
        assert!(!view.note_frame(frame(5.0, 0.0, 800.0, 600.0)));
        assert!(!view.note_frame(frame(10.0, 0.0, 800.0, 600.0)));

        view.save_frame().unwrap();

        assert!(view.note_frame(frame(20.0, 0.0, 800.0, 600.0)));
    }

    #[test]
    fn saving_writes_the_frame_noted_last() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());
        view.note_frame(frame(0.0, 0.0, 800.0, 600.0));
        view.note_frame(frame(30.0, 40.0, 900.0, 700.0));

        view.save_frame().unwrap();

        let expected = Some(frame(30.0, 40.0, 900.0, 700.0));
        assert_eq!(view.saved_frame(), expected);
        assert_eq!(View::open(dir.path()).saved_frame(), expected);
    }

    #[test]
    fn saving_without_a_noted_frame_changes_nothing() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());

        view.save_frame().unwrap();

        assert_eq!(view.saved_frame(), None);
        assert!(!dir.path().join(VIEW_FILE).exists());

        view.note_frame(frame(1.0, 2.0, 800.0, 600.0));
        view.save_frame().unwrap();
        view.save_frame().unwrap();

        assert_eq!(view.saved_frame(), Some(frame(1.0, 2.0, 800.0, 600.0)));
    }

    #[test]
    fn the_saved_file_uses_camel_case_names_and_keeps_the_filter() {
        let dir = temp_config_dir();
        let view = View::open(dir.path());
        view.set_filter(Filter::Untagged).unwrap();
        view.note_frame(frame(1.0, 2.0, 800.0, 600.0));
        view.save_frame().unwrap();

        let written: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(VIEW_FILE)).unwrap()).unwrap();

        assert_eq!(
            written,
            json!({
                "mainFrame": { "x": 1.0, "y": 2.0, "width": 800.0, "height": 600.0 },
                "filter": { "kind": "untagged" },
            })
        );
    }
}
