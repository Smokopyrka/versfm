use std::{io, pin::Pin};

use async_trait::async_trait;
use bytes::Bytes;
use tokio_stream::Stream;

pub mod err;
mod filesystem_list;
mod s3list;

pub use filesystem_list::FilesystemList;
pub use s3list::S3List;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState},
};

use crate::providers::Kind;

use self::err::ComponentError;

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
where
    T: FileEntry + 'static,
{
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

pub type BoxedByteStream = Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + 'static>;

#[async_trait]
pub trait FileCRUD {
    async fn refresh(&mut self) -> Result<(), ComponentError>;
    async fn get_file_stream(
        &mut self,
        file_name: &str,
    ) -> Result<Pin<BoxedByteStream>, ComponentError>;
    async fn put_file(
        &mut self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError>;
    async fn delete_file(&mut self, file_name: &str) -> Result<(), ComponentError>;
    fn get_filenames(&self) -> Result<Vec<&str>, ComponentError>;
    fn move_into_selected_dir(&mut self);
    fn move_out_of_selected_dir(&mut self);
    fn get_current_path(&self) -> String;
    fn get_resource_name(&self) -> String;
}

pub trait TuiDisplay {
    fn make_file_list(&self, is_focused: bool) -> List;
    // fn transform_list(options: &[ListEntry<Box<dyn FileEntry>>]) -> Vec<ListItem> {
}

fn transform_list<T>(options: &[Box<ListEntry<T>>]) -> Vec<ListItem>
where
    T: FileEntry,
{
    options
        .iter()
        .map(|o| {
            let mut text = String::from(o.value().get_name());
            let mut style = Style::default();

            if let Kind::Directory = o.value().get_kind() {
                style = style.add_modifier(Modifier::ITALIC);
            }
            match o.selected() {
                State::ToMove => {
                    style = style.bg(Color::LightBlue);
                    text.push_str(" [M]");
                }
                State::ToDelete => {
                    style = style.bg(Color::Red);
                    text.push_str(" [D]");
                }
                State::ToCopy => {
                    style = style.bg(Color::LightGreen);
                    text.push_str(" [C]");
                }
                _ => (),
            }
            ListItem::new(text).style(style)
        })
        .collect()
}

pub trait FileList:
    StatefulContainer + SelectableContainer<Box<dyn FileEntry>> + FileCRUD + TuiDisplay
{
}
impl<T> FileList for T where
    T: StatefulContainer + SelectableContainer<Box<dyn FileEntry>> + FileCRUD + TuiDisplay
{
}
