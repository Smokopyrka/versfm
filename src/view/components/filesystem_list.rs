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

use crate::{
    providers::{
        filesystem::{self, FilesystemObject},
        Kind,
    },
    utils::{append_path_to_dir, split_path_into_dir_and_filename},
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

    fn set_item_state(&self, file_name: &str, state: State) {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        if let Some(item) = items.iter_mut().find(|v| v.value().get_name() == file_name) {
            item.select(state);
        }
    }

    fn remove_element_of_filename(&self, file_name: &str) {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        if let Some((element_index, _)) = items
            .iter()
            .enumerate()
            .find(|(_, v)| v.value().get_name() == file_name)
        {
            let mut state = self.state.lock().expect("Couldn't lock mutex");
            items.remove(element_index);
            // Modifies state to compensate for now removed item
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

    fn get_name_of_selected(&self) -> Option<String> {
        let items = self.items.lock().expect("Couldn't lock mutex");
        let state = self.state.lock().expect("Couldn't lock mutex");
        if let Some(i) = state.selected() {
            return Some(
                items
                    .get(i)
                    .expect("FilesystemList index out of range")
                    .value()
                    .get_name()
                    .to_owned(),
            );
        }
        None
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
        let items = self.items.lock().expect("Couldn't lock mutex");
        let mut state = self.state.lock().expect("Couldn't lock mutex");
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
        let items = self.items.lock().expect("Couldn't lock mutex");
        let mut state = self.state.lock().expect("Couldn't lock mutex");
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

impl SelectableContainer<String> for FilesystemList {
    fn select(&self, selection: State) {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        match self.get_current().selected() {
            None => (),
            Some(i) => {
                if items.len() > i {
                    match items[i].value().get_kind() {
                        Kind::File => items[i].select(selection),
                        Kind::Directory | Kind::Unknown => (),
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
    fn start_processing_item(&self, file_name: &str) {
        self.set_item_state(file_name, State::Proccessed);
    }

    fn stop_processing_item(&self, file_name: &str) {
        self.set_item_state(file_name, State::Unselected);
    }

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
        let (dir, file_name) = split_path_into_dir_and_filename(&path);
        let curr_path = self.curr_path.lock().expect("Couldn't lock mutex");
        if curr_path.to_str().unwrap() == dir {
            self.add_new_element(file_name);
        }
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<(), ComponentError> {
        filesystem::remove_file(Path::new(path)).map_err(|e| Self::handle_error(e, Some(path)))?;
        let (_, file_name) = split_path_into_dir_and_filename(path);
        self.remove_element_of_filename(file_name);
        Ok(())
    }

    async fn refresh(&self) -> Result<(), ComponentError> {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        *items = Self::get_list_entries(Path::new(&self.get_prefix()))?;
        Ok(())
    }

    fn move_out_of_selected_dir(&self) {
        let mut current = self.curr_path.lock().expect("Couldn't lock mutex");
        if let Some(path) = current.parent() {
            *current = path.to_path_buf();
        }
    }

    fn move_into_selected_dir(&self) {
        let mut curr_path = self.curr_path.lock().expect("Couldn't lock mutex");
        let current = curr_path.to_str().unwrap();
        if let Some(selected) = self.get_name_of_selected() {
            let path = append_path_to_dir(current, &selected);
            let path = Path::new(&path);
            let metadata = fs::metadata(path);
            if metadata.is_ok() && metadata.unwrap().is_dir() {
                *curr_path = path.to_path_buf();
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
