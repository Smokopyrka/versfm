use crossterm::{
    event::{DisableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::Stdout,
    sync::{Arc, Mutex, MutexGuard},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{List, ListItem},
    Terminal,
};

use crate::{
    utils::append_path_to_dir,
    view::components::{err::ComponentError, FileCRUDListWidget, State},
};

/// Takes a list of ComponentErrors and creates a Vector of ListItems
/// from it
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

/// Enum representing which list is currently under focus
enum CurrentList {
    LeftList,
    RightList,
}

/// A view consisting of two lists of file entries that can be
/// moved, copied, deleted between one another
pub struct DualPaneList {
    term: Terminal<CrosstermBackend<Stdout>>,
    curr_list: CurrentList,
    left_pane: Arc<Box<dyn FileCRUDListWidget>>,
    right_pane: Arc<Box<dyn FileCRUDListWidget>>,
    err_stack: Arc<Mutex<Vec<ComponentError>>>,
}

impl DualPaneList {
    pub async fn new(
        term: Terminal<CrosstermBackend<Stdout>>,
        left_pane: Box<dyn FileCRUDListWidget>,
        right_pane: Box<dyn FileCRUDListWidget>,
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
            left_pane: Arc::new(left_pane),
            right_pane: Arc::new(right_pane),
            err_stack: Arc::new(Mutex::new(err_stack)),
        }
    }

    fn lock_err_stack(&self) -> MutexGuard<Vec<ComponentError>> {
        self.err_stack
            .lock()
            .expect("Couldn't lock err_stack mutex")
    }

    /// Handles given `ComponentError` by pushing it onto the error stack
    fn handle_err(&self, e: ComponentError) {
        self.lock_err_stack().push(e);
    }

    /// Return `true` if the error stack is empty
    fn err_stack_empty(&self) -> bool {
        self.lock_err_stack().is_empty()
    }

    /// Clears the error stack
    fn err_stack_clear(&self) {
        self.lock_err_stack().clear()
    }

    /// Handles the event sent to the applications by the input thread
    pub async fn handle_event(&mut self, event: KeyEvent) {
        let curr_list = self.get_curr_list();

        match event.code {
            KeyCode::Enter => {
                if self.err_stack_empty() {
                    self.move_items();
                    self.copy_items();
                    self.delete_items();
                } else {
                    self.err_stack_clear();
                }
            }
            KeyCode::Char(' ') => {
                curr_list.move_into_selected_dir();
                if let Err(e) = curr_list.refresh().await {
                    curr_list.move_out_of_selected_dir();
                    self.handle_err(e);
                }
            }
            KeyCode::Backspace => {
                curr_list.move_out_of_selected_dir();
                if let Err(e) = curr_list.refresh().await {
                    curr_list.move_into_selected_dir();
                    self.handle_err(e);
                }
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

    /// Refreshes both of the lists
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

    /// Copies items between the lists
    fn copy_items(&self) {
        self.copy_from_to(self.right_pane.clone(), self.left_pane.clone());
        self.copy_from_to(self.left_pane.clone(), self.right_pane.clone());
    }

    fn copy_from_to(
        &self,
        from: Arc<Box<dyn FileCRUDListWidget>>,
        to: Arc<Box<dyn FileCRUDListWidget>>,
    ) {
        for selected in from.get_selected(State::ToCopy) {
            self.spawn_copy_task(from.clone(), to.clone(), selected.to_owned());
        }
    }

    /// Sprawns a copy task for given file
    fn spawn_copy_task(
        &self,
        from: Arc<Box<dyn FileCRUDListWidget>>,
        to: Arc<Box<dyn FileCRUDListWidget>>,
        file_name: String,
    ) {
        let err_stack = self.err_stack.clone();

        let from_path = append_path_to_dir(&from.get_current_path(), &file_name);
        let to_path = append_path_to_dir(&to.get_current_path(), &file_name);
        tokio::spawn(async move {
            match from.get_file_stream(&from_path).await {
                Err(e) => err_stack
                    .lock()
                    .expect("Couldn't lock err_stack mutex")
                    .push(e),
                Ok(file) => {
                    from.start_processing_item(&file_name);
                    to.put_file(&to_path, file).await.unwrap_or_else(|e| {
                        err_stack
                            .lock()
                            .expect("Couldn't lock err_stack mutex")
                            .push(e)
                    });
                    from.stop_processing_item(&file_name);
                }
            }
        });
    }

    /// Deletes selected items from both lists
    fn delete_items(&self) {
        self.delete_from(self.left_pane.clone());
        self.delete_from(self.right_pane.clone());
    }

    fn delete_from(&self, from: Arc<Box<dyn FileCRUDListWidget>>) {
        for selected in from.get_selected(State::ToDelete) {
            self.spawn_delete_task(from.clone(), selected.to_owned());
        }
    }

    /// Spawns a delete task for given file
    fn spawn_delete_task(&self, from: Arc<Box<dyn FileCRUDListWidget>>, file_name: String) {
        let err_stack = self.err_stack.clone();

        let from_path = append_path_to_dir(&from.get_current_path(), &file_name);
        tokio::spawn(async move {
            from.start_processing_item(&file_name);
            from.delete_file(&from_path).await.unwrap_or_else(|e| {
                err_stack
                    .lock()
                    .expect("Couldn't lock err_stack mutex")
                    .push(e)
            });
        });
    }

    /// Moves items between lists
    fn move_items(&self) {
        self.move_from_to(self.right_pane.clone(), self.left_pane.clone());
        self.move_from_to(self.left_pane.clone(), self.right_pane.clone());
    }

    fn move_from_to(
        &self,
        from: Arc<Box<dyn FileCRUDListWidget>>,
        to: Arc<Box<dyn FileCRUDListWidget>>,
    ) {
        for selected in from.get_selected(State::ToMove) {
            let from = from.clone();
            let to = to.clone();
            self.spawn_move_task(from, to, selected.to_owned());
        }
    }

    /// Spawn move task for the given file
    fn spawn_move_task(
        &self,
        from: Arc<Box<dyn FileCRUDListWidget>>,
        to: Arc<Box<dyn FileCRUDListWidget>>,
        file_name: String,
    ) {
        let err_stack = self.err_stack.clone();

        let from_path = append_path_to_dir(&from.get_current_path(), &file_name);
        let to_path = append_path_to_dir(&to.get_current_path(), &file_name);

        tokio::spawn(async move {
            match from.get_file_stream(&from_path).await {
                Ok(file) => {
                    from.start_processing_item(&file_name);
                    to.put_file(&to_path, file).await.unwrap_or_else(|e| {
                        err_stack
                            .lock()
                            .expect("Couldn't lock err_stack mutex")
                            .push(e)
                    });
                    from.delete_file(&from_path).await.unwrap_or_else(|e| {
                        err_stack
                            .lock()
                            .expect("Couldn't lock err_stack mutex")
                            .push(e)
                    });
                }
                Err(e) => err_stack.lock().unwrap().push(e),
            }
        });
    }

    /// Returns the list that is currently selected
    fn get_curr_list(&mut self) -> Arc<Box<dyn FileCRUDListWidget>> {
        match self.curr_list {
            CurrentList::LeftList => self.left_pane.clone(),
            CurrentList::RightList => self.right_pane.clone(),
        }
    }

    /// Shuts down this screen, releases the terminal etc.
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

    /// Renders this screen
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
                        .make_list(matches!(self.curr_list, CurrentList::LeftList)),
                    chunks[0],
                    &mut self.left_pane.get_current(),
                );
                f.render_stateful_widget(
                    self.right_pane
                        .make_list(matches!(self.curr_list, CurrentList::RightList)),
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
