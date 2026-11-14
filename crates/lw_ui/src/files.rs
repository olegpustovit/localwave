//! Worker-prepared source and directory labels; frames only borrow these immutable snapshots.
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Location {
    pub user: i64,
    pub source: i64,
    pub parent: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub user: i64,
    pub source: i64,
    pub root: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Roots,
    Browse(Location),
    Rescan(Location),
    Import(Location),
    ImportPath { user: i64, source: i64, path: i64 },
    Remove(Target),
    Cancel,
}

pub struct Folder {
    pub id: i64,
    pub root: i64,
    pub name: String,
    pub path: String,
    pub detail: String,
    pub current: bool,
}

pub struct Entry {
    pub id: i64,
    pub name: String,
    pub detail: String,
    pub directory: bool,
    pub audio: bool,
}

pub struct Directory {
    pub location: Location,
    pub source_root: i64,
    pub up: Option<i64>,
    pub title: String,
    pub path: String,
    pub detail: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone)]
pub struct Selection {
    pub revision: u64,
    pub songs: Arc<[i64]>,
    pub context: Arc<lw_core::context::PlaybackContext>,
    pub dropped: bool,
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub revision: u64,
    pub account: Option<i64>,
    pub sources: Arc<[Folder]>,
    pub source_count: String,
    pub directory: Option<Arc<Directory>>,
    pub message: String,
    pub progress: String,
    pub busy: bool,
    pub catalog_revision: u64,
    pub selection: Option<Selection>,
}
