use std::{pin::Pin, sync::MutexGuard};

use async_trait::async_trait;

pub mod err;
mod filesystem_list;
mod s3_list;

pub use filesystem_list::FilesystemList;
pub use s3_list::S3List;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
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

#[derive(Clone)]
pub struct FilenameEntry {
    file_name: String,
    kind: Kind,
}

impl FilenameEntry {
    pub fn name(&self) -> &str {
        &self.file_name
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

pub trait StatefulContainer {
    fn previous(&self);
    fn next(&self);
    fn get_current(&self) -> ListState;
    fn clear_state(&self);
}

pub trait SelectableContainer<T> {
    fn select(&self, selection: State);
    fn get_selected(&self, selection: State) -> Vec<T>;
}

pub trait ASelectableFilenameList:
    StatefulContainer + SelectableContainer<String> + Sync + Send
{
    fn lock_items(&self) -> MutexGuard<Vec<SelectableEntry<FilenameEntry>>>;
    fn lock_state(&self) -> MutexGuard<ListState>;

    fn get_name_of_selected(&self) -> Option<String> {
        let items = self.lock_items();
        let state = self.lock_state();
        if let Some(i) = state.selected() {
            return Some(
                items
                    .get(i)
                    .expect("SelectableFilenameList index out of range")
                    .value()
                    .name()
                    .to_owned(),
            );
        }
        None
    }

    fn set_item_state_by_filename(&self, file_name: &str, state: State) {
        let mut items = self.lock_items();
        if let Some(item) = items.iter_mut().find(|v| v.value().name() == file_name) {
            item.select(state);
        }
    }

    fn get_item_by_filename(&self, file_name: &str) -> Option<FilenameEntry> {
        let mut items = self.lock_items();
        if let Some(item) = items.iter_mut().find(|v| v.value().name() == file_name) {
            return Some(item.value().clone());
        }
        None
    }

    fn remove_element_of_filename(&self, file_name: &str) {
        let mut items = self.lock_items();
        let mut state = self.lock_state();
        if let Some((element_index, _)) = items
            .iter()
            .enumerate()
            .find(|(_, v)| v.value().name() == file_name)
        {
            items.remove(element_index);
            if let Some(selected) = state.selected() {
                if element_index < selected {
                    state.select(Some(selected - 1));
                } else if element_index == selected {
                    state.select(None);
                }
            }
        }
    }

    fn add_new_element(&self, file_name: &str) {
        if self.get_item_by_filename(file_name).is_none() {
            let mut items = self.lock_items();
            items.push(SelectableEntry::new(FilenameEntry {
                file_name: file_name.to_owned(),
                kind: Kind::File,
            }));
        }
    }
}

impl<T: ASelectableFilenameList> StatefulContainer for T {
    fn get_current(&self) -> ListState {
        self.lock_state().clone()
    }

    fn clear_state(&self) {
        self.lock_state().select(None);
    }

    fn next(&self) {
        let items = self.lock_items();
        let mut state = self.lock_state();
        if items.len() > 0 {
            let i = match state.selected() {
                Some(i) => {
                    if i >= items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };

            state.select(Some(i));
        }
    }

    fn previous(&self) {
        let items = self.lock_items();
        let mut state = self.lock_state();
        if items.len() > 0 {
            let i = match state.selected() {
                Some(i) => {
                    if i == 0 {
                        items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };

            state.select(Some(i));
        }
    }
}

impl<T: ASelectableFilenameList> SelectableContainer<String> for T {
    fn select(&self, selection: State) {
        let mut items = self.lock_items();
        match self.get_current().selected() {
            None => (),
            Some(i) => {
                if items.len() > i {
                    match items[i].value().kind() {
                        Kind::File => items[i].select(selection),
                        Kind::Directory | Kind::Unknown => (),
                    }
                }
            }
        };
    }

    fn get_selected(&self, selection: State) -> Vec<String> {
        self.lock_items()
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| i.value().name().to_owned())
            .collect()
    }
}

pub trait Navigatable {
    fn move_into_selected_dir(&self);
    fn move_out_of_selected_dir(&self);
    fn get_current_path(&self) -> String;
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
    fn get_resource_name(&self) -> &str;
    fn get_provider_name(&self) -> &str;
}

pub trait TuiListDisplay {
    fn make_file_list(&self, is_focused: bool) -> List;
}

impl<T: ASelectableFilenameList + FileCRUD + Navigatable> TuiListDisplay for T {
    fn make_file_list(&self, is_focused: bool) -> List {
        let mut style = Style::default().fg(Color::White);
        if is_focused {
            style = style.fg(Color::LightBlue);
        }
        let block = Block::default()
            .title(format!(
                "{}@{}:{}",
                self.get_resource_name(),
                self.get_provider_name(),
                self.get_current_path()
            ))
            .style(style)
            .borders(Borders::ALL);
        let items = transform_list(self.lock_items());
        List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    }
}

fn transform_list(options: MutexGuard<Vec<SelectableEntry<FilenameEntry>>>) -> Vec<ListItem> {
    options
        .iter()
        .map(|o| {
            let mut text = o.value().name().to_owned();
            let mut style = Style::default();

            match o.value().kind() {
                Kind::Directory => style = style.add_modifier(Modifier::ITALIC),
                Kind::Unknown => style = style.fg(Color::DarkGray),
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
pub trait FileCRUDListWidget:
    ASelectableFilenameList + Navigatable + FileCRUD + TuiListDisplay
{
}
impl<T: ASelectableFilenameList + Navigatable + FileCRUD + TuiListDisplay> FileCRUDListWidget
    for T
{
}
