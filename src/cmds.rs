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

use crate::{ipc, state::STATE};
use niri_ipc::{Action, Request, Response, Window, WorkspaceReferenceArg};
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
    /// Toggles the scratchpad state of the current window.
    ///
    /// If it's no scratchpad window currently, makes it foating (if it's not
    /// already) and moves it to the scratchpad workspace (the bottom-most
    /// workspace).
    ///
    /// If it's already a scratchpad window, removes it from there, i.e., from
    /// then on, it's just a normal window.
    ScratchpadToggle,
    /// Shows a window from the scratchpad or moves it back to the scratchpad
    /// if the current window is a scratchpad window.  Repeated invocations
    /// cycle through all scratchpad windows.
    ScratchpadShow,
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct MatchOptions {
    #[clap(short = 'a', long, help = "A regex  matched on window app-ids")]
    app_id: Option<String>,

    #[clap(short = 't', long, help = "A regex matched on window titles")]
    title: Option<String>,
}

static DEFAULT_MARK: &str = "__default__";

pub fn exec_nirius_cmd(cmd: NiriusCmd) -> Result<String, String> {
    match &cmd {
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
        NiriusCmd::ScratchpadToggle => scratchpad_toggle(),
        NiriusCmd::ScratchpadShow => scratchpad_show(),
    }
}

fn toggle_follow_mode() -> Result<String, String> {
    let mut w_state = STATE.write().expect("Could not write() STATE.");
    if let Some(focused_win_id) = w_state.get_focused_win_id() {
        if w_state.follow_mode_win_ids.contains(&focused_win_id) {
            if let Some(index) = w_state
                .follow_mode_win_ids
                .iter()
                .position(|id| *id == focused_win_id)
            {
                // swap_remove() would be more efficient but I think we
                // want to retain the order.
                w_state.follow_mode_win_ids.remove(index);
            }
            Ok(format!("Disabled follow mode for window {focused_win_id}"))
        } else {
            w_state.follow_mode_win_ids.push(focused_win_id);
            Ok(format!("Enabled follow mode for window {focused_win_id}"))
        }
    } else {
        Err("No focused window".to_owned())
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
    let state = STATE.read().expect("Could not read() STATE.");
    if let Some(win) = state
        .all_windows
        .iter()
        .find(|w| window_matches(w, match_opts))
    {
        focus_window_by_id(win.id)
    } else {
        Err(NO_MATCHING_WINDOW.to_owned())
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

fn move_to_current_workspace(
    match_opts: &MatchOptions,
    focus: bool,
) -> Result<String, String> {
    let state = STATE.read().expect("Could not read() STATE");
    let focused_ws_id = state
        .get_focused_workspace_id()
        .ok_or("No focused workspace.")?;
    if let Some(win) = state.all_windows.iter().find(|w| {
        w.workspace_id.is_none_or(|ws_id| ws_id != focused_ws_id)
            && window_matches(w, match_opts)
    }) {
        let move_result = move_window_to_workspace(
            win.id,
            niri_ipc::WorkspaceReferenceArg::Id(focused_ws_id),
            focus,
        );
        if focus {
            focus_window_by_id(win.id)?;
        }
        move_result
    } else {
        Err(NO_MATCHING_WINDOW.to_owned())
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

pub fn move_window_to_workspace(
    window_id: u64,
    workspace_ref: niri_ipc::WorkspaceReferenceArg,
    focus: bool,
) -> Result<String, String> {
    match ipc::query_niri(Request::Action(Action::MoveWindowToWorkspace {
        window_id: Some(window_id),
        reference: workspace_ref,
        focus,
    }))? {
        Response::Handled => Ok("Moved successfully".to_string()),
        x => Err(format!("Received unexpected reply {x:?}")),
    }
}

fn toggle_mark(mark: String) -> Result<String, String> {
    let mut state = STATE.write().expect("Could not write() STATE.");
    if let Some(focused_win_id) = state.get_focused_win_id() {
        let ids = state.mark_to_win_ids.entry(mark).or_default();
        if ids.contains(&focused_win_id) {
            if let Some(index) = ids.iter().position(|id| *id == focused_win_id)
            {
                // swap_remove() would be more efficient but I think we
                // want to retain the order.
                ids.remove(index);
            }
            Ok(format!("Unset mark for window {focused_win_id:?}"))
        } else {
            ids.push(focused_win_id);
            Ok(format!("Set mark for window {focused_win_id:?}"))
        }
    } else {
        Err("No focused window.".to_owned())
    }
}

fn focus_marked(mark: String) -> Result<String, String> {
    let state = STATE.read().expect("Could not read() STATE.");

    if let Some(marked_windows) = state.mark_to_win_ids.get(&mark).cloned() {
        if let Some(win) = state
            .all_windows
            .iter()
            .find(|w| marked_windows.contains(&w.id))
        {
            focus_window_by_id(win.id)
        } else {
            Err("No marked window.".to_owned())
        }
    } else {
        Err("No such mark.".to_owned())
    }
}

fn list_marked(mark: String) -> Result<String, String> {
    let state = STATE.read().expect("Could not read() STATE.");

    if let Some(marked_windows) = state.mark_to_win_ids.get(&mark).cloned() {
        {
            let wins: Vec<&Window> = state
                .all_windows
                .iter()
                .filter(|w| marked_windows.contains(&w.id))
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
    } else {
        Err("No such mark.".to_owned())
    }
}

fn list_all_marked() -> Result<String, String> {
    let keys: Vec<String>;
    // In a block so that we drop the RwLock before calling list_marked().  Not
    // strictly needed anymore since we switched from a Mutex to a RwLock, but
    // anyway.
    {
        keys = STATE
            .read()
            .expect("Could not read() STATE.")
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

fn scratchpad_toggle() -> Result<String, String> {
    let mut state = STATE.write().expect("Could not write() STATE.");
    if let Some(id) = state.get_focused_win_id() {
        if state.scratchpad_win_ids.contains(&id) {
            state.scratchpad_win_ids.retain(|wid| *wid != id);
            Ok(format!("Removed window {id} from scratchpad."))
        } else {
            state.scratchpad_win_ids.push(id);
            drop(state);
            scratchpad_move()
        }
    } else {
        Err("No focused window.".to_owned())
    }
}

pub(crate) fn scratchpad_move() -> Result<String, String> {
    let state = STATE.read().expect("Could not read() STATE.");
    let output = state
        .all_workspaces
        .iter()
        .find(|ws| ws.is_focused)
        .map(|ws| ws.output.clone())
        .expect("No workspace is focused.");
    let ws_id = state
        .all_workspaces
        .iter()
        .filter(|ws| ws.output == output)
        .max_by(|a, b| a.idx.cmp(&b.idx))
        .map(|ws| ws.id)
        .expect("No max workspace.");
    let mut i = 0;
    for w in state
        .all_windows
        .iter()
        .filter(|w| state.scratchpad_win_ids.contains(&w.id))
    {
        if !w.is_floating {
            ipc::query_niri(Request::Action(Action::ToggleWindowFloating {
                id: Some(w.id),
            }))?;
        }
        move_window_to_workspace(
            w.id,
            niri_ipc::WorkspaceReferenceArg::Id(ws_id),
            false,
        )?;
        i += 1;
    }
    Ok(format!(
        "Moved {i} scratchpad windows to workspace with id {ws_id}."
    ))
}

fn scratchpad_show() -> Result<String, String> {
    let state = STATE.read().expect("Could not read STATE.");
    let opt_win_id = state.get_focused_win_id();
    if opt_win_id
        .as_ref()
        .is_some_and(|w| state.scratchpad_win_ids.contains(w))
    {
        scratchpad_move()
    } else {
        let focused_ws_id = state
            .get_focused_workspace_id()
            .ok_or("No focused workspace.")?;

        if let Some(id) = state
            .all_windows
            .iter()
            .find(|w| state.scratchpad_win_ids.contains(&w.id))
            .map(|w| w.id)
        {
            move_window_to_workspace(
                id,
                WorkspaceReferenceArg::Id(focused_ws_id),
                true,
            )?;
            focus_window_by_id(id)
        } else {
            Err("No window in the scratchpad.".to_owned())
        }
    }
}
