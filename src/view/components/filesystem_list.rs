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
    err::ComponentError, BoxedByteStream, FileCRUD, FileEntry, SelectableContainer,
    SelectableEntry, State, StatefulContainer, TuiListDisplay,
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
    user: String,
    curr_path: PathBuf,
    items: Vec<SelectableEntry<FilesystemObject>>,
    state: ListState,
}

impl FilesystemList {
    pub fn new() -> FilesystemList {
        let curr_path = env::current_dir().unwrap();
        FilesystemList {
            user: whoami::username(),
            curr_path,
            items: Vec::new(),
            state: ListState::default(),
        }
    }

    fn add_prefix(&self, to: &str) -> String {
        format!("{}/{}", self.curr_path.to_str().unwrap(), to)
    }

    fn get_list_entries(
        path: &Path,
    ) -> Result<Vec<SelectableEntry<FilesystemObject>>, ComponentError> {
        Ok(filesystem::get_files_list(path)
            .map_err(|e| Self::handle_error(e, Some(path.as_os_str().to_str().unwrap())))?
            .into_iter()
            .map(SelectableEntry::new)
            .collect())
    }

    fn handle_error(err: io::Error, file: Option<&str>) -> ComponentError {
        let message = match err.kind() {
            io::ErrorKind::NotFound => "File couldn't be found",
            io::ErrorKind::PermissionDenied => {
                "Insufficient file permissions to perform this operation on file"
            }
            io::ErrorKind::AlreadyExists => "File already exists",
            io::ErrorKind::InvalidData => "File contains invalid data",
            io::ErrorKind::WriteZero | io::ErrorKind::UnexpectedEof => {
                "Operation was not able to complete"
            }
            io::ErrorKind::Unsupported => "This operation is not supported",
            _ => "Unexpected error occured",
        };
        ComponentError::new(
            String::from("Local Filesystem"),
            match file {
                None => message.to_owned(),
                Some(file_name) => format!("(File: {}) {}", file_name, message),
            },
            format!("{:?}", err.kind()),
        )
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

impl SelectableContainer<String> for FilesystemList {
    fn select(&mut self, selection: State) {
        match self.state.selected() {
            None => (),
            Some(i) => {
                let obj = &mut self.items[i];
                match obj.value().kind {
                    Kind::File => obj.select(selection),
                    Kind::Directory => (),
                }
            }
        };
    }

    fn get_selected(&self, selection: State) -> Vec<String> {
        self.items
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| i.value().get_name().to_owned())
            .collect()
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
                .map_err(|e| Self::handle_error(e, Some(file_name)))?,
        ))
    }

    async fn put_file(
        &mut self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError> {
        filesystem::write_file_from_stream(Path::new(&self.add_prefix(file_name)), stream)
            .await
            .map_err(|e| Self::handle_error(e, Some(file_name)))?;
        Ok(())
    }

    async fn delete_file(&mut self, file_name: &str) -> Result<(), ComponentError> {
        filesystem::remove_file(Path::new(&self.add_prefix(file_name)))
            .map_err(|e| Self::handle_error(e, Some(file_name)))?;
        Ok(())
    }

    async fn refresh(&mut self) -> Result<(), ComponentError> {
        self.items = Self::get_list_entries(&self.curr_path)?;
        Ok(())
    }

    fn get_filenames(&self) -> Result<Vec<&str>, ComponentError> {
        Ok(self.items.iter().map(|i| i.value().name.as_str()).collect())
    }

    async fn move_out_of_selected_dir(&mut self) -> Result<(), ComponentError> {
        let current = self.curr_path.as_path().to_owned();
        match current.parent() {
            Some(path) => self.curr_path = path.to_path_buf(),
            None => (),
        }
        match self.refresh().await {
            Err(err) => {
                self.curr_path = current.to_path_buf();
                self.move_into_selected_dir().await?;
                Err(err)
            }
            Ok(_) => Ok(()),
        }
    }

    async fn move_into_selected_dir(&mut self) -> Result<(), ComponentError> {
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
        match self.refresh().await {
            Err(err) => {
                self.move_out_of_selected_dir().await?;
                Err(err)
            }
            Ok(_) => {
                self.state.select(None);
                Ok(())
            }
        }
    }

    fn get_current_path(&self) -> &str {
        self.curr_path.to_str().unwrap()
    }

    fn get_resource_name(&self) -> &str {
        &self.user
    }
}

impl TuiListDisplay for FilesystemList {
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
