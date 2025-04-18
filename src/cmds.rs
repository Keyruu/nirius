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

use crate::ipc;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub enum NiriusCmd {
    FocusOrSpawn {
        #[clap(flatten)]
        match_opts: MatchOptions,
        command: Vec<String>,
    },
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct MatchOptions {
    #[clap(short = 'a', long, help = "Matches window app-ids")]
    app_id: Option<String>,

    #[clap(short = 't', long, help = "Matches window titles")]
    title: Option<String>,
}

pub fn exec_nirius_cmd(cmd: NiriusCmd) -> Result<String, String> {
    match cmd {
        NiriusCmd::FocusOrSpawn {
            match_opts,
            command,
        } => focus_or_spawn(match_opts, command),
    }
}

fn focus_or_spawn(
    match_opts: MatchOptions,
    command: Vec<String>,
) -> Result<String, String> {
    match ipc::query_niri(niri_ipc::Request::Windows)? {
        niri_ipc::Response::Windows(wins) => {
            if let Some(win) =
                wins.iter().find(|w| window_matches(w, &match_opts))
            {
                focus_window(win.id)
            } else {
                let r = ipc::query_niri(niri_ipc::Request::Action(
                    niri_ipc::Action::Spawn { command },
                ))?;
                match r {
                    niri_ipc::Response::Handled => {
                        Ok("Spawned successfully".to_string())
                    }
                    x => Err(format!("Received unexpected reply {:?}", x)),
                }
            }
        }
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}

fn focus_window(id: u64) -> Result<String, String> {
    match ipc::query_niri(niri_ipc::Request::Action(
        niri_ipc::Action::FocusWindow { id },
    ))? {
        niri_ipc::Response::Handled => {
            Ok(format!("Focused window with id {}", id))
        }
        x => Err(format!("Received unexpected reply {:?}", x)),
    }
}

fn window_matches(w: &niri_ipc::Window, match_opts: &MatchOptions) -> bool {
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
