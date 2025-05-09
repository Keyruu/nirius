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
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct MatchOptions {
    #[clap(short = 'a', long, help = "A regex  matched on window app-ids")]
    app_id: Option<String>,

    #[clap(short = 't', long, help = "A regex matched on window titles")]
    title: Option<String>,
}

static FOCUSED_WIN_IDS: Mutex<Vec<u64>> = Mutex::new(vec![]);

pub fn exec_nirius_cmd(cmd: NiriusCmd) -> Result<String, String> {
    let mut clear_focused_win_ids = true;

    let result = match &cmd {
        NiriusCmd::Nop => Ok("Nothing done".to_string()),
        NiriusCmd::Focus { match_opts } => {
            clear_focused_win_ids = false;
            focus(match_opts)
        }
        NiriusCmd::FocusOrSpawn {
            match_opts,
            command,
        } => {
            clear_focused_win_ids = false;
            focus_or_spawn(match_opts, command)
        }
    };

    if clear_focused_win_ids {
        FOCUSED_WIN_IDS
            .lock()
            .expect("Could not lock mutex")
            .clear()
    }

    result
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
            let mut ids = FOCUSED_WIN_IDS.lock().expect("Could not lock mutex");
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
