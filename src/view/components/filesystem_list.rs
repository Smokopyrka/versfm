use std::{
    env, fs, io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
};

use async_trait::async_trait;
use tui::widgets::ListState;

use crate::{
    providers::filesystem,
    utils::{append_path_to_dir, split_path_into_dir_and_filename},
};

use super::{
    err::ComponentError, ASelectableFilenameList, BoxedByteStream, FileCRUD, FilenameEntry,
    Navigatable, SelectableEntry, State, StatefulContainer,
};

/// Interactive list of entries representing files in the local filesystem
pub struct FilesystemList {
    user: String,
    curr_path: Arc<Mutex<PathBuf>>,
    items: Arc<Mutex<Vec<SelectableEntry<FilenameEntry>>>>,
    state: Arc<Mutex<ListState>>,
}

impl FilesystemList {
    pub fn new() -> FilesystemList {
        let curr_path = env::current_dir().expect("Couldn't obtain path of the current directory");
        FilesystemList {
            user: whoami::username(),
            curr_path: Arc::new(Mutex::new(curr_path)),
            items: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(Mutex::new(ListState::default())),
        }
    }

    fn lock_curr_path(&self) -> MutexGuard<PathBuf> {
        self.curr_path
            .lock()
            .expect("Couldn't lock curr_path mutex")
    }

    /// Maps given io::Error to a ComponentError
    ///
    /// * `err` - io::Error to map
    /// * `file` - OPTIONAL path to the file which caused the error
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
            _ => "Unexpected error ocurred",
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

impl ASelectableFilenameList for FilesystemList {
    fn lock_items(&self) -> MutexGuard<Vec<SelectableEntry<FilenameEntry>>> {
        self.items.lock().expect("Couldn't lock items mutex")
    }

    fn lock_state(&self) -> MutexGuard<ListState> {
        self.state.lock().expect("Couldn't lock state mutex")
    }
}

impl Navigatable for FilesystemList {
    fn move_out_of_selected_dir(&self) {
        let mut curr_path = self.lock_curr_path();
        if let Some(parent_path) = curr_path.parent() {
            *curr_path = parent_path.to_path_buf();
            self.clear_state();
        }
    }

    fn move_into_selected_dir(&self) {
        let mut curr_path = self.lock_curr_path();
        let curr_path_str = curr_path
            .to_str()
            .expect("Couldn't convert current path to string");
        if let Some(selected) = self.get_name_of_selected() {
            let mut new_path = append_path_to_dir(curr_path_str, &selected);
            // Removes last '/' from directory name
            new_path.pop();
            let new_path = Path::new(&new_path);
            let metadata = fs::metadata(new_path);
            if metadata.is_ok() && metadata.unwrap().is_dir() {
                *curr_path = new_path.to_path_buf();
            }
            self.clear_state();
        }
    }

    fn get_current_path(&self) -> String {
        self.lock_curr_path().to_str().unwrap().to_owned()
    }
}

#[async_trait]
impl FileCRUD for FilesystemList {
    fn get_resource_name(&self) -> &str {
        &self.user
    }

    fn get_provider_name(&self) -> &str {
        "local"
    }

    fn start_processing_item(&self, file_name: &str) {
        self.set_item_state_by_filename(file_name, State::Processed);
    }

    fn stop_processing_item(&self, file_name: &str) {
        self.set_item_state_by_filename(file_name, State::Unselected);
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
        let curr_path = self.lock_curr_path();
        if curr_path
            .to_str()
            .expect("Couldn't convert current path to string")
            == dir
        {
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
        let path = &self.get_current_path();
        let mut items = self.lock_items();
        *items = filesystem::get_files_list(Path::new(path))
            .map_err(|e| Self::handle_error(e, Some(path)))?
            .into_iter()
            .map(|i| {
                SelectableEntry::new(FilenameEntry {
                    file_name: i.name,
                    kind: i.kind,
                })
            })
            .collect();
        Ok(())
    }
}
