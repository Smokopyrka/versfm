use crossterm::{
    event::{DisableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::{
    env,
    error::Error,
    fs::{self, read_dir},
    io::Stdout,
    path::{Path, PathBuf},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use crate::view::utils::{ListOption, State, StatefulList};
use crate::{aws::s3::Cli, view::utils::Kind};

enum CurrentList {
    LeftList,
    RightList,
}

pub struct MainScreen<'clilife> {
    curr_path: PathBuf,
    s3_prefix: Option<String>,
    client: &'clilife Cli,
    term: Terminal<CrosstermBackend<Stdout>>,
    curr_list: CurrentList,
    s3_list: StatefulList,
    fs_list: StatefulList,
}

impl<'clilife> MainScreen<'clilife> {
    pub async fn new(
        term: Terminal<CrosstermBackend<Stdout>>,
        client: &'clilife Cli,
    ) -> MainScreen<'clilife> {
        let current_dir = env::current_dir().unwrap();
        MainScreen {
            s3_prefix: None,
            curr_path: current_dir.clone(),
            client: client,
            term: term,
            curr_list: CurrentList::LeftList,
            s3_list: Self::get_s3_list(&client, None).await,
            fs_list: Self::get_filesystem_list(current_dir.as_path()),
        }
    }

    pub async fn get_s3_list(client: &Cli, prefix: Option<String>) -> StatefulList {
        let items = client.list_objects(prefix).await;
        StatefulList::with_items(items.into_iter().map(move |i| i.name).collect())
    }

    pub fn get_filesystem_list(path: &Path) -> StatefulList {
        StatefulList::with_items(
            read_dir(path)
                .unwrap()
                .map(|f| {
                    let path = f.unwrap().path();
                    let mut file_name = String::from(path.file_name().unwrap().to_str().unwrap());
                    if !fs::metadata(&path).unwrap().is_file() {
                        file_name.push_str("/");
                    }
                    file_name
                })
                .collect(),
        )
    }

    pub async fn refresh_s3_list(&mut self) {
        self.s3_list = Self::get_s3_list(&self.client, self.s3_prefix.clone()).await
    }

    pub async fn refresh_fs_list(&mut self) {
        self.fs_list = Self::get_filesystem_list(self.curr_path.as_path());
    }

    pub fn extend_s3_prefix(&mut self) {
        let mut current = self.s3_prefix.clone().unwrap_or(String::new());
        match self.s3_list.get_state().selected() {
            None => (),
            Some(i) => {
                let selected = self.s3_list.get(i).value();
                if selected.chars().last().unwrap() == '/' {
                    current.push_str(selected);
                    self.s3_prefix = Some(current);
                }
            }
        };
    }

    pub fn shorten_s3_prefix(&mut self) {
        self.s3_prefix = match &self.s3_prefix {
            None => return,
            Some(prefix) => prefix
                .rmatch_indices('/')
                .nth(1)
                .map(|(i, _)| String::from(&prefix[..i + 1])),
        };
    }

    pub fn shorten_path(&mut self) {
        match &self.curr_path.as_path().parent() {
            Some(path) => self.curr_path = path.to_path_buf(),
            None => (),
        }
    }

    pub fn extend_path(&mut self) {
        let current = self.curr_path.to_str().unwrap();
        match self.fs_list.get_state().selected() {
            None => (),
            Some(i) => {
                let selected = self.fs_list.get(i);
                let path = &format!("{}/{}", current, selected.value());
                let path = Path::new(path);
                if fs::metadata(path).unwrap().is_dir() {
                    self.curr_path = path.to_path_buf();
                }
            }
        };
    }

    pub async fn handle_event(&mut self, event: KeyEvent) {
        let curr_list = self.get_curr_list();

        match event.code {
            KeyCode::Enter => {
                self.move_items().await;
                self.copy_items().await;
                self.delete_items().await;
                self.refresh_s3_list().await;
                self.refresh_fs_list().await;
            }
            KeyCode::Char(' ') => match self.curr_list {
                CurrentList::LeftList => {
                    self.extend_s3_prefix();
                    self.refresh_s3_list().await;
                }
                CurrentList::RightList => {
                    self.extend_path();
                    self.refresh_fs_list().await;
                }
            },
            KeyCode::Backspace => match self.curr_list {
                CurrentList::LeftList => {
                    self.shorten_s3_prefix();
                    self.refresh_s3_list().await;
                }
                CurrentList::RightList => {
                    self.shorten_path();
                    self.refresh_fs_list().await;
                }
            },
            KeyCode::Down | KeyCode::Char('j') => curr_list.next(),
            KeyCode::Up | KeyCode::Char('k') => curr_list.prev(),
            KeyCode::Left | KeyCode::Char('h') => self.curr_list = CurrentList::LeftList,
            KeyCode::Right | KeyCode::Char('l') => self.curr_list = CurrentList::RightList,
            KeyCode::Char('m') => curr_list.select(State::ToMove),
            KeyCode::Char('c') => curr_list.select(State::ToCopy),
            KeyCode::Char('d') => curr_list.select(State::ToDelete),
            KeyCode::Char('r') => self.refresh_s3_list().await,
            _ => (),
        }
    }

    fn get_s3_file_path(&self, file_name: &str) -> String {
        match &self.s3_prefix {
            Some(prefix) => format!("{}{}", prefix, file_name),
            None => String::from(file_name),
        }
    }

    fn get_fs_file_path(&self, file_name: &str) -> PathBuf {
        let path = format!("{}/{}", self.curr_path.to_str().unwrap(), file_name);
        Path::new(&path).to_path_buf()
    }

    async fn copy_items(&mut self) {
        let s3_selected: Vec<String> = self
            .s3_list
            .get_selected(State::ToCopy)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        for item in s3_selected {
            self.client
                .download_object(&self.get_s3_file_path(&item), &self.get_fs_file_path(&item))
                .await;
        }
        let fs_selected: Vec<String> = self
            .fs_list
            .get_selected(State::ToCopy)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        for item in fs_selected {
            self.client
                .put_object(&self.get_s3_file_path(&item), &self.get_fs_file_path(&item))
                .await;
        }
    }

    async fn move_items(&mut self) {
        let s3_selected: Vec<String> = self
            .s3_list
            .get_selected(State::ToMove)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        self.move_from_s3_to_filesystem(s3_selected).await;
        let fs_selected: Vec<String> = self
            .fs_list
            .get_selected(State::ToMove)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        self.move_from_filesystem_to_s3(fs_selected).await;
    }

    async fn move_from_s3_to_filesystem(&mut self, items: Vec<String>) {
        for item in items {
            self.client
                .download_object(&self.get_s3_file_path(&item), &self.get_fs_file_path(&item))
                .await;
            self.client
                .delete_object(&self.get_s3_file_path(&item))
                .await;
        }
    }

    async fn move_from_filesystem_to_s3(&mut self, items: Vec<String>) {
        for item in items {
            self.client
                .put_object(&self.get_s3_file_path(&item), &self.get_fs_file_path(&item))
                .await;
            fs::remove_file(&self.get_fs_file_path(&item)).unwrap();
        }
    }

    async fn delete_items(&mut self) {
        self.delete_s3_items().await;
        self.delete_filesystem_items();
    }

    async fn delete_s3_items(&mut self) {
        let selected: Vec<String> = self
            .s3_list
            .get_selected(State::ToDelete)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        for item in selected {
            self.client
                .delete_object(&self.get_s3_file_path(&item))
                .await;
        }
    }

    fn delete_filesystem_items(&mut self) {
        let selected: Vec<String> = self
            .fs_list
            .get_selected(State::ToDelete)
            .iter_mut()
            .map(|x| x.clone())
            .collect();
        for item in selected {
            fs::remove_file(&self.get_fs_file_path(&item)).unwrap();
        }
    }

    fn get_curr_list(&mut self) -> &mut StatefulList {
        match self.curr_list {
            CurrentList::LeftList => &mut self.s3_list,
            CurrentList::RightList => &mut self.fs_list,
        }
    }

    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(
            self.term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.term.show_cursor()?;
        self.term.clear()?;
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let s3_pane_name = format!(
                "{}/{}",
                self.client.bucket_name,
                self.s3_prefix.as_deref().unwrap_or("")
            );
            let list = make_file_list(
                &s3_pane_name,
                self.s3_list.get_items(),
                matches!(self.curr_list, CurrentList::LeftList),
            );
            f.render_stateful_widget(list, chunks[0], &mut self.s3_list.get_state());

            let list = make_file_list(
                self.curr_path.to_str().unwrap(),
                self.fs_list.get_items(),
                matches!(self.curr_list, CurrentList::RightList),
            );
            f.render_stateful_widget(list, chunks[1], &mut self.fs_list.get_state());
        })?;
        Ok(())
    }
}

fn make_file_list<'a>(name: &'a str, items: &'a [ListOption], is_focused: bool) -> List<'a> {
    let mut style = Style::default().fg(Color::White);
    if is_focused {
        style = style.fg(Color::LightBlue);
    }
    let block = Block::default()
        .title(name)
        .style(style)
        .borders(Borders::ALL);
    let items: Vec<ListItem> = transform_list(items);
    List::new(items)
        .block(block)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ")
}

fn transform_list(options: &[ListOption]) -> Vec<ListItem> {
    options
        .iter()
        .map(|o| {
            let text = o.value().as_str();
            let mut style = Style::default();

            if let Kind::Directory = o.kind() {
                style = style.add_modifier(Modifier::ITALIC);
            }
            match o.selected() {
                State::ToMove => style = style.bg(Color::LightBlue),
                State::ToDelete => style = style.bg(Color::Red),
                State::ToCopy => style = style.bg(Color::LightGreen),
                _ => (),
            }
            ListItem::new(text).style(style)
        })
        .collect()
}
