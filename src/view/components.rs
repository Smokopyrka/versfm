use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

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
    Proccessed,
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
        if new == State::Proccessed {
            self.state = new;
            return;
        }
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
    fn previous(&self);
    fn next(&self);
    fn get_current(&self) -> ListState;
}

pub trait SelectableContainer<T> {
    fn select(&self, selection: State);
    fn get_selected(&self, selection: State) -> Vec<T>;
}

#[async_trait]
pub trait FileCRUD {
    async fn refresh(&self) -> Result<(), ComponentError>;
    fn start_processing_item(&self, file_name: &str);
    fn stop_processing_item(&self, file_name: &str);
    async fn get_file_stream(
        &self,
        file_name: &str,
    ) -> Result<Pin<BoxedByteStream>, ComponentError>;
    async fn put_file(
        &self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError>;
    async fn delete_file(&self, file_name: &str) -> Result<(), ComponentError>;
    fn move_into_selected_dir(&self);
    fn move_out_of_selected_dir(&self);
    fn get_current_path(&self) -> String;
    fn get_resource_name(&self) -> &str;
}

pub trait TuiListDisplay {
    fn make_file_list(&self, is_focused: bool) -> List;
}

fn transform_list<'entry_life, T>(
    options: Arc<Mutex<Vec<SelectableEntry<T>>>>,
) -> Vec<ListItem<'entry_life>>
where
    T: FileEntry,
{
    options
        .lock()
        .expect("Couldn't lock mutex")
        .iter()
        .map(|o| {
            let mut text = o.value().get_name().to_owned();
            let mut style = Style::default();

            match o.value().get_kind() {
                Kind::Directory => style = style.add_modifier(Modifier::ITALIC),
                Kind::Unknown => style = style.fg(Color::Gray),
                _ => (),
            };
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
                    style = style.bg(Color::Green);
                    text.push_str(" [C]");
                }
                State::Proccessed => {
                    style = style.bg(Color::DarkGray);
                    text.push_str(" [/]");
                }
                _ => (),
            }
            ListItem::new(text).style(style)
        })
        .collect()
}

fn split_path_into_dir_and_filename(path: &str) -> (&str, &str) {
    let split: Vec<&str> = path.rsplitn(2, "/").collect();
    if split.len() != 2 {
        panic!("Path has no '/' separators in it");
    }
    return (split[1], split[0]);
}

pub trait FileList:
    StatefulContainer + SelectableContainer<String> + FileCRUD + TuiListDisplay
{
}
impl<T> FileList for T where
    T: StatefulContainer + SelectableContainer<String> + FileCRUD + TuiListDisplay
{
}
