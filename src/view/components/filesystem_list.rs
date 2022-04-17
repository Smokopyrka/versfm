use std::{
    env, fs, io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
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
    err::ComponentError, get_filename_from_path, BoxedByteStream, FileCRUD, FileEntry,
    SelectableContainer, SelectableEntry, State, StatefulContainer, TuiListDisplay,
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
    curr_path: Arc<Mutex<PathBuf>>,
    items: Arc<Mutex<Vec<SelectableEntry<FilesystemObject>>>>,
    state: Arc<Mutex<ListState>>,
}

impl FilesystemList {
    pub fn new() -> FilesystemList {
        let curr_path = env::current_dir().unwrap();
        FilesystemList {
            user: whoami::username(),
            curr_path: Arc::new(Mutex::new(curr_path)),
            items: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(Mutex::new(ListState::default())),
        }
    }

    fn remove_element_of_filename(&self, file_name: &str) {
        let mut elements = self.items.lock().expect("Couldn't lock mutex");
        let mut state = self.state.lock().expect("Couldn't lock mutex");
        if let Some((element_index, _)) = elements
            .iter()
            .enumerate()
            .find(|(_, v)| v.value().get_name() == file_name)
        {
            elements.remove(element_index);
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
        let mut elements = self.items.lock().expect("Couldn't lock mutex");
        elements.push(SelectableEntry::new(FilesystemObject {
            name: file_name.to_owned(),
            dir: None,
            kind: Kind::File,
        }));
    }

    fn len(&self) -> usize {
        self.items.lock().expect("Couldn't lock mutex").len()
    }

    fn get_entry_name(&self, i: usize) -> String {
        self.items
            .lock()
            .expect("Coldn't lock mutex")
            .get(i)
            .expect("FilesystemList index out of range")
            .value()
            .get_name()
            .to_owned()
    }

    fn get_prefix(&self) -> String {
        self.curr_path
            .lock()
            .expect("Couldn't lock mutex")
            .to_str()
            .unwrap()
            .to_owned()
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
        self.state.lock().expect("Couldn't lock mutex").clone()
    }

    fn next(&self) {
        if self.len() > 0 {
            let i = match self.get_current().selected() {
                Some(i) => {
                    if i >= self.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };

            self.state
                .lock()
                .expect("Couldn't lock mutex")
                .select(Some(i));
        }
    }

    fn previous(&self) {
        if self.len() > 0 {
            let i = match self.get_current().selected() {
                Some(i) => {
                    if i == 0 {
                        self.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };

            self.state
                .lock()
                .expect("Coudln't lock mutex")
                .select(Some(i));
        }
    }
}

impl SelectableContainer<String> for FilesystemList {
    fn select(&self, selection: State) {
        match self.get_current().selected() {
            None => (),
            Some(i) => {
                let mut items = self.items.lock().expect("Couldn't lock mutex");
                if items.len() > i && i > 0 {
                    match items[i].value().get_kind() {
                        Kind::File => items[i].select(selection),
                        Kind::Directory => (),
                    }
                }
            }
        };
    }

    fn get_selected(&self, selection: State) -> Vec<String> {
        self.items
            .lock()
            .expect("Coldn't lock mutex")
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| i.value().get_name().to_owned())
            .collect()
    }
}

#[async_trait]
impl FileCRUD for FilesystemList {
    async fn get_file_stream(&self, path: &str) -> Result<Pin<BoxedByteStream>, ComponentError> {
        Ok(Box::pin(
            filesystem::get_file_byte_stream(Path::new(path))
                .map_err(|e| Self::handle_error(e, Some(path)))?,
        ))
    }

    async fn put_file(
        &self,
        path: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError> {
        filesystem::write_file_from_stream(Path::new(path), stream)
            .await
            .map_err(|e| Self::handle_error(e, Some(path)))?;
        let path_dir = path.rsplitn(2, "/").last().unwrap();
        let mut curr_dir = self.get_current_path();
        if curr_dir == path_dir {
            self.add_new_element(get_filename_from_path(path));
        }
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<(), ComponentError> {
        filesystem::remove_file(Path::new(path)).map_err(|e| Self::handle_error(e, Some(path)))?;
        self.remove_element_of_filename(get_filename_from_path(path));
        Ok(())
    }

    async fn refresh(&self) -> Result<(), ComponentError> {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        *items = Self::get_list_entries(Path::new(&self.get_prefix()))?;
        Ok(())
    }

    async fn move_out_of_selected_dir(&self) -> Result<(), ComponentError> {
        let current = self.get_prefix();
        let current = Path::new(&current);
        let curr_path = self.curr_path.clone();
        match current.parent() {
            Some(path) => {
                let mut curr_path = curr_path.lock().expect("Couldn't lock mutex");
                *curr_path = path.to_path_buf();
                Ok(())
            }
            None => Ok(()),
        }
        // let curr_path = self.curr_path.clone();
        // match self.refresh().await {
        //     Err(err) => {
        //         let curr_path = curr_path.lock().expect("Couldn't lock mutex");
        //         *curr_path = current.to_path_buf();
        //         self.move_into_selected_dir().await?;
        //         Err(err)
        //     }
        //     Ok(_) => Ok(()),
        // }
    }

    async fn move_into_selected_dir(&self) -> Result<(), ComponentError> {
        let current = self.get_prefix();
        match self.get_current().selected() {
            None => (),
            Some(i) => {
                let selected = self.get_entry_name(i);

                let path;
                if current.chars().last().unwrap() == '/' {
                    path = format!("{}{}", &current, selected);
                } else {
                    path = format!("{}/{}", &current, selected);
                }
                let path = Path::new(&path);

                if fs::metadata(path).unwrap().is_dir() {
                    let mut curr_path = self.curr_path.lock().expect("Couldn't lock mutex");
                    *curr_path = path.to_path_buf();
                }
            }
        };
        match self.refresh().await {
            Err(err) => {
                self.move_out_of_selected_dir().await?;
                Err(err)
            }
            Ok(_) => {
                self.state.lock().expect("Coudn't lock mutex").select(None);
                Ok(())
            }
        }
    }

    fn get_current_path(&self) -> String {
        self.get_prefix()
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
        let items = super::transform_list(self.items.clone());
        List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    }
}
