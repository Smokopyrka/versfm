use crossterm::{
    event::{DisableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::Stdout,
    sync::{Arc, Mutex},
};
use tokio::join;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{List, ListItem},
    Terminal,
};

use crate::view::components::{err::ComponentError, FileList, State};

async fn move_from_to<'a>(
    from: &Box<dyn FileList>,
    to: &Box<dyn FileList>,
) -> Result<(), ComponentError> {
    for selected in from.get_selected(State::ToMove) {
        to.put_file(&selected, from.get_file_stream(&selected).await?)
            .await?;
        from.delete_file(&selected).await?;
    }
    Ok(())
}

async fn copy_from_to(
    from: &Box<dyn FileList>,
    to: &Box<dyn FileList>,
) -> Result<(), ComponentError> {
    for selected in from.get_selected(State::ToCopy) {
        to.put_file(&selected, from.get_file_stream(&selected).await?)
            .await?;
    }
    Ok(())
}

async fn delete_from(from: &Box<dyn FileList>) -> Result<(), ComponentError> {
    for selected in from.get_selected(State::ToDelete) {
        from.delete_file(&selected).await?;
    }
    Ok(())
}

fn get_err_list<'err_stack_lif>(
    errs: Arc<Mutex<Vec<ComponentError>>>,
) -> Vec<ListItem<'err_stack_lif>> {
    errs.lock()
        .expect("Couldn't lock mutex")
        .iter()
        .map(|e| {
            ListItem::new(format!(
                "{} Err: {} - {}",
                e.component(),
                e.code(),
                e.message()
            ))
        })
        .collect()
}

enum CurrentList {
    LeftList,
    RightList,
}

pub struct DualPaneList {
    term: Terminal<CrosstermBackend<Stdout>>,
    curr_list: CurrentList,
    left_pane: Box<dyn FileList>,
    right_pane: Box<dyn FileList>,
    err_stack: Arc<Mutex<Vec<ComponentError>>>,
}

impl DualPaneList {
    pub async fn new(
        term: Terminal<CrosstermBackend<Stdout>>,
        left_pane: Box<dyn FileList>,
        right_pane: Box<dyn FileList>,
    ) -> DualPaneList {
        let mut err_stack: Vec<ComponentError> = Vec::new();
        left_pane
            .refresh()
            .await
            .unwrap_or_else(|e| err_stack.push(e));
        right_pane
            .refresh()
            .await
            .unwrap_or_else(|e| err_stack.push(e));
        DualPaneList {
            term,
            curr_list: CurrentList::LeftList,
            left_pane,
            right_pane,
            err_stack: Arc::new(Mutex::new(err_stack)),
        }
    }

    fn handle_err(&self, e: ComponentError) {
        self.err_stack.lock().expect("Couldn't lock mutex").push(e);
    }

    fn err_stack_empty(&self) -> bool {
        self.err_stack
            .lock()
            .expect("Couldn't lock mutex")
            .is_empty()
    }

    fn err_stack_clear(&self) {
        self.err_stack.lock().expect("Couldn't lock mutex").clear()
    }

    pub async fn handle_event(&mut self, event: KeyEvent) {
        let curr_list = self.get_curr_list();

        match event.code {
            KeyCode::Enter => {
                if self.err_stack_empty() {
                    let move_ft = self.move_items();
                    let copy_ft = self.copy_items();
                    let delete_ft = self.delete_items();
                    join!(move_ft, copy_ft, delete_ft);
                    self.refresh_lists().await;
                } else {
                    self.err_stack_clear();
                }
            }
            KeyCode::Char(' ') => {
                curr_list
                    .move_into_selected_dir()
                    .await
                    .unwrap_or_else(|e| self.handle_err(e));
            }
            KeyCode::Backspace => {
                curr_list
                    .move_out_of_selected_dir()
                    .await
                    .unwrap_or_else(|e| self.handle_err(e));
            }
            KeyCode::Down | KeyCode::Char('j') => curr_list.next(),
            KeyCode::Up | KeyCode::Char('k') => curr_list.previous(),
            KeyCode::Left | KeyCode::Char('h') => self.curr_list = CurrentList::LeftList,
            KeyCode::Right | KeyCode::Char('l') => self.curr_list = CurrentList::RightList,
            KeyCode::Char('m') => curr_list.select(State::ToMove),
            KeyCode::Char('c') => curr_list.select(State::ToCopy),
            KeyCode::Char('d') => curr_list.select(State::ToDelete),
            KeyCode::Char('r') => self.refresh_lists().await,
            _ => (),
        }
    }

    async fn refresh_lists(&mut self) {
        self.left_pane
            .refresh()
            .await
            .unwrap_or_else(|e| self.handle_err(e));
        self.right_pane
            .refresh()
            .await
            .unwrap_or_else(|e| self.handle_err(e));
    }

    async fn copy_items(&self) {
        copy_from_to(&self.right_pane, &self.left_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
        copy_from_to(&self.left_pane, &self.right_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
    }

    async fn delete_items(&self) {
        delete_from(&self.left_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
        delete_from(&self.right_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
    }

    async fn move_items(&self) {
        move_from_to(&self.right_pane, &self.left_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
        move_from_to(&self.left_pane, &self.right_pane)
            .await
            .unwrap_or_else(|e| self.handle_err(e));
    }

    fn get_curr_list(&mut self) -> &mut Box<dyn FileList> {
        match self.curr_list {
            CurrentList::LeftList => &mut self.left_pane,
            CurrentList::RightList => &mut self.right_pane,
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
        let term_size = self.term.size().unwrap();
        if self.err_stack_empty() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(term_size);

            self.term.draw(|f| {
                f.render_stateful_widget(
                    self.left_pane
                        .make_file_list(matches!(self.curr_list, CurrentList::LeftList)),
                    chunks[0],
                    &mut self.left_pane.get_current(),
                );
                f.render_stateful_widget(
                    self.right_pane
                        .make_file_list(matches!(self.curr_list, CurrentList::RightList)),
                    chunks[1],
                    &mut self.right_pane.get_current(),
                );
            })?;
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(100)])
                .split(term_size);

            let mut err_list = get_err_list(self.err_stack.clone());
            err_list.push(ListItem::new("Press ENTER to continue"));
            let err_list = List::new(err_list);
            self.term.draw(|f| {
                f.render_widget(err_list, chunks[0]);
            })?;
        }
        Ok(())
    }
}
