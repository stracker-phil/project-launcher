//! The wire format: one JSON line per connection, a bare `IpcMessage` (one-way) or an
//! `IpcRequest` answered with an `IpcReply`.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorKind};

/// Every request another process can make. Each CLI subcommand is one message; `remote::handle`
/// dispatches them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IpcMessage {
    /// Another launch: bring the window to the front unless `hidden`.
    Activate {
        #[serde(default)]
        hidden: bool,
    },
    Error {
        message: String,
    },
    /// Shows the window; `focus` also brings it to the front.
    Show {
        focus: bool,
    },
    Hide,
    Quit,
    Status,
    /// Runs JavaScript in a window (used by scripts/dev to drive the UI).
    #[cfg(debug_assertions)]
    Eval {
        js: String,
    },
    /// Replaces the debug data with the demo seed; refused without `yes`.
    #[cfg(debug_assertions)]
    DevSeed {
        yes: bool,
    },
}

impl IpcMessage {
    /// The message type for logs; never its contents (scripts).
    pub fn kind(&self) -> &'static str {
        match self {
            IpcMessage::Activate { .. } => "Activate",
            IpcMessage::Error { .. } => "Error",
            IpcMessage::Show { .. } => "Show",
            IpcMessage::Hide => "Hide",
            IpcMessage::Quit => "Quit",
            IpcMessage::Status => "Status",
            #[cfg(debug_assertions)]
            IpcMessage::Eval { .. } => "Eval",
            #[cfg(debug_assertions)]
            IpcMessage::DevSeed { .. } => "DevSeed",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IpcRequest {
    pub id: String,
    pub message: IpcMessage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct IpcReply {
    pub id: String,
    pub ok: bool,
    pub result: serde_json::Value,
    pub error: Option<IpcError>,
}

impl IpcReply {
    pub(super) fn new(id: String, outcome: HandlerResult) -> Self {
        match outcome {
            Ok(result) => Self {
                id,
                ok: true,
                result,
                error: None,
            },
            Err(error) => Self {
                id,
                ok: false,
                result: serde_json::Value::Null,
                error: Some(error),
            },
        }
    }

    pub fn into_result(self) -> HandlerResult {
        if self.ok {
            Ok(self.result)
        } else {
            Err(self
                .error
                .unwrap_or_else(|| IpcError::new(ErrorKind::Io, "The request failed")))
        }
    }
}

/// Why a request failed, as its sender sees it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IpcError {
    pub kind: ErrorKind,
    pub message: String,
}

impl IpcError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<AppError> for IpcError {
    fn from(e: AppError) -> Self {
        Self::new(e.kind(), e.to_string())
    }
}

/// What handling a message produced: a JSON result, or an error for the sender.
pub type HandlerResult = Result<serde_json::Value, IpcError>;

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Incoming {
    Request(IpcRequest),
    OneWay(IpcMessage),
}

/// The answer to a one-way message.
pub(super) const ACK: &str = "ok";
/// Longer lines get no answer.
pub const MAX_MESSAGE_BYTES: u64 = 64 * 1024;
/// How long either side waits for the other.
pub(super) const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
