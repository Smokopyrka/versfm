use tui::widgets::ListState;

#[derive(PartialEq)]
pub enum State {
    Unselected,
    ToMove,
    ToDelete,
    ToCopy,
}

pub enum Kind {
    File,
    Directory,
}
pub struct ListOption {
    value: String,
    state: State,
    kind: Kind,
}

impl ListOption {
    fn new(value: String, kind: Kind) -> ListOption {
        ListOption {
            value,
            state: State::Unselected,
            kind,
        }
    }

    pub fn value(&self) -> &String {
        &self.value
    }

    pub fn selected(&self) -> &State {
        &self.state
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    fn select(&mut self, new: State) {
        self.state = match self.state {
            State::Unselected => new,
            _ => State::Unselected,
        }
    }
}

pub struct StatefulList {
    state: ListState,
    items: Vec<ListOption>,
}

impl StatefulList {
    pub fn with_items(items: Vec<String>) -> StatefulList {
        let mut list_items: Vec<ListOption> = Vec::with_capacity(items.len() as usize);
        for item in items {
            if item.chars().last().unwrap() == '/' {
                list_items.push(ListOption::new(item, Kind::Directory));
            } else {
                list_items.push(ListOption::new(item, Kind::File));
            }
        }
        let mut state = ListState::default();
        if list_items.len() > 0 {
            state.select(Some(0));
        }
        StatefulList {
            state: state,
            items: list_items,
        }
    }

    pub fn get(&self, i: usize) -> &ListOption {
        &self.items[i]
    }

    pub fn get_items(&self) -> &[ListOption] {
        self.items.as_slice()
    }

    pub fn get_state(&self) -> ListState {
        self.state.clone()
    }

    fn reset_cursor(&mut self) {
        if self.items.len() > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    pub fn next(&mut self) {
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

    pub fn prev(&mut self) {
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

    pub fn select(&mut self, sel_type: State) {
        match self.state.selected() {
            None => (),
            Some(i) => {
                let obj = self.items.get_mut(i).unwrap();
                match obj.kind {
                    Kind::File => obj.select(sel_type),
                    Kind::Directory => (),
                }
            }
        };
    }

    pub fn get_selected(&self, sel_type: State) -> Vec<&String> {
        self.items
            .iter()
            .filter(|i| *i.selected() == sel_type)
            .map(|i| i.value())
            .collect()
    }

    pub fn add(&mut self, item: String, kind: Kind) {
        self.items.push(ListOption::new(item, kind));
        self.reset_cursor();
    }

    pub fn remove(&mut self, item: &String) {
        let mut to_remove: Option<usize> = None;
        for (i, val) in self.items.iter().enumerate() {
            if val.value() == item {
                to_remove = Some(i);
            }
        }
        if let Some(i) = to_remove {
            self.items.remove(i);
        }
        self.reset_cursor();
    }
}
