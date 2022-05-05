use std::{
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
};

use super::{
    err::ComponentError, ASelectableFilenameList, BoxedByteStream, FileCRUD, FilenameEntry,
    Navigatable, SelectableEntry, State, StatefulContainer,
};
use crate::{
    providers::s3::{S3Error, S3Object, S3Provider},
    utils::{append_path_to_dir, split_path_into_dir_and_filename},
};

use async_trait::async_trait;
use futures::stream::Stream;
use rusoto_core::ByteStream;
use tui::widgets::ListState;

/// Interactive list of entries representing files in an S3 bucket
pub struct S3List {
    client: S3Provider,
    s3_prefix: Mutex<String>,
    items: Arc<Mutex<Vec<SelectableEntry<FilenameEntry>>>>,
    state: Arc<Mutex<ListState>>,
}

impl S3List {
    pub fn new(client: S3Provider) -> S3List {
        S3List {
            client,
            s3_prefix: Mutex::new(String::new()),
            items: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(Mutex::new(ListState::default())),
        }
    }

    fn lock_s3_prefix(&self) -> MutexGuard<String> {
        self.s3_prefix
            .lock()
            .expect("Couldn't lock s3_prefix mutex")
    }

    /// Maps given io::Error to a ComponentError
    ///
    /// * `err` - io::Error to map
    /// * `file` - OPTIONAL path to the file which caused the error
    fn handle_err(err: S3Error, file: Option<&str>) -> ComponentError {
        ComponentError::new(
            String::from("S3"),
            match file {
                None => err.message().to_owned(),
                Some(file_name) => format!("(File: {}) {}", file_name, err.message()),
            },
            err.code().to_owned(),
        )
    }
}

impl ASelectableFilenameList for S3List {
    fn lock_items(&self) -> MutexGuard<Vec<SelectableEntry<FilenameEntry>>> {
        self.items.lock().expect("Clouldn't lock items mutex")
    }
    fn lock_state(&self) -> MutexGuard<ListState> {
        self.state.lock().expect("Clouldn't lock state mutex")
    }
}

impl Navigatable for S3List {
    fn move_into_selected_dir(&self) {
        if let Some(mut dir) = self.get_name_of_selected() {
            let dir_last_char = dir.chars().last();
            if dir_last_char.is_none() || dir_last_char.unwrap() != '/' {
                return;
            }
            // Removes last '/' from directory name
            dir.pop();
            let mut s3_prefix = self.lock_s3_prefix();
            let new_prefix = append_path_to_dir(&s3_prefix, &dir);
            // [1..] is used here to remove the trailing '/' from the new_prefix
            *s3_prefix = new_prefix[1..].to_owned();
            self.clear_state();
        }
    }

    fn move_out_of_selected_dir(&self) {
        let mut s3_prefix = self.lock_s3_prefix();
        if !s3_prefix.is_empty() {
            *s3_prefix = s3_prefix
                .rmatch_indices('/')
                .nth(0)
                .map(|(i, _)| s3_prefix[..i].to_owned())
                .unwrap_or(String::new());
            self.clear_state();
        };
    }

    fn get_current_path(&self) -> String {
        self.lock_s3_prefix().to_owned()
    }
}

#[async_trait]
impl FileCRUD for S3List {
    fn get_resource_name(&self) -> &str {
        &self.client.bucket_name
    }

    fn get_provider_name(&self) -> &str {
        "S3"
    }

    fn start_processing_item(&self, file_name: &str) {
        self.set_item_state_by_filename(file_name, State::Proccessed);
    }

    fn stop_processing_item(&self, file_name: &str) {
        self.set_item_state_by_filename(file_name, State::Unselected);
    }

    async fn get_file_stream(&self, path: &str) -> Result<Pin<BoxedByteStream>, ComponentError> {
        Ok(Box::pin(
            // [1..] is used here to remove the trailing '/' from path
            self.client
                .download_object(&path[1..])
                .await
                .map_err(|e| Self::handle_err(e, Some(path)))?,
        ))
    }

    async fn put_file(
        &self,
        path: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError> {
        let size = stream.size_hint();
        if let None = size.1 {
            panic!("Stream must implement size hint in order to be be sent to S3");
        }
        // [1..] is used here to remove the trailing '/' from path
        let content = ByteStream::new_with_size(stream, size.0);
        self.client
            .put_object(&path[1..], content)
            .await
            .map_err(|e| Self::handle_err(e, Some(path)))?;
        let (dir, file_name) = split_path_into_dir_and_filename(&path);
        if self.get_current_path() == dir[1..] {
            self.add_new_element(&file_name);
        }
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<(), ComponentError> {
        // [1..] is used here to remove the trailing '/' from path
        self.client
            .delete_object(&path[1..])
            .await
            .map_err(|e| Self::handle_err(e, Some(path)))?;
        let (_, file_name) = split_path_into_dir_and_filename(path);
        self.remove_element_of_filename(file_name);
        Ok(())
    }

    async fn refresh(&self) -> Result<(), ComponentError> {
        let path = self.get_current_path();
        let files: Vec<S3Object> = self
            .client
            .list_objects(&path)
            .await
            .map_err(|e| Self::handle_err(e, Some(&self.get_current_path())))?;
        let mut items = self.lock_items();
        *items = files
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
