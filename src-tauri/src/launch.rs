//! What a launch leaves for the page: queued until the page listens.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::events::LAUNCH_EVENT;
use crate::ipc::IpcMessage;
use crate::lock::MutexExt;
use crate::state::AppState;
use crate::window;

/// Something the page acts on once it listens: from the launch that started the app or a later one.
#[derive(Serialize, Debug, Clone, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchEvent {
    Error { message: String },
    Activate,
}

/// `LaunchEvent`s waiting for the page to take them (`take_launch_events`).
#[derive(Default)]
pub struct LaunchQueue {
    events: Mutex<Vec<LaunchEvent>>,
}

impl LaunchQueue {
    /// Returns true when the message produced an event for the page.
    pub fn push(&self, message: IpcMessage) -> bool {
        let event = match message {
            IpcMessage::Error { message } => LaunchEvent::Error { message },
            IpcMessage::Activate { hidden: false } | IpcMessage::Show { .. } => {
                LaunchEvent::Activate
            }
            _ => return false,
        };
        self.events.lock_unpoisoned().push(event);
        true
    }

    pub fn take_events(&self) -> Vec<LaunchEvent> {
        std::mem::take(&mut *self.events.lock_unpoisoned())
    }
}

/// Also shows the window, except for a hidden activation.
pub fn queue(handle: &AppHandle, message: IpcMessage) {
    let show = !matches!(message, IpcMessage::Activate { hidden: true });
    if handle.state::<AppState>().launches.push(message)
        && let Err(e) = handle.emit(LAUNCH_EVENT, ())
    {
        log::warn!("Couldn't notify the page of a launch: {e}");
    }
    if show {
        window::show_main(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_come_out_in_the_order_they_went_in_and_only_once() {
        let queue = LaunchQueue::default();
        queue.push(IpcMessage::Error {
            message: "boom".into(),
        });
        queue.push(IpcMessage::Show { focus: false });

        assert_eq!(
            queue.take_events(),
            vec![
                LaunchEvent::Error {
                    message: "boom".into()
                },
                LaunchEvent::Activate
            ]
        );
        assert_eq!(queue.take_events(), vec![]);
    }

    #[test]
    fn messages_for_the_page_produce_an_event() {
        let cases = [
            IpcMessage::Error {
                message: "boom".into(),
            },
            IpcMessage::Activate { hidden: false },
            IpcMessage::Show { focus: false },
            IpcMessage::Show { focus: true },
        ];
        for message in cases {
            let queue = LaunchQueue::default();
            assert!(queue.push(message.clone()), "{message:?}");
            assert_eq!(queue.take_events().len(), 1, "{message:?}");
        }
    }

    #[test]
    fn messages_the_page_does_not_act_on_produce_no_event() {
        let mut cases = vec![
            IpcMessage::Activate { hidden: true },
            IpcMessage::Hide,
            IpcMessage::Quit,
            IpcMessage::Status,
        ];
        #[cfg(debug_assertions)]
        cases.push(IpcMessage::Eval { js: "1".into() });
        for message in cases {
            let queue = LaunchQueue::default();
            assert!(!queue.push(message.clone()), "{message:?}");
            assert_eq!(queue.take_events(), vec![], "{message:?}");
        }
    }

    #[test]
    fn events_serialize_with_a_type_tag() {
        assert_eq!(
            serde_json::to_value(LaunchEvent::Error {
                message: "boom".into()
            })
            .unwrap(),
            serde_json::json!({ "type": "error", "message": "boom" })
        );
        assert_eq!(
            serde_json::to_value(LaunchEvent::Activate).unwrap(),
            serde_json::json!({ "type": "activate" })
        );
    }
}
