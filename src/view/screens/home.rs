use crossterm::{
    event::{DisableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::{error::Error, io::Stdout, sync::Arc};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

use crate::providers::s3::S3Provider;
use crate::view::components::{FileCRUD, FileList, FilesystemList, S3List, State};

enum CurrentList {
    LeftList,
    RightList,
}

pub struct MainScreen {
    term: Terminal<CrosstermBackend<Stdout>>,
    curr_list: CurrentList,
    s3_list: Box<dyn FileList>,
    fs_list: Box<dyn FileList>,
}

impl MainScreen {
    pub async fn new(
        term: Terminal<CrosstermBackend<Stdout>>,
        client: Arc<S3Provider>,
    ) -> MainScreen {
        let mut s3_list = Box::new(S3List::new(client));
        s3_list.refresh().await;
        let fs_list = Box::new(FilesystemList::new());
        MainScreen {
            term: term,
            curr_list: CurrentList::LeftList,
            s3_list,
            fs_list,
        }
    }

    pub async fn handle_event(&mut self, event: KeyEvent) {
        let curr_list = self.get_curr_list();

        match event.code {
            KeyCode::Enter => {
                self.move_items().await;
                self.copy_items().await;
                self.delete_items().await;
                self.s3_list.refresh().await;
                self.fs_list.refresh().await;
            }
            KeyCode::Char(' ') => {
                curr_list.move_into_selected_dir();
                curr_list.refresh().await;
            }
            KeyCode::Backspace => {
                curr_list.move_out_of_selected_dir();
                curr_list.refresh().await;
            }
            KeyCode::Down | KeyCode::Char('j') => curr_list.next(),
            KeyCode::Up | KeyCode::Char('k') => curr_list.previous(),
            KeyCode::Left | KeyCode::Char('h') => self.curr_list = CurrentList::LeftList,
            KeyCode::Right | KeyCode::Char('l') => self.curr_list = CurrentList::RightList,
            KeyCode::Char('m') => curr_list.select(State::ToMove),
            KeyCode::Char('c') => curr_list.select(State::ToCopy),
            KeyCode::Char('d') => curr_list.select(State::ToDelete),
            KeyCode::Char('r') => curr_list.refresh().await,
            _ => (),
        }
    }

    async fn copy_items(&mut self) {
        Self::copy_from_to(&mut self.fs_list, &mut self.s3_list).await;
        Self::copy_from_to(&mut self.s3_list, &mut self.fs_list).await;
    }

    async fn copy_from_to(from: &mut Box<dyn FileList>, to: &mut Box<dyn FileList>) {
        for selected in from.get_selected(State::ToCopy) {
            let name = selected.get_name();
            to.put_file(name, from.get_file_stream(name).await).await;
        }
    }

    async fn delete_items(&mut self) {
        Self::delete_from(&mut self.s3_list).await;
        Self::delete_from(&mut self.fs_list).await;
    }

    async fn delete_from(from: &mut Box<dyn FileList>) {
        for selected in from.get_selected(State::ToDelete) {
            let name = selected.get_name();
            from.delete_file(name).await;
        }
    }

    async fn move_items(&mut self) {
        Self::move_from_to(&mut self.fs_list, &mut self.s3_list).await;
        Self::move_from_to(&mut self.s3_list, &mut self.fs_list).await;
    }

    async fn move_from_to(from: &mut Box<dyn FileList>, to: &mut Box<dyn FileList>) {
        for selected in from.get_selected(State::ToMove) {
            let name = selected.get_name();
            to.put_file(name, from.get_file_stream(name).await).await;
            from.delete_file(name).await;
        }
    }

    fn get_curr_list(&mut self) -> &mut Box<dyn FileList> {
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

            f.render_stateful_widget(
                self.s3_list
                    .make_file_list(matches!(self.curr_list, CurrentList::LeftList)),
                chunks[0],
                &mut self.s3_list.get_current(),
            );

            f.render_stateful_widget(
                self.fs_list
                    .make_file_list(matches!(self.curr_list, CurrentList::LeftList)),
                chunks[1],
                &mut self.fs_list.get_current(),
            );
        })?;
        Ok(())
    }
}
