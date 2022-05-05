//! Module defining componenets that are later used when
//! composing screens
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

/// Enum representing various selection types an entry can be in
#[derive(Clone, PartialEq)]
pub enum State {
    Unselected,
    Proccessed,
    ToMove,
    ToDelete,
    ToCopy,
}

/// Struct containing a selectable value, and its current selection type (state)
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

    /// Gets the current value of the entry
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Gets the current selection type of the entry
    pub fn selected(&self) -> &State {
        &self.state
    }

    /// Selects the entry currently under cursor giving
    /// it the state provided in `new` function argument
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

/// Struct containing a filename, and an information whether
/// the file is a directory, or a regular file
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
    /// Selects the previous entry of the container
    fn previous(&self);
    /// Selects the next entry of the container
    fn next(&self);
    /// Retruns the current state of the container
    fn get_current(&self) -> ListState;
    /// Clears the current state of the container
    fn clear_state(&self);
}

pub trait SelectableContainer<T> {
    /// Selects given entry. Giving it the provided selection
    /// type
    fn select(&self, selection: State);
    /// Gets all the items that have a given selection type
    fn get_selected(&self, selection: State) -> Vec<T>;
}

pub trait ASelectableFilenameList:
    StatefulContainer + SelectableContainer<String> + Sync + Send
{
    fn lock_items(&self) -> MutexGuard<Vec<SelectableEntry<FilenameEntry>>>;
    fn lock_state(&self) -> MutexGuard<ListState>;

    /// Gets all the items that have a given selection type
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

    /// Sets the state of an item of given filename
    ///
    /// # Arguments
    /// * `file_name` - File name of the item the state should be set for
    /// * `state` - New state that should be set for the item
    fn set_item_state_by_filename(&self, file_name: &str, state: State) {
        let mut items = self.lock_items();
        if let Some(item) = items.iter_mut().find(|v| v.value().name() == file_name) {
            item.select(state);
        }
    }

    /// Obtains an item by its filename
    ///
    /// # Arguments
    /// * `file_name` - Filename of the item to obtain
    fn get_item_by_filename(&self, file_name: &str) -> Option<FilenameEntry> {
        let mut items = self.lock_items();
        if let Some(item) = items.iter_mut().find(|v| v.value().name() == file_name) {
            return Some(item.value().clone());
        }
        None
    }

    /// Deletes the element of given filename from the list
    ///
    /// # Arguments
    /// * `file_name` - Filename of the item to delete
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

    /// Adds a new element to the list
    ///
    /// # Arguments
    /// * `file_name` - Filename of the element to add
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
    /// Moves into the directory currently under cursor
    fn move_into_selected_dir(&self);
    /// Moves out of the current directory
    fn move_out_of_selected_dir(&self);
    /// Returns the path of the current directory
    fn get_current_path(&self) -> String;
}

#[async_trait]
pub trait FileCRUD {
    async fn refresh(&self) -> Result<(), ComponentError>;
    /// Signifies that the processing of a given item has begun
    ///
    /// # Arguments:
    ///
    /// * `file_name` - Name of the file that has started to be processed
    fn start_processing_item(&self, file_name: &str);
    /// Signifies that the processing of a given item has stopped
    ///
    /// # Arguments:
    ///
    /// * `file_name` - Name of the file that is no longer processed
    fn stop_processing_item(&self, file_name: &str);
    /// Obtains the file stream of the file with given filename
    ///
    /// # Arguments:
    ///
    /// * `file_name` - Name of the file the stream should be obtained for
    async fn get_file_stream(
        &self,
        file_name: &str,
    ) -> Result<Pin<BoxedByteStream>, ComponentError>;
    /// Saves given file from the provided file stream
    ///
    /// # Arguments:
    ///
    /// * `file_name` - Filename of the new file
    /// * `stream` - File Stream used to create the file
    async fn put_file(
        &self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError>;
    /// Deletes file of given filename
    ///
    /// # Arguments:
    ///
    /// * `file_name` - Filename of the file to be deleted
    async fn delete_file(&self, file_name: &str) -> Result<(), ComponentError>;
    /// Return the name of the resource FileCRUD is implemented over
    ///
    /// eg. name of the S3 bucket
    fn get_resource_name(&self) -> &str;
    /// Return the name of the provider FileCRUD is implemented over
    ///
    /// eg. 'S3', 'local', 'onedrive', etc.
    fn get_provider_name(&self) -> &str;
}

pub trait TuiListDisplay {
    /// Creates a styled list to be displayed by tui-rs
    ///
    /// # Arguments
    ///
    /// * `is_focused` - signifies whether the list that is
    /// generated is currently focused
    fn make_list(&self, is_focused: bool) -> List;
}

impl<T: ASelectableFilenameList + FileCRUD + Navigatable> TuiListDisplay for T {
    fn make_list(&self, is_focused: bool) -> List {
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

/// Helper function used to stylize the filename entries based on their properties
///
/// * `options` - A mutex guard to the list of selectable filename entries from which
/// to create the stylized list items
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
