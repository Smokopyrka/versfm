use crate::aws::{s3::{Cli, S3Object}, Kind};
use super::{FileCRUD, SelectableContainer, StatefulContainer, State, ListEntry, FileEntry};

use async_trait::async_trait;
use tui::widgets::ListState;
use tokio_stream::{self, Stream};

pub struct S3List<'clilife> {
    client: &'clilife Cli,
    s3_prefix: Option<String>,
    items: Vec<ListEntry<S3Object>>,
    state: ListState,
}

impl<'clilife> S3List<'clilife> {
    pub fn new(client: &'clilife Cli) -> S3List {
        S3List {
            client,
            s3_prefix: None,
            items: Vec::new(),
            state: ListState::default(),
        }
    }

    pub fn get_bucket_name(&self) -> &str {
        &self.client.bucket_name
    }
}

impl<'clilife> StatefulContainer for S3List<'clilife> {

    fn get_current(&self) -> ListState {
        self.state.clone()
    }

    // fn reset_cursor(&mut self) {
    //     if self.items.len() > 0 {
    //         self.state.select(Some(0));
    //     } else {
    //         self.state.select(None);
    //     }
    // }

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

impl<'clilife> SelectableContainer<FileEntry> for S3List<'clilife> {

    fn get(&self, i: usize) -> ListEntry<FileEntry> {
        ListEntry::from(&self.items[i])
    }

    fn get_items(&self) -> Vec<ListEntry<FileEntry>> {
        self.items.iter().map(|i| ListEntry::from(i)).collect()
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

    fn get_selected(&mut self, selection: State) -> Vec<FileEntry> {
        self.get_items()
            .into_iter()
            .filter(|i| matches!(i.selected(), selection))
            .map(|i| i.value)
            .collect()
    }
}

#[async_trait]
impl<'clilife> FileCRUD for S3List<'clilife> {

    async fn get_file_stream(&mut self, file_name: &str) -> Box<dyn Stream<Item=u8>> {
        let items: Vec<u8> = vec![1, 2, 3];
        Box::new(tokio_stream::iter(items))
    }

    async fn put_file(&mut self, file_name: &str, stream: Box<dyn Stream<Item=u8> + Send>) {
        println!("dummy");
    }

    async fn delete_file(&mut self, file_name: &str) {
        ()
    }
    
    async fn refresh(&mut self) {
        let files: Vec<S3Object> = self.client.list_objects(self.s3_prefix.clone()).await;
        self.items = files
            .into_iter()
            .map(|i| ListEntry::new(i))
            .collect()
    }

    fn get_filenames(&self) -> Vec<&str> {
        self.items
            .iter()
            .map(|i| i.value().name.as_str())
            .collect()
    }

}