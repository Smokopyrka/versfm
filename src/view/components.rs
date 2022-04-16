use std::pin::Pin;

use async_trait::async_trait;

pub mod err;
mod filesystem_list;
mod s3_list;

pub use filesystem_list::FilesystemList;
pub use s3_list::S3List;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState},
};

use crate::providers::{BoxedByteStream, Kind};

use self::err::ComponentError;

#[derive(Clone, PartialEq)]
pub enum State {
    Unselected,
    ToMove,
    ToDelete,
    ToCopy,
}

pub struct SelectableEntry<T> {
    value: T,
    state: State,
}

impl<T> SelectableEntry<T> {
    fn new(value: T) -> SelectableEntry<T> {
        SelectableEntry {
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
    fn get_selected(&self, selection: State) -> Vec<T>;
}

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
    async fn move_into_selected_dir(&mut self) -> Result<(), ComponentError>;
    async fn move_out_of_selected_dir(&mut self) -> Result<(), ComponentError>;
    fn get_current_path(&self) -> &str;
    fn get_resource_name(&self) -> &str;
}

pub trait TuiListDisplay {
    fn make_file_list(&self, is_focused: bool) -> List;
}

fn transform_list<T>(options: &[SelectableEntry<T>]) -> Vec<ListItem>
where
    T: FileEntry,
{
    options
        .iter()
        .map(|o| {
            let mut text = o.value().get_name().to_owned();
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
    StatefulContainer + SelectableContainer<String> + FileCRUD + TuiListDisplay
{
}
impl<T> FileList for T where
    T: StatefulContainer + SelectableContainer<String> + FileCRUD + TuiListDisplay
{
}
