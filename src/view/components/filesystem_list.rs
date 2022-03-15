use std::{
    env, fs, io,
    path::{Path, PathBuf},
    pin::Pin,
};

use async_trait::async_trait;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListState},
};

use crate::providers::{
    filesystem::{self, FilesystemObject},
    Kind,
};

use super::{
    err::ComponentError, BoxedByteStream, FileCRUD, FileEntry, ListEntry, SelectableContainer,
    State, StatefulContainer, TuiDisplay,
};

impl FileEntry for FilesystemObject {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_kind(&self) -> &Kind {
        &self.kind
    }
}

pub struct FilesystemList {
    curr_path: PathBuf,
    items: Vec<Box<ListEntry<FilesystemObject>>>,
    state: ListState,
}

impl FilesystemList {
    pub fn new() -> FilesystemList {
        let curr_path = env::current_dir().unwrap();
        let items = Self::get_list_entries(&curr_path);
        FilesystemList {
            curr_path,
            items,
            state: ListState::default(),
        }
    }

    fn add_prefix(&self, to: &str) -> String {
        format!("{}/{}", self.curr_path.to_str().unwrap(), to)
    }

    fn get_list_entries(path: &Path) -> Vec<Box<ListEntry<FilesystemObject>>> {
        filesystem::get_files_list(path)
            .unwrap()
            .into_iter()
            .map(|i| Box::new(ListEntry::new(i)))
            .collect()
    }

    fn handle_error(err: io::Error) -> ComponentError {
        let message = match err.kind() {
            io::ErrorKind::NotFound => "File couldn't be found",
            io::ErrorKind::PermissionDenied => {
                "Insufficient file permissions to perform this operation"
            }
            io::ErrorKind::AlreadyExists => "File already exists",
            io::ErrorKind::InvalidData => "File contains invalid data",
            io::ErrorKind::WriteZero | io::ErrorKind::UnexpectedEof => {
                "Operation was not able to complete"
            }
            io::ErrorKind::Unsupported => "This operation is not supported",
            _ => "Unexpected error occured",
        };

        ComponentError::new(message.to_string(), format!("{:?}", err.kind()))
    }
}

impl StatefulContainer for FilesystemList {
    fn get_current(&self) -> ListState {
        self.state.clone()
    }

    fn next(&mut self) {
        if self.items.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };

            self.state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        if self.items.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };

            self.state.select(Some(i));
        }
    }
}

impl SelectableContainer<Box<dyn FileEntry>> for FilesystemList {
    fn get(&self, i: usize) -> ListEntry<Box<dyn FileEntry>> {
        ListEntry::from(self.items[i].clone())
    }

    fn get_items(&self) -> Vec<ListEntry<Box<dyn FileEntry>>> {
        self.items
            .iter()
            .map(|i| ListEntry::from(i.clone()))
            .collect()
    }

    fn select(&mut self, selection: State) {
        match self.state.selected() {
            None => (),
            Some(i) => {
                let obj = self.items.get_mut(i).unwrap();
                match obj.value().kind {
                    Kind::File => obj.select(selection),
                    Kind::Directory => (),
                }
            }
        };
    }

    fn get_selected(&mut self, selection: State) -> Vec<Box<dyn FileEntry>> {
        let files: Vec<Box<FilesystemObject>> = self
            .items
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| Box::new(i.value.clone()))
            .collect();
        let mut out: Vec<Box<dyn FileEntry>> = vec![];
        for file in files {
            out.push(file);
        }
        out
    }
}

#[async_trait]
impl FileCRUD for FilesystemList {
    async fn get_file_stream(
        &mut self,
        file_name: &str,
    ) -> Result<Pin<BoxedByteStream>, ComponentError> {
        Ok(Box::pin(
            filesystem::get_file_byte_stream(Path::new(&self.add_prefix(file_name)))
                .map_err(Self::handle_error)?,
        ))
    }

    async fn put_file(
        &mut self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError> {
        filesystem::write_file_from_stream(Path::new(&self.add_prefix(file_name)), stream)
            .await
            .map_err(Self::handle_error)?;
        Ok(())
    }

    async fn delete_file(&mut self, file_name: &str) -> Result<(), ComponentError> {
        filesystem::remove_file(Path::new(&self.add_prefix(file_name)))
            .map_err(Self::handle_error)?;
        Ok(())
    }

    async fn refresh(&mut self) -> Result<(), ComponentError> {
        self.items = Self::get_list_entries(&self.curr_path);
        Ok(())
    }

    fn get_filenames(&self) -> Result<Vec<&str>, ComponentError> {
        Ok(self.items.iter().map(|i| i.value().name.as_str()).collect())
    }

    fn move_out_of_selected_dir(&mut self) {
        match self.curr_path.as_path().parent() {
            Some(path) => self.curr_path = path.to_path_buf(),
            None => (),
        }
    }

    fn move_into_selected_dir(&mut self) {
        let current = self.curr_path.to_str().unwrap();
        match self.state.selected() {
            None => (),
            Some(i) => {
                let selected = self.items[i].value.get_name();
                let path;
                if current.chars().last().unwrap() == '/' {
                    path = format!("{}{}", current, selected);
                } else {
                    path = format!("{}/{}", current, selected);
                }
                let path = Path::new(&path);
                if fs::metadata(path).unwrap().is_dir() {
                    self.curr_path = path.to_path_buf();
                }
            }
        };
    }

    fn get_current_path(&self) -> String {
        String::from(self.curr_path.to_str().unwrap())
    }

    fn get_resource_name(&self) -> String {
        whoami::username()
    }
}

impl TuiDisplay for FilesystemList {
    fn make_file_list(&self, is_focused: bool) -> List {
        let mut style = Style::default().fg(Color::White);
        if is_focused {
            style = style.fg(Color::LightBlue);
        }
        let block = Block::default()
            .title(format!(
                "{}@local:{}",
                self.get_resource_name(),
                self.get_current_path()
            ))
            .style(style)
            .borders(Borders::ALL);
        let items = super::transform_list(&self.items);
        List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    }
}
