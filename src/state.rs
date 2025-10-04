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
    sync::{LazyLock, RwLock},
};

use niri_ipc::{Window, Workspace};

pub struct State {
    pub all_windows: VecDeque<Window>,
    pub all_workspaces: Vec<Workspace>,
    pub follow_mode_win_ids: Vec<u64>,
    pub scratchpad_win_ids: Vec<u64>,
    pub mark_to_win_ids: HashMap<String, Vec<u64>>,
}

impl State {
    pub fn get_focused_win_id(&self) -> Option<u64> {
        self.all_windows.iter().find(|w| w.is_focused).map(|w| w.id)
    }

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
        self.follow_mode_win_ids.retain(|i| i != id);
        self.scratchpad_win_ids.retain(|i| i != id);
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
        if let Some(idx) = self.all_windows.iter().position(|w| w.is_focused) {
            if let Some(win) = self.all_windows.remove(idx) {
                self.all_windows.push_back(win);
                Ok("Updated focus.".to_string())
            } else {
                Err(format!("Could not remove window at index {idx}."))
            }
        } else {
            Ok("Updated focus (no window is focused).".to_string())
        }
    }

    pub fn get_focused_workspace_id(&self) -> Option<u64> {
        self.all_workspaces
            .iter()
            .find(|ws| ws.is_focused)
            .map(|ws| ws.id)
    }

    pub fn workspaces_changed(
        &mut self,
        workspaces: Vec<Workspace>,
    ) -> Result<String, String> {
        self.all_workspaces = workspaces;
        Ok("Updated all workspaces.".to_owned())
    }

    pub fn workspace_focused(&mut self, id: u64) {
        for ws in &mut self.all_workspaces {
            ws.is_focused = ws.id == id;
        }
    }

    pub fn is_bottom_workspace(&self, ws_id: u64) -> bool {
        if let Some(ws) = self.all_workspaces.iter().find(|ws| ws.id == ws_id) {
            // It's the bottom workspace if the max index of all workspaces on the
            // same output is this workspace's index + 1 because there is always
            // one empty workspace at the bottom.
            ws.idx + 1
                == self
                    .all_workspaces
                    .iter()
                    .filter(|ws2| ws2.output == ws.output)
                    .map(|ws2| ws2.idx)
                    .max()
                    .unwrap_or(ws.idx + 1)
        } else {
            // Well, this should not happen, but better move one time too often...
            true
        }
    }
}

pub static STATE: LazyLock<RwLock<State>> = LazyLock::new(|| {
    RwLock::new(State {
        all_windows: VecDeque::new(),
        all_workspaces: Vec::new(),
        follow_mode_win_ids: vec![],
        scratchpad_win_ids: vec![],
        mark_to_win_ids: HashMap::new(),
    })
});
