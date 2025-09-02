// Copyright (C) 2025  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

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
