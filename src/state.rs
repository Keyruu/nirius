use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

pub struct State {
    pub already_focused_win_ids: Vec<u64>,
    pub follow_mode_win_ids: Vec<u64>,
    pub mark_to_win_ids: HashMap<String, Vec<u64>>,
}

impl State {
    pub fn remove_window(&mut self, id: &u64) {
        self.already_focused_win_ids.retain(|i| i != id);
        self.follow_mode_win_ids.retain(|i| i != id);
        for v in self.mark_to_win_ids.values_mut() {
            v.retain(|i| i != id);
        }
    }
}

pub static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        follow_mode_win_ids: vec![],
        already_focused_win_ids: vec![],
        mark_to_win_ids: HashMap::new(),
    })
});
