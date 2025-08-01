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

use crate::{ipc, state::STATE};
use niri_ipc::{Action, Request, Response, Window, Workspace};
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
    /// Move a window matching the given options to the current workspace.
    /// Only windows of unfocused workspaces are considered.  If there is no
    /// such window, exit non-zero.
    MoveToCurrentWorkspace {
        #[clap(flatten)]
        match_opts: MatchOptions,
        #[clap(
            short = 'f',
            long,
            help = "Focus the window after moving it to the current workspace."
        )]
        focus: bool,
    },
    /// Move a window matching the given options to the current workspace.
    /// Only windows of unfocused workspaces are considered.  If there is no
    /// such window, spawn the given command.
    MoveToCurrentWorkspaceOrSpawn {
        #[clap(flatten)]
        match_opts: MatchOptions,
        #[clap(
            short = 'f',
            long,
            help = "Focus the window after moving it to the current workspace."
        )]
        focus: bool,
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
    /// Marks or unmarks the currently focused window with the given or default
    /// mark.  You can switch to the marked window or cycle trough all marked
    /// windows using the `focus-marked` command.
    ToggleMark { mark: Option<String> },
    /// Focuses the window with the given mark or the default mark, if no mark
    /// is given.  If there are multiple marked windows, cycles through all of
    /// them.  To mark a window, use the `toggle-mark` command.
    FocusMarked { mark: Option<String> },
    /// List all windows with the given or default mark, if no mark is given,
    /// on stdout.
    ListMarked {
        mark: Option<String>,
        #[clap(short = 'a', long, help = "List all marks with their windows")]
        all: bool,
    },
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct MatchOptions {
    #[clap(short = 'a', long, help = "A regex  matched on window app-ids")]
    app_id: Option<String>,

    #[clap(short = 't', long, help = "A regex matched on window titles")]
    title: Option<String>,
}

static DEFAULT_MARK: &str = "__default__";
static LAST_COMMAND: Mutex<Option<NiriusCmd>> = Mutex::new(None);

pub fn exec_nirius_cmd(cmd: NiriusCmd) -> Result<String, String> {
    let mut last_command =
        LAST_COMMAND.lock().expect("Could not lock LAST_COMMAND.");
    let clear_focused_win_ids =
        last_command.as_ref().is_some_and(|lc| lc != &cmd);

    let result = match &cmd {
        NiriusCmd::Nop => Ok("Nothing done".to_string()),
        NiriusCmd::Focus { match_opts } => focus(match_opts),
        NiriusCmd::FocusOrSpawn {
            match_opts,
            command,
        } => focus_or_spawn(match_opts, command),
        NiriusCmd::MoveToCurrentWorkspace { match_opts, focus } => {
            move_to_current_workspace(match_opts, *focus)
        }
        NiriusCmd::MoveToCurrentWorkspaceOrSpawn {
            match_opts,
            focus,
            command,
        } => move_to_current_workspace_or_spawn(match_opts, *focus, command),
        NiriusCmd::ToggleFollowMode => toggle_follow_mode(),
        NiriusCmd::ToggleMark { mark } => {
            toggle_mark(mark.clone().unwrap_or(DEFAULT_MARK.to_owned()))
        }
        NiriusCmd::FocusMarked { mark } => {
            focus_marked(mark.clone().unwrap_or(DEFAULT_MARK.to_owned()))
        }
        NiriusCmd::ListMarked { mark, all } => {
            if *all {
                list_all_marked()
            } else {
                list_marked(mark.clone().unwrap_or(DEFAULT_MARK.to_owned()))
            }
        }
    };

    if clear_focused_win_ids {
        STATE
            .lock()
            .expect("Could not lock STATE.")
            .already_focused_win_ids
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
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn toggle_follow_mode() -> Result<String, String> {
    let focused_win = get_focused_window()?;
    let mut state = STATE.lock().expect("Could not lock state.");
    if state.follow_mode_win_ids.contains(&focused_win.id) {
        if let Some(index) = state
            .follow_mode_win_ids
            .iter()
            .position(|id| *id == focused_win.id)
        {
            // swap_remove() would be more efficient but I think we
            // want to retain the order.
            state.follow_mode_win_ids.remove(index);
        }
        Ok(format!("Disabled follow mode for window {focused_win:?}"))
    } else {
        state.follow_mode_win_ids.push(focused_win.id);
        Ok(format!("Enabled follow mode for window {focused_win:?}"))
    }
}

fn focus_or_spawn(
    match_opts: &MatchOptions,
    command: &[String],
) -> Result<String, String> {
    match focus(match_opts) {
        Err(str) if NO_MATCHING_WINDOW == str => {
            match ipc::query_niri(Request::Action(Action::Spawn {
                command: command.to_vec(),
            }))? {
                Response::Handled => Ok("Spawned successfully".to_string()),
                x => Err(format!("Received unexpected reply {x:?}")),
            }
        }
        x => x,
    }
}

fn focus(match_opts: &MatchOptions) -> Result<String, String> {
    match ipc::query_niri(Request::Windows)? {
        Response::Windows(mut wins) => {
            let mut state = STATE.lock().expect("Could not lock mutex");
            wins.retain(|w| window_matches(w, match_opts));
            if wins
                .iter()
                .all(|w| state.already_focused_win_ids.contains(&w.id))
            {
                state.already_focused_win_ids.clear();
            }
            wins.sort_by(|a, b| {
                if a.is_focused {
                    return Ordering::Greater;
                }
                if b.is_focused {
                    return Ordering::Less;
                }

                let a_visited = state.already_focused_win_ids.contains(&a.id);
                let b_visited = state.already_focused_win_ids.contains(&b.id);

                if a_visited && !b_visited {
                    return Ordering::Greater;
                }
                if !a_visited && b_visited {
                    return Ordering::Less;
                }

                a.id.cmp(&b.id)
            });
            if let Some(win) = wins.first() {
                if !state.already_focused_win_ids.contains(&win.id) {
                    state.already_focused_win_ids.push(win.id);
                }
                focus_window_by_id(win.id)
            } else {
                Err(NO_MATCHING_WINDOW.to_owned())
            }
        }
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn focus_window_by_id(id: u64) -> Result<String, String> {
    match ipc::query_niri(Request::Action(Action::FocusWindow { id }))? {
        Response::Handled => Ok(format!("Focused window with id {id}")),
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn window_matches(w: &Window, match_opts: &MatchOptions) -> bool {
    log::debug!("Matching window {w:?}");
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

fn get_focused_workspace() -> Result<Workspace, String> {
    match ipc::query_niri(Request::Workspaces)? {
        Response::Workspaces(workspaces) => workspaces
            .into_iter()
            .find(|ws| ws.is_focused)
            .ok_or(String::from("No focused workspace")),
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn move_to_current_workspace(
    match_opts: &MatchOptions,
    focus: bool,
) -> Result<String, String> {
    let focused_ws = get_focused_workspace()?;
    match ipc::query_niri(Request::Windows)? {
        Response::Windows(mut wins) => {
            wins.retain(|w| {
                // Only windows which are not on the current workspace already.
                w.workspace_id.is_none_or(|ws_id| ws_id != focused_ws.id)
                    && window_matches(w, match_opts)
            });

            if let Some(win) = wins.first() {
                let move_result =
                    move_window_to_workspace(win.id, focused_ws.id, focus);
                if focus {
                    focus_window_by_id(win.id)?;
                }
                move_result
            } else {
                Err(NO_MATCHING_WINDOW.to_owned())
            }
        }
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn move_to_current_workspace_or_spawn(
    match_opts: &MatchOptions,
    focus: bool,
    command: &[String],
) -> Result<String, String> {
    match move_to_current_workspace(match_opts, focus) {
        Err(str) if NO_MATCHING_WINDOW == str => {
            match ipc::query_niri(Request::Action(Action::Spawn {
                command: command.to_vec(),
            }))? {
                Response::Handled => Ok("Spawned successfully".to_string()),
                x => Err(format!("Received unexpected reply {x:?}")),
            }
        }
        x => x,
    }
}

fn move_window_to_workspace(
    window_id: u64,
    workspace_id: u64,
    focus: bool,
) -> Result<String, String> {
    match ipc::query_niri(Request::Action(Action::MoveWindowToWorkspace {
        window_id: Some(window_id),
        reference: niri_ipc::WorkspaceReferenceArg::Id(workspace_id),
        focus,
    }))? {
        Response::Handled => Ok("Moved successfully".to_string()),
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn toggle_mark(mark: String) -> Result<String, String> {
    let focused_win = get_focused_window()?;
    let mut state = STATE.lock().expect("Could not lock mutex.");
    let ids = state.mark_to_win_ids.entry(mark).or_default();
    if ids.contains(&focused_win.id) {
        if let Some(index) = ids.iter().position(|id| *id == focused_win.id) {
            // swap_remove() would be more efficient but I think we
            // want to retain the order.
            ids.remove(index);
        }
        Ok(format!("Unset mark for window {focused_win:?}"))
    } else {
        ids.push(focused_win.id);
        Ok(format!("Set mark for window {focused_win:?}"))
    }
}

fn focus_marked(mark: String) -> Result<String, String> {
    let mut state = STATE.lock().expect("Could not lock mutex.");

    if let Some(marked_windows) = state.mark_to_win_ids.get(&mark).cloned() {
        // The currently focused window is already visited, too.
        if let Ok(current_win) = get_focused_window() {
            if !state.already_focused_win_ids.contains(&current_win.id) {
                state.already_focused_win_ids.push(current_win.id);
            }
        }

        // If we already visited all of the marked window, start a new
        // cycle.
        if marked_windows
            .iter()
            .all(|w| state.already_focused_win_ids.contains(w))
        {
            state.already_focused_win_ids.clear();
        }

        if let Some(win_id) = marked_windows
            .iter()
            .find(|id| !state.already_focused_win_ids.contains(id))
        {
            state.already_focused_win_ids.push(*win_id);
            focus_window_by_id(*win_id)
        } else {
            Err("No marked window.".to_owned())
        }
    } else {
        Err("No such mark.".to_owned())
    }
}

fn list_marked(mark: String) -> Result<String, String> {
    let mut state = STATE.lock().expect("Could not lock state.");

    if let Some(marked_windows) = state.mark_to_win_ids.get_mut(&mark) {
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
            x => Err(format!("Received unexpected reply {x:?}")),
        }
    } else {
        Err("No such mark.".to_owned())
    }
}

fn list_all_marked() -> Result<String, String> {
    let keys: Vec<String>;
    // In a block so that the mutex is unlocked again immediately before we
    // call list_marked() which will lock again below in the loop.
    {
        keys = STATE
            .lock()
            .expect("Could not lock state.")
            .mark_to_win_ids
            .keys()
            .cloned()
            .collect::<Vec<String>>();
    }

    let mut s = String::new();
    for mark in keys {
        s.push_str(format!("-> {mark}:\n").as_str());
        match list_marked(mark.to_string()) {
            Ok(marks) => s.push_str(marks.as_str()),
            err @ Err(_) => return err,
        }
    }
    Ok(s)
}
