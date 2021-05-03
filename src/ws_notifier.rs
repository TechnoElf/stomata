/*
 * stomata - Backend for the Thyme project
 * Copyright (C) 2021 TechnoElf
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::thread;
use std::sync::{Mutex, Arc};

use mysql::Pool;

use websocket::sync::Server;
use websocket::OwnedMessage;

const WS_PORT: usize = 8001;

pub fn run(_db_conn: Arc<Mutex<Pool>>) {
    let server = Server::bind(format!("0.0.0.0:{}", WS_PORT)).unwrap();
    println!("[WS]: Started");
    for request in server.filter_map(Result::ok) {
        thread::spawn(|| {
            let client = request.accept().unwrap();
            let addr = client.peer_addr().unwrap();
            println!("[WS]: Connection from {}", addr);
            let (mut recv, mut send) = client.split().unwrap();

            for message in recv.incoming_messages() {
                if let Ok(message) = message {
                    match message {
                        OwnedMessage::Close(_) => {
                            let message = OwnedMessage::Close(None);
                            send.send_message(&message).unwrap();
                            println!("[WS]: Client {} disconnected", addr);
                            break;
                        }
                        OwnedMessage::Ping(ping) => {
                            let message = OwnedMessage::Pong(ping);
                            send.send_message(&message).unwrap();
                        }
                        _ => send.send_message(&message).unwrap(),
                    }
                } else {
                    break;
                }
            }
        });
    }
}
