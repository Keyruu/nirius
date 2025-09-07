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
    collections::{HashMap, VecDeque},
    sync::{LazyLock, Mutex},
};

use niri_ipc::Window;

pub struct State {
    pub all_windows: VecDeque<Window>,
    pub already_focused_win_ids: Vec<u64>,
    pub follow_mode_win_ids: Vec<u64>,
    pub mark_to_win_ids: HashMap<String, Vec<u64>>,
}

impl State {
    pub fn activate_window(&mut self, win: Window) -> Result<String, String> {
        let mut new_win = true;
        if let Some(idx) = self.all_windows.iter().position(|w| w.id == win.id)
        {
            self.all_windows.remove(idx);
            new_win = false;
        }
        let msg = if new_win {
            format!(
                "Registered window {}. Currently managing {} windows.",
                &win.id,
                self.all_windows.len() + 1
            )
        } else {
            format!("Activated window {}.", &win.id)
        };
        self.all_windows.push_back(win);
        Ok(msg)
    }

    pub fn remove_window(&mut self, id: &u64) -> Result<String, String> {
        self.all_windows.retain(|w| w.id != *id);
        self.already_focused_win_ids.retain(|i| i != id);
        self.follow_mode_win_ids.retain(|i| i != id);
        for v in self.mark_to_win_ids.values_mut() {
            v.retain(|i| i != id);
        }
        Ok(format!(
            "Removed window with id {id}. Currently managing {} windows.",
            self.all_windows.len()
        ))
    }

    pub fn window_focus_changed(
        &mut self,
        opt_id: Option<u64>,
    ) -> Result<String, String> {
        for win in self.all_windows.iter_mut() {
            win.is_focused = opt_id.is_some_and(|id| win.id == id)
        }
        Ok("Updated focus.".to_string())
    }
}

pub static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        all_windows: VecDeque::new(),
        follow_mode_win_ids: vec![],
        already_focused_win_ids: vec![],
        mark_to_win_ids: HashMap::new(),
    })
});
