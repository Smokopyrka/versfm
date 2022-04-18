use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::{
    err::ComponentError, BoxedByteStream, FileCRUD, FileEntry, SelectableContainer,
    SelectableEntry, State, StatefulContainer, TuiListDisplay,
};
use crate::{
    providers::{
        s3::{S3Error, S3Object, S3Provider},
        Kind,
    },
    utils::{append_path_to_dir, split_path_into_dir_and_filename},
};

use async_trait::async_trait;
use futures::stream::Stream;
use rusoto_core::ByteStream;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListState},
};

impl FileEntry for S3Object {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_kind(&self) -> &Kind {
        &self.kind
    }
}

pub struct S3List {
    client: S3Provider,
    s3_prefix: Mutex<String>,
    items: Arc<Mutex<Vec<SelectableEntry<S3Object>>>>,
    state: Arc<Mutex<ListState>>,
}

impl S3List {
    fn len(&self) -> usize {
        self.items.lock().unwrap().len()
    }

    fn get_entry_name(&self, i: usize) -> String {
        self.items
            .lock()
            .expect("Couldn't lock mutex")
            .get(i)
            .expect("S3List: index out of range")
            .value()
            .get_name()
            .to_owned()
    }

    fn set_item_state(&self, file_name: &str, state: State) {
        let mut items = self.items.lock().expect("Couldn't lock mutex");
        if let Some(item) = items.iter_mut().find(|v| v.value().get_name() == file_name) {
            item.select(state);
        }
    }
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
        elements.push(SelectableEntry::new(S3Object {
            name: file_name.to_owned(),
            kind: Kind::File,
            prefix: String::from(""),
            last_mod: None,
            size: None,
            storage_class: None,
            owner: None,
        }));
    }

    fn get_prefix(&self) -> String {
        self.s3_prefix
            .lock()
            .expect("Couldn't lock mutex")
            .to_owned()
    }

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

impl StatefulContainer for S3List {
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
                .expect("Couldn't lock mutex")
                .select(Some(i));
        }
    }
}

impl SelectableContainer<String> for S3List {
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
            .unwrap()
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| i.value().get_name().to_owned())
            .collect()
    }
}

#[async_trait]
impl FileCRUD for S3List {
    fn start_processing_item(&self, file_name: &str) {
        self.set_item_state(file_name, State::Proccessed);
    }

    fn stop_processing_item(&self, file_name: &str) {
        self.set_item_state(file_name, State::Unselected);
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
        let path = self.get_prefix();
        let files: Vec<S3Object> = self
            .client
            .list_objects(&path)
            .await
            .map_err(|e| Self::handle_err(e, Some(&self.get_prefix())))?;
        let mut items = self.items.lock().expect("Could not lock mutex");
        *items = files.into_iter().map(|i| SelectableEntry::new(i)).collect();
        Ok(())
    }

    fn move_into_selected_dir(&self) {
        let state = self.state.lock().expect("Couldn't lock mutex");
        if let Some(i) = state.selected() {
            let mut dir = self.get_entry_name(i);
            // Removes last '/' from directory name
            dir.pop();
            let mut s3_prefix = self.s3_prefix.lock().expect("Couldn't lock mutex");
            let new_prefix = append_path_to_dir(&s3_prefix, &dir);
            // [1..] is used here to remove the trailing '/' from new_prefix
            *s3_prefix = new_prefix[1..].to_owned();
        }
    }

    fn move_out_of_selected_dir(&self) {
        let mut s3_prefix = self.s3_prefix.lock().expect("Couldn't lock mutex");
        if !s3_prefix.is_empty() {
            *s3_prefix = s3_prefix
                .rmatch_indices('/')
                .nth(0)
                .map(|(i, _)| s3_prefix[..i].to_owned())
                .unwrap_or(String::from(""));
        };
    }

    fn get_current_path(&self) -> String {
        self.get_prefix()
    }

    fn get_resource_name(&self) -> &str {
        &self.client.bucket_name
    }
}

impl TuiListDisplay for S3List {
    fn make_file_list(&self, is_focused: bool) -> List {
        let mut style = Style::default().fg(Color::White);
        if is_focused {
            style = style.fg(Color::LightBlue);
        }
        let block = Block::default()
            .title(format!(
                "{}@S3:/{}",
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
