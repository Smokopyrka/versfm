use std::{io::Stdout, error::Error, path::Path};
use crossterm::{
    terminal::{
        disable_raw_mode,
        LeaveAlternateScreen},
        execute,
        event::{DisableMouseCapture, KeyEvent, KeyCode}};
use tui::{
    backend::{CrosstermBackend},
    Terminal,
    widgets::{Borders, Block, ListItem, List},
    layout::{Layout, Constraint, Direction}, style::{Style, Color, Modifier}};

use crate::view::utils::{StatefulList, ListOption, Selection};
use crate::aws::s3::Cli;

const BUCKET_NAME: &str = "test-bucket";

enum CurrentList {
    LeftList,
    RightList,
}

pub struct MainScreen<'clilife> {
    client: &'clilife Cli,
    term: Terminal<CrosstermBackend<Stdout>>,
    curr_list: CurrentList,
    l_list: StatefulList<String>,
    r_list: StatefulList<String>,
}

impl<'clilife> MainScreen<'clilife> {

    pub fn new(term:Terminal<CrosstermBackend<Stdout>>, client: &'clilife Cli) -> MainScreen {

        let l_items = vec![
        String::from("Item 1"),
        String::from("Item 2"),
        String::from("Item 3"),
        String::from("Item 4")];
        
        let l_list = StatefulList::with_items(l_items);
        
        let r_items = vec![
        String::from("Item 5"),
        String::from("Item 6"),
        String::from("Item 7"),
        String::from("Item 8")];

        let r_list = StatefulList::with_items(r_items);
        
        MainScreen {
            client: client,
            term: term,
            curr_list: CurrentList::LeftList,
            l_list: l_list,
            r_list: r_list,
        }
    }

    pub async fn populate_s3_list(&mut self, bucket_name: &str) {
        let items = self.client.list_objects(bucket_name).await;
        self.l_list = StatefulList::with_items(
            items.into_iter()
            .map(move |i| i.name)
            .collect());
    }

    pub async fn handle_event(&mut self, event: KeyEvent) {
        let curr_list = self.get_curr_list();

        match event.code {
            KeyCode::Enter => {
                self.move_items();
                self.copy_items().await;
                self.delete_items();
            },
            KeyCode::Down | KeyCode::Char('j') => {
                curr_list.next();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                curr_list.prev();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.curr_list = CurrentList::LeftList;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.curr_list = CurrentList::RightList;
            }
            KeyCode::Char('m') => {
                curr_list.select(Selection::Move);
            }
            KeyCode::Char('c') => {
                curr_list.select(Selection::Copy);
            }
            KeyCode::Char('d') => {
                curr_list.select(Selection::Delete);
            }
            KeyCode::Char('r') => {
                self.populate_s3_list(BUCKET_NAME).await;
            }
            _ => ()
        }
    }

    async fn copy_items(&mut self) {
        let l_selected: Vec<String> = self.l_list.get_selected(Selection::Copy).iter_mut()
            .map(|x| x.clone())
            .collect();
        for item in l_selected {
            self.client.download_object(BUCKET_NAME, item.as_str(), Path::new(item.as_str())).await;
            self.r_list.add(item);
        }
    }

    fn move_items(&mut self) {
        let l_selected: Vec<String> = self.l_list.get_selected(Selection::Move).iter_mut()
            .map(|x| x.clone())
            .collect();
        MainScreen::move_items_between_lists(l_selected,&mut self.l_list, &mut self.r_list);
        let r_selected: Vec<String> = self.r_list.get_selected(Selection::Move).iter_mut()
            .map(|x| x.clone())
            .collect();
        MainScreen::move_items_between_lists(r_selected, &mut self.r_list, &mut self.l_list);
    }

    fn move_items_between_lists(items: Vec<String>, from: &mut StatefulList<String>, to: &mut StatefulList<String>) {
        for item in items {
            from.remove(&item);
            to.add(item);
        }
    }

    fn delete_items(&mut self) {
        MainScreen::delete_items_from_list(&mut self.l_list);
        MainScreen::delete_items_from_list(&mut self.r_list);
    }

    fn delete_items_from_list(list: &mut StatefulList<String>) {
        let selected: Vec<String> = list.get_selected(Selection::Delete).iter_mut()
            .map(|x| x.clone())
            .collect();
        
        for item in selected {
            list.remove(&item);
        }
    }

    fn get_curr_list(&mut self) -> &mut StatefulList<String> {
        match self.curr_list {
            CurrentList::LeftList => &mut self.l_list,
            CurrentList::RightList => &mut self.r_list,
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
                    .constraints(
                        [
                            Constraint::Percentage(50),
                            Constraint::Percentage(50),
                        ].as_ref()
                    )
                    .split(f.size());

                let list = make_file_list("Left", self.l_list.get_items());
                f.render_stateful_widget(list, chunks[0], &mut self.l_list.get_state());

                let list = make_file_list("Right", self.r_list.get_items());
                f.render_stateful_widget(list, chunks[1], &mut self.r_list.get_state());
        })?;
        Ok(())
    }

}

fn make_file_list<'a>(name: &'a str, items: &'a [ListOption<String>]) -> List<'a> {
    let block = Block::default()
        .title(name)
        .borders(Borders::ALL);
    let items: Vec<ListItem> = transform_list(items);
    List::new(items)
        .block(block)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ")
}

fn transform_list(options: &[ListOption<String>]) -> Vec<ListItem> {
    options.iter().map(|o| {
        let text = o.value().as_str();
        let mut style = Style::default();

        match o.selected() {
            Selection::Move => style = style.bg(Color::LightBlue),
            Selection::Delete => style = style.bg(Color::Red),
            Selection::Copy => style = style.bg(Color::LightGreen),
            _ => ()
        }
        ListItem::new(text).style(style)
    }).collect()
}

