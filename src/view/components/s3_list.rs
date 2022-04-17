use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::{
    err::ComponentError, BoxedByteStream, FileCRUD, FileEntry, SelectableContainer,
    SelectableEntry, State, StatefulContainer, TuiListDisplay,
};
use crate::providers::{
    s3::{S3Error, S3Object, S3Provider},
    Kind,
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
    s3_prefix: String,
    items: Arc<Mutex<Vec<SelectableEntry<S3Object>>>>,
    state: ListState,
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
}

impl S3List {
    pub fn new(client: S3Provider) -> S3List {
        S3List {
            client,
            s3_prefix: String::new(),
            items: Arc::new(Mutex::new(Vec::new())),
            state: ListState::default(),
        }
    }

    fn add_prefix(&self, to: &str) -> String {
        format!("{}{}", self.s3_prefix, to)
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
        self.state.clone()
    }

    fn next(&mut self) {
        if self.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.len() - 1 {
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
        if self.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.len() - 1
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

impl SelectableContainer<String> for S3List {
    fn select(&mut self, selection: State) {
        match self.state.selected() {
            None => (),
            Some(i) => {
                let mut items = self.items.lock().expect("Couldn't lock mutex");
                match items[i].value().get_kind() {
                    Kind::File => items[i].select(selection),
                    Kind::Directory => (),
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
    async fn get_file_stream(
        &self,
        file_name: &str,
    ) -> Result<Pin<BoxedByteStream>, ComponentError> {
        Ok(Box::pin(
            self.client
                .download_object(&self.add_prefix(file_name))
                .await
                .map_err(|e| Self::handle_err(e, Some(file_name)))?,
        ))
    }

    async fn put_file(
        &self,
        file_name: &str,
        stream: Pin<BoxedByteStream>,
    ) -> Result<(), ComponentError> {
        let size = stream.size_hint();
        if let None = size.1 {
            panic!("Stream must implement size hint in order to be be sent to S3");
        }
        let content = ByteStream::new_with_size(stream, size.0);
        self.client
            .put_object(&self.add_prefix(file_name), content)
            .await
            .map_err(|e| Self::handle_err(e, Some(file_name)))?;
        Ok(())
    }

    async fn delete_file(&self, file_name: &str) -> Result<(), ComponentError> {
        self.client
            .delete_object(&self.add_prefix(file_name))
            .await
            .map_err(|e| Self::handle_err(e, Some(file_name)))?;
        Ok(())
    }

    async fn refresh(&self) -> Result<(), ComponentError> {
        let files: Vec<S3Object> = self
            .client
            .list_objects(&self.s3_prefix)
            .await
            .map_err(|e| Self::handle_err(e, Some(&self.s3_prefix)))?;
        let mut items = self.items.lock().expect("Could not lock mutex");
        *items = files.into_iter().map(|i| SelectableEntry::new(i)).collect();
        Ok(())
    }

    async fn move_into_selected_dir(&mut self) -> Result<(), ComponentError> {
        match self.state.selected() {
            None => (),
            Some(i) => {
                let selected = self.get_entry_name(i);
                if selected.chars().last().unwrap() == '/' {
                    self.s3_prefix.push_str(&selected);
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

    async fn move_out_of_selected_dir(&mut self) -> Result<(), ComponentError> {
        let current = self.s3_prefix.to_owned();
        if !current.is_empty() {
            self.s3_prefix = current
                .rmatch_indices('/')
                .nth(1)
                .map(|(i, _)| current[..(i + 1)].to_owned())
                .unwrap_or(String::from(""));
        };
        match self.refresh().await {
            Err(err) => {
                self.s3_prefix = current;
                self.move_into_selected_dir().await?;
                Err(err)
            }
            Ok(_) => Ok(()),
        }
    }

    fn get_current_path(&self) -> &str {
        &self.s3_prefix
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
