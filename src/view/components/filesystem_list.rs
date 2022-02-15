use std::{path::{PathBuf, Path}, env, fs};

use async_trait::async_trait;
use tokio_stream::Stream;
use tui::widgets::ListState;

use crate::aws::{Kind};

use super::{ListEntry, State, SelectableContainer, FileCRUD, StatefulContainer, FileEntry};

#[derive(Clone)]
pub struct FilesystemObject {
    pub name: String,
    pub dir: Option<PathBuf>,
    pub kind: Kind,
}

pub struct FilesystemList {
    curr_path: PathBuf,
    items: Vec<ListEntry<FilesystemObject>>,
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

    fn get_list_entries(path: &Path) -> Vec<ListEntry<FilesystemObject>> {
        let files: Vec<FilesystemObject> = fs::read_dir(path)
            .unwrap()
            .map(|f| {
                let path = f.unwrap().path();
                let mut file_name = String::from(path.file_name().unwrap().to_str().unwrap());
                let kind: Kind;
                if !fs::metadata(&path).unwrap().is_file() {
                    file_name.push_str("/");
                    kind = Kind::Directory
                } else {
                    kind = Kind::File;
                }
                FilesystemObject {
                    name: file_name,
                    dir: path.parent().and_then(|p| Some(p.to_path_buf())),
                    kind: kind,
                }
            })
            .collect();
            files
                .into_iter()
                .map(|i| ListEntry::new(i))
                .collect()
    }
}

impl StatefulContainer for FilesystemList {

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

impl SelectableContainer<FileEntry> for FilesystemList {

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
impl FileCRUD for FilesystemList {

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
        let files: Vec<FilesystemObject> = fs::read_dir(&self.curr_path)
            .unwrap()
            .map(|f| {
                let path = f.unwrap().path();
                let mut file_name = String::from(path.file_name().unwrap().to_str().unwrap());
                let kind: Kind;
                if !fs::metadata(&path).unwrap().is_file() {
                    file_name.push_str("/");
                    kind = Kind::Directory
                } else {
                    kind = Kind::File;
                }
                FilesystemObject {
                    name: file_name,
                    dir: path.parent().and_then(|p| Some(p.to_path_buf())),
                    kind: kind,
                }
            })
            .collect();
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