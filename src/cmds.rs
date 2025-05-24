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

use std::{cmp::Ordering, sync::Mutex};

use crate::ipc;
use niri_ipc::{Action, Request, Response, Window};
use regex::Regex;
use serde::{Deserialize, Serialize};

static NO_MATCHING_WINDOW: &str = "No matching window.";

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub enum NiriusCmd {
    /// Focus the window matching the given options.  If there is more than one
    /// matching window, cycle through them.  If there is none, exit non-zero.
    Focus {
        #[clap(flatten)]
        match_opts: MatchOptions,
    },
    /// Focus the window matching the given options.  If there is more than one
    /// matching window, cycle through them.  If there is none, spawn the given
    /// COMMAND instead.
    FocusOrSpawn {
        #[clap(flatten)]
        match_opts: MatchOptions,
        command: Vec<String>,
    },
    /// Does nothing except having the side-effect of clearing the list of
    /// windows that were already visited by a sequence of `focus` or
    /// `focus-or-spawn` commands.
    Nop,
    /// Enables or disables follow-mode for the currently focused window.  A
    /// window in follow-mode moves automatically to whatever workspace that
    /// receives focus.
    ToggleFollowMode,
    /// Marks or unmarks the currently focused window.  You can switch to the
    /// marked window or cycle trough all marked windows using the
    /// `focus-marked` command.
    ToggleMark,
    /// Focuses the marked window or cycles through all marked windows.  To
    /// mark a window, use the `toggle-mark` command.
    FocusMarked,
    /// List all windows which are currenty marked on stdout.
    ListMarked,
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct MatchOptions {
    #[clap(short = 'a', long, help = "A regex  matched on window app-ids")]
    app_id: Option<String>,

    #[clap(short = 't', long, help = "A regex matched on window titles")]
    title: Option<String>,
}

static ALREADY_FOCUSED_WIN_IDS: Mutex<Vec<u64>> = Mutex::new(vec![]);
static MARKED_WIN_IDS: Mutex<Vec<u64>> = Mutex::new(vec![]);
static LAST_COMMAND: Mutex<Option<NiriusCmd>> = Mutex::new(None);

pub fn exec_nirius_cmd(cmd: NiriusCmd) -> Result<String, String> {
    let mut last_command = LAST_COMMAND.lock().expect("Could not lock mutex.");
    let clear_focused_win_ids =
        last_command.as_ref().is_some_and(|lc| lc != &cmd);

    let result = match &cmd {
        NiriusCmd::Nop => Ok("Nothing done".to_string()),
        NiriusCmd::Focus { match_opts } => focus(match_opts),
        NiriusCmd::FocusOrSpawn {
            match_opts,
            command,
        } => focus_or_spawn(match_opts, command),
        NiriusCmd::ToggleFollowMode => toggle_follow_mode(),
        NiriusCmd::ToggleMark => toggle_mark(),
        NiriusCmd::FocusMarked => focus_marked(),
        NiriusCmd::ListMarked => list_marked(),
    };

    if clear_focused_win_ids {
        ALREADY_FOCUSED_WIN_IDS
            .lock()
            .expect("Could not lock mutex")
            .clear()
    }

    *last_command = Some(cmd.clone());

    result
}

fn get_focused_window() -> Result<niri_ipc::Window, String> {
    match ipc::query_niri(Request::FocusedWindow)? {
        Response::FocusedWindow(window) => {
            window.ok_or("No focused window".to_owned())
        }
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}

fn toggle_follow_mode() -> Result<String, String> {
    let focused_win = get_focused_window()?;
    match crate::daemon::FOLLOW_MODE_WIN_IDS.lock() {
        Ok(mut ids) => {
            if ids.contains(&focused_win.id) {
                if let Some(index) =
                    ids.iter().position(|id| *id == focused_win.id)
                {
                    // swap_remove() would be more efficient but I think we
                    // want to retain the order.
                    ids.remove(index);
                }
                Ok(format!("Disabled follow mode for window {:?}", focused_win))
            } else {
                ids.push(focused_win.id);
                Ok(format!("Enabled follow mode for window {:?}", focused_win))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

fn focus_or_spawn(
    match_opts: &MatchOptions,
    command: &[String],
) -> Result<String, String> {
    match focus(match_opts) {
        Err(str) if NO_MATCHING_WINDOW == str => {
            let r = ipc::query_niri(Request::Action(Action::Spawn {
                command: command.to_vec(),
            }))?;
            match r {
                Response::Handled => Ok("Spawned successfully".to_string()),
                x => Err(format!("Received unexpected reply {:?}", x)),
            }
        }
        x => x,
    }
}

fn focus(match_opts: &MatchOptions) -> Result<String, String> {
    match ipc::query_niri(Request::Windows)? {
        Response::Windows(mut wins) => {
            let mut ids = ALREADY_FOCUSED_WIN_IDS
                .lock()
                .expect("Could not lock mutex");
            wins.retain(|w| window_matches(w, match_opts));
            if wins.iter().all(|w| ids.contains(&w.id)) {
                ids.clear();
            }
            wins.sort_by(|a, b| {
                if a.is_focused {
                    return Ordering::Greater;
                }
                if b.is_focused {
                    return Ordering::Less;
                }

                let a_visited = ids.contains(&a.id);
                let b_visited = ids.contains(&b.id);

                if a_visited && !b_visited {
                    return Ordering::Greater;
                }
                if !a_visited && b_visited {
                    return Ordering::Less;
                }

                a.id.cmp(&b.id)
            });
            if let Some(win) = wins.first() {
                if !ids.contains(&win.id) {
                    ids.push(win.id);
                }
                focus_window_by_id(win.id)
            } else {
                Err(NO_MATCHING_WINDOW.to_owned())
            }
        }
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}

fn focus_window_by_id(id: u64) -> Result<String, String> {
    match ipc::query_niri(Request::Action(Action::FocusWindow { id }))? {
        Response::Handled => Ok(format!("Focused window with id {}", id)),
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}

fn window_matches(w: &Window, match_opts: &MatchOptions) -> bool {
    log::debug!("Matching window {:?}", w);
    if w.app_id.is_none() && match_opts.app_id.is_some()
        || match_opts.app_id.as_ref().is_some_and(|rx| {
            !Regex::new(rx).unwrap().is_match(w.app_id.as_ref().unwrap())
        })
    {
        log::debug!("app-id does not match.");
        return false;
    }

    if w.title.is_none() && match_opts.title.is_some()
        || match_opts.title.as_ref().is_some_and(|rx| {
            !Regex::new(rx).unwrap().is_match(w.title.as_ref().unwrap())
        })
    {
        log::debug!("title does not match.");
        return false;
    }

    true
}

fn toggle_mark() -> Result<String, String> {
    let focused_win = get_focused_window()?;
    match MARKED_WIN_IDS.lock() {
        Ok(mut ids) => {
            if ids.contains(&focused_win.id) {
                if let Some(index) =
                    ids.iter().position(|id| *id == focused_win.id)
                {
                    // swap_remove() would be more efficient but I think we
                    // want to retain the order.
                    ids.remove(index);
                }
                Ok(format!("Unset mark for window {:?}", focused_win))
            } else {
                ids.push(focused_win.id);
                Ok(format!("Set mark for window {:?}", focused_win))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

fn focus_marked() -> Result<String, String> {
    let mut marked_windows =
        MARKED_WIN_IDS.lock().expect("Could not lock mutex.");
    if marked_windows.is_empty() {
        return Err("No marked windows.".to_owned());
    }

    let mut already_focused = ALREADY_FOCUSED_WIN_IDS
        .lock()
        .expect("Could not lock mutex");

    // Do some cleanup, i.e., remove window ids from MARKED_WIN_IDS which don't
    // exist anymore.
    match ipc::query_niri(Request::Windows)? {
        Response::Windows(wins) => {
            // Remove marked window ids that don't exist anymore.
            marked_windows.retain(|mw| wins.iter().any(|w| &w.id == mw));
        }
        x => return Err(format!("Received unexpected reply {:?}", x)),
    }

    // The currently focused window is already visited, too.
    if let Ok(current_win) = get_focused_window() {
        if !already_focused.contains(&current_win.id) {
            already_focused.push(current_win.id);
        }
    }

    // If we already visited all of the marked window, start a new
    // cycle.
    if marked_windows.iter().all(|w| already_focused.contains(w)) {
        already_focused.clear();
    }

    if let Some(win_id) = marked_windows
        .iter()
        .find(|id| !already_focused.contains(id))
    {
        already_focused.push(*win_id);
        focus_window_by_id(*win_id)
    } else {
        Err("No marked window.".to_owned())
    }
}

fn list_marked() -> Result<String, String> {
    let mut marked_windows =
        MARKED_WIN_IDS.lock().expect("Could not lock mutex.");
    if marked_windows.is_empty() {
        return Err("No marked windows.".to_owned());
    }

    match ipc::query_niri(Request::Windows)? {
        Response::Windows(wins) => {
            // Remove marked window ids that don't exist anymore.
            marked_windows.retain(|mw| wins.iter().any(|w| &w.id == mw));
            let wins: Vec<&Window> = marked_windows
                .iter()
                .flat_map(|id| wins.iter().find(|w| &w.id == id))
                .collect();
            let mut str = String::new();
            for win in wins {
                let line = format!(
                    "id: {}, app-id: {:?}, title: {:?}, on workspace: {:?}",
                    win.id, win.app_id, win.title, win.workspace_id
                );
                str.push_str(line.as_str());
                str.push('\n');
            }
            Ok(str)
        }
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}
