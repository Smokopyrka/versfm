use std::{pin::Pin, sync::Arc};

use super::{
    err::ComponentError, BoxedByteStream, FileCRUD, FileEntry, ListEntry, SelectableContainer,
    State, StatefulContainer, TuiDisplay,
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

pub struct S3List {
    client: Arc<S3Provider>,
    s3_prefix: Option<String>,
    items: Vec<Box<ListEntry<S3Object>>>,
    state: ListState,
}

impl S3List {
    pub fn new(client: Arc<S3Provider>) -> S3List {
        S3List {
            client,
            s3_prefix: None,
            items: Vec::new(),
            state: ListState::default(),
        }
    }

    fn add_prefix(&self, to: &str) -> String {
        match &self.s3_prefix {
            Some(prefix) => format!("{}{}", prefix, to),
            None => String::from(to),
        }
    }

    fn handle_err(err: S3Error, file: Option<&str>) -> ComponentError {
        ComponentError::new(
            String::from("S3"),
            format!("(File: {}) {}", file.unwrap_or(""), err.message()),
            err.code().to_string(),
        )
    }
}

impl StatefulContainer for S3List {
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

impl SelectableContainer<Box<dyn FileEntry>> for S3List {
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
        let files: Vec<S3Object> = self
            .items
            .iter()
            .filter(|i| *i.selected() == selection)
            .map(|i| i.clone().value)
            .collect();
        let mut out: Vec<Box<dyn FileEntry>> = vec![];
        for file in files {
            out.push(Box::new(file));
        }
        out
    }
}

#[async_trait]
impl FileCRUD for S3List {
    async fn get_file_stream(
        &mut self,
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
        &mut self,
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

    async fn delete_file(&mut self, file_name: &str) -> Result<(), ComponentError> {
        self.client
            .delete_object(&self.add_prefix(file_name))
            .await
            .map_err(|e| Self::handle_err(e, Some(file_name)))?;
        Ok(())
    }

    async fn refresh(&mut self) -> Result<(), ComponentError> {
        let files: Vec<S3Object> = self
            .client
            .list_objects(self.s3_prefix.clone())
            .await
            .map_err(|e| Self::handle_err(e, None))?;
        self.items = files
            .into_iter()
            .map(|i| Box::new(ListEntry::new(i)))
            .collect();
        Ok(())
    }

    fn get_filenames(&self) -> Result<Vec<&str>, ComponentError> {
        Ok(self.items.iter().map(|i| i.value().name.as_str()).collect())
    }

    fn move_into_selected_dir(&mut self) {
        let mut current = self.s3_prefix.clone().unwrap_or(String::new());
        match self.state.selected() {
            None => (),
            Some(i) => {
                let selected = self.items[i].value.get_name();
                if selected.chars().last().unwrap() == '/' {
                    current.push_str(selected);
                    self.s3_prefix = Some(current);
                }
            }
        };
    }

    fn move_out_of_selected_dir(&mut self) {
        self.s3_prefix = match &self.s3_prefix {
            None => return,
            Some(prefix) => prefix
                .rmatch_indices('/')
                .nth(1)
                .map(|(i, _)| String::from(&prefix[..(i + 1)])),
        };
    }

    fn get_current_path(&self) -> String {
        match &self.s3_prefix {
            Some(prefix) => prefix.clone(),
            None => String::new(),
        }
    }

    fn get_resource_name(&self) -> String {
        self.client.bucket_name.clone()
    }
}

impl TuiDisplay for S3List {
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
        let items = super::transform_list(&self.items);
        List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    }
}
