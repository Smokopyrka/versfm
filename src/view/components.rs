use tokio_stream::Stream;
use async_trait::async_trait;

mod s3list;
mod filesystem_list;

pub use s3list::S3List;
pub use filesystem_list::{FilesystemList, FilesystemObject};
use tui::widgets::ListState;

use crate::aws::{Kind, s3::S3Object};

#[derive(Clone)]
pub enum State {
    Unselected,
    ToMove,
    ToDelete,
    ToCopy,
}

#[derive(Clone)]
pub struct ListEntry<T> {
    value: T,
    state: State,
}

impl<T> ListEntry<T> {
    fn new(value: T) -> ListEntry<T> {
        ListEntry {
            value,
            state: State::Unselected,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn selected(&self) -> &State {
        &self.state
    }

    fn select(&mut self, new: State) {
        self.state = match self.state {
            State::Unselected => new,
            _ => State::Unselected,
        }
    }
}

impl<T> From<&ListEntry<T>> for ListEntry<FileEntry>
where T: Into<FileEntry> + Clone {
    fn from(entry: &ListEntry<T>) -> Self {
        ListEntry {
            value: entry.value.clone().into(),
            state: entry.state.clone(),
        }
    }
}

pub struct FileEntry {
    pub name: String,
    pub kind: Kind,
}

impl FileEntry {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_kind(&self) -> &Kind {
        &self.kind
    }
}

impl From<S3Object> for FileEntry {
    fn from(entry: S3Object) -> Self {
        FileEntry {
            name: entry.name,
            kind: entry.kind,
        }
    }
}

impl From<FilesystemObject> for FileEntry {
    fn from(entry: FilesystemObject) -> Self {
        FileEntry {
            name: entry.name,
            kind: entry.kind,
        }
    }
}

pub trait StatefulContainer {
    fn previous(&mut self);
    fn next(&mut self);
    fn get_current(&self) -> ListState;
}

pub trait SelectableContainer<T> {
    fn select(&mut self, selection: State);
    fn get_selected(&mut self, selection: State) -> Vec<T>;
    fn get(&self, i: usize) -> ListEntry<T>;
    fn get_items(&self) -> Vec<ListEntry<T>>;
}

#[async_trait]
pub trait FileCRUD {
    async fn refresh(&mut self);
    async fn get_file_stream(&mut self, file_name: &str) -> Box<dyn Stream<Item=u8>>;
    async fn put_file(&mut self, file_name: &str, stream: Box<dyn Stream<Item=u8> + Send>);
    async fn delete_file(&mut self, file_name: &str);
    fn get_filenames(&self) -> Vec<&str>;
}

pub trait FileList: StatefulContainer + SelectableContainer<FileEntry> + FileCRUD {}
impl<T> FileList for T where T: StatefulContainer + SelectableContainer<FileEntry> + FileCRUD {}