use std::{pin::Pin, io};

use bytes::Bytes;
use tokio_stream::Stream;
use async_trait::async_trait;

mod s3list;
mod filesystem_list;

pub use s3list::S3List;
pub use filesystem_list::{FilesystemList};
use tui::widgets::ListState;

use crate::providers::{Kind};

#[derive(Clone, PartialEq)]
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
            value: value,
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

impl<T> From<Box<ListEntry<T>>> for ListEntry<Box<dyn FileEntry>>
where T: FileEntry + 'static {
    fn from(entry: Box<ListEntry<T>>) -> Self {
        ListEntry {
            value: Box::new(entry.value),
            state: entry.state,
        }
    }
}

pub trait FileEntry {
    fn get_name(&self) -> &str;
    fn get_kind(&self) -> &Kind;
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

pub type BoxedByteStream = Box<dyn Stream<Item=Result<Bytes, io::Error>> + Send + 'static>;

#[async_trait]
pub trait FileCRUD {
    async fn refresh(&mut self);
    async fn get_file_stream(&mut self, file_name: &str) -> Pin<BoxedByteStream>;
    async fn put_file(&mut self, file_name: &str, stream: Pin<BoxedByteStream>);
    async fn delete_file(&mut self, file_name: &str);
    fn get_filenames(&self) -> Vec<&str>;
    fn move_into_selected_dir(&mut self);
    fn move_out_of_selected_dir(&mut self);
    fn get_current_path(&self) -> String;
    fn get_resource_name(&self) -> String;
}

pub trait FileList: StatefulContainer + SelectableContainer<Box<dyn FileEntry>> + FileCRUD {}
impl<T> FileList for T where T: StatefulContainer + SelectableContainer<Box<dyn FileEntry>> + FileCRUD {}