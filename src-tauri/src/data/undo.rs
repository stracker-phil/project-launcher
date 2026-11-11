//! The undo and redo history of this app run (ADR 0015): each entry is the op that takes a change back (or,
//! once undone, forward again).

use std::collections::VecDeque;

use super::ops::Op;

const LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// What the change did, e.g. "Rename Project".
    pub label: String,
    pub op: Op,
}

#[derive(Debug, Default)]
pub struct UndoStack {
    done: VecDeque<Entry>,
    undone: Vec<Entry>,
}

impl UndoStack {
    /// A new change: undoable, and nothing is left to redo. The oldest entry goes past `LIMIT`.
    pub fn push(&mut self, entry: Entry) {
        self.undone.clear();
        self.done.push_back(entry);
        if self.done.len() > LIMIT {
            self.done.pop_front();
        }
    }

    pub fn take_undo(&mut self) -> Option<Entry> {
        self.done.pop_back()
    }

    pub fn take_redo(&mut self) -> Option<Entry> {
        self.undone.pop()
    }

    /// An undone change, ready to redo.
    pub fn push_undone(&mut self, entry: Entry) {
        self.undone.push(entry);
    }

    /// A redone change, ready to undo again; keeps what is left to redo.
    pub fn push_redone(&mut self, entry: Entry) {
        self.done.push_back(entry);
        if self.done.len() > LIMIT {
            self.done.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
    }
}
