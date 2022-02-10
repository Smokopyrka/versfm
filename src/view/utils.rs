use tui::widgets::ListState;

#[derive(PartialEq)]
pub enum Selection {
    Normal,
    Move,
    Delete,
    Copy,
}
pub struct ListOption<T> 
where T: PartialEq {
    value: T,
    selected: Selection,
}

impl<T> ListOption<T> 
where T: PartialEq {
    fn new(value: T) -> ListOption<T> {
        ListOption { value: value, selected: Selection::Normal }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn selected(&self) -> &Selection {
        &self.selected
    }
    
    fn select(&mut self, new: Selection) {
        self.selected = match self.selected {
            Selection::Normal => new,
            _ => Selection::Normal
        }
    }
}

pub struct StatefulList<T>
where T: PartialEq {
    state: ListState,
    items: Vec<ListOption<T>>,
}

impl<T> StatefulList<T>
where T: PartialEq {

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut list_items: Vec<ListOption<T>> = Vec::with_capacity(items.len() as usize);
        for item in items {
            list_items.push(ListOption::new(item));
        }
        let mut state = ListState::default();
        if list_items.len() > 0 {
            state.select(Some(0));
        }
        StatefulList { state: state, items: list_items}
    }

    pub fn get(&self, i: usize) -> &ListOption<T> {
        &self.items[i]
    }

    pub fn get_items(&self) -> &[ListOption<T>] {
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
                    },
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
                    },
                    None => 0,
                };

            self.state.select(Some(i));
        }
    }

    pub fn select(&mut self, sel_type: Selection) {
        match self.state.selected() {
            None => (),
            Some(i) => self.items[i].select(sel_type),
        };
    }

    pub fn get_selected(&self, sel_type: Selection) -> Vec<&T> {
        self.items.iter()
            .filter(|i| *i.selected() == sel_type)
            .map(|i| i.value())
            .collect()
    }

    pub fn add(&mut self, item: T) {
        self.items.push(ListOption::new(item));
        self.reset_cursor();
    }

    pub fn remove(&mut self, item: &T) {
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