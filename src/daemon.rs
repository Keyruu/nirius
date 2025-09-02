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

//! Functions and data structures of the niriusd daemon.

use std::io::ErrorKind;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;

use niri_ipc::Request;
use niri_ipc::Response;
use niri_ipc::WorkspaceReferenceArg;

use crate::cmds;
use crate::state::STATE;
use crate::util;

pub fn run_daemon() {
    std::thread::spawn(process_events);
    serve_client_requests();
}

fn process_events() -> std::io::Result<()> {
    let mut socket = niri_ipc::socket::Socket::connect()?;

    match socket.send(Request::EventStream) {
        Ok(response) => match response {
            Ok(Response::Handled) => {
                let mut read_event = socket.read_events();
                loop {
                    match read_event() {
                        Ok(event) => match handle_event(&event) {
                            Ok(msg) => {
                                log::info!(
                                    "Handled event successfully: {event:?} => {msg}"
                                )
                            }
                            Err(e) => {
                                log::error!(
                                    "Error during event-handling: {e:?}"
                                )
                            }
                        },
                        Err(err) => {
                            if err.kind() == ErrorKind::UnexpectedEof {
                                log::error!(
                                    "Received EOF, niri has quit and so do I. Goodbye!"
                                );
                                std::process::exit(0)
                            }
                            log::error!("Could not read event: {err:?}")
                        }
                    }
                }
            }
            Ok(other) => {
                let msg = format!(
                    "Unexpected response for Request::EventStream: {other:?}"
                );
                log::error!("{msg}");
                panic!("{msg}")
            }
            Err(e) => {
                let msg = format!("Error when requesting EventStream: {e:?}");
                log::error!("{msg}");
                panic!("{msg}")
            }
        },
        Err(e) => {
            let msg = format!("Could not send Request::EventStream: {e:?}");
            log::error!("{msg}");
            panic!("{msg}")
        }
    }
}

fn handle_event(event: &niri_ipc::Event) -> Result<String, String> {
    match event {
        niri_ipc::Event::WorkspaceActivated { id, focused } if *focused => {
            move_follow_mode_windows(*id)
        }
        niri_ipc::Event::WindowClosed { id } => {
            let mut state = STATE.lock().expect("Could not lock state.");
            state.remove_window(id);
            Ok(String::new())
        }
        _other => Ok("Nothing to do.".to_owned()),
    }
}

fn move_follow_mode_windows(workspace_id: u64) -> Result<String, String> {
    let state = STATE.lock().expect("Could not lock mutex");
    let mut n = 0;
    for id in state.follow_mode_win_ids.iter() {
        n+=1;
        crate::ipc::query_niri(Request::Action(
            niri_ipc::Action::MoveWindowToWorkspace {
                window_id: Some(*id),
                reference: WorkspaceReferenceArg::Id(workspace_id),
                focus: true,
            },
        ))?;
    }
    Ok(format!("Moved {n} follow-mode windows."))
}

fn serve_client_requests() {
    let socket_path = util::get_nirius_socket_path();

    match std::fs::exists(&socket_path) {
        Ok(true) => match std::fs::remove_file(&socket_path) {
            Ok(()) => log::debug!(
                "Deleted stale socket {socket_path} from previous run."
            ),
            Err(e) => {
                panic!("Could not delete stale socket {socket_path}.\n{e:?}");
            }
        },
        Err(err) => {
            panic!("Error when trying to access {socket_path}.\n{err:?}")
        }
        _ => (),
    };

    log::debug!("niriusd starts listening on {socket_path}.");

    match UnixListener::bind(socket_path) {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        handle_client_request(stream);
                    }
                    Err(err) => {
                        log::error!("Error handling client request: {err}");
                    }
                }
            }
        }
        Err(err) => {
            log::error!("Could not bind socket: {err}")
        }
    }
}

fn handle_client_request(stream: UnixStream) {
    match serde_json::from_reader::<_, cmds::NiriusCmd>(&stream) {
        Ok(cmd) => {
            log::debug!("Received command: {cmd:?}");
            if let Err(err) = stream.shutdown(std::net::Shutdown::Read) {
                log::error!("Could not shutdown stream for read: {err}")
            }
            let result = cmds::exec_nirius_cmd(cmd);
            log::debug!("Executed command, returning result {result:?}");
            if let Err(err) = serde_json::to_writer(&stream, &result) {
                log::error!("Couldn't send result back to client: {err}");
            }
            if let Err(err) = stream.shutdown(std::net::Shutdown::Write) {
                log::error!("Could not shutdown stream for read: {err}");
            }
        }
        Err(err) => {
            log::error!("Could not read command from client: {err}");
        }
    }
}
