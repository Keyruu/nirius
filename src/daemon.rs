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

//! Functions and data structures of the swayrd daemon.

use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;

use crate::cmds;
use crate::util;

pub fn run_daemon() {
    serve_client_requests();
}

pub fn serve_client_requests() {
    let socket_path = util::get_nirius_socket_path();

    match std::fs::exists(&socket_path) {
        Ok(true) => match std::fs::remove_file(&socket_path) {
            Ok(()) => log::debug!(
                "Deleted stale socket {} from previous run.",
                socket_path
            ),
            Err(e) => {
                panic!(
                    "Could not delete stale socket {}.\n{:?}",
                    socket_path, e
                );
            }
        },
        Err(err) => {
            panic!("Error when trying to access {}.\n{:?}", socket_path, err)
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
            log::debug!("Received command: {:?}", cmd);
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
