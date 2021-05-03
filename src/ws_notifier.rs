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

use std::sync::{Mutex, Arc};
use std::net::SocketAddr;

use mysql::Pool;
use serde::{Deserialize, Serialize};
use serde_json;
use websocket::sync::{Server, Client, stream::TcpStream};
use websocket::OwnedMessage;

const WS_PORT: usize = 8001;

pub fn run(_db_conn: Arc<Mutex<Pool>>) {
    let mut server = Server::bind(format!("0.0.0.0:{}", WS_PORT)).unwrap();
    server.set_nonblocking(true).unwrap();
    println!("[WS]: Started");

    let mut stations: Vec<Station> = Vec::new();

    loop {
        if let Ok(request) = server.accept() {
            let cli = request.accept().unwrap();
            let addr = cli.peer_addr().unwrap();
            stations.push(Station {
                cli,
                addr,
                id: None,
                token: None
            });
            println!("[WS]: Connection from {}", addr);
        }

        for station in stations.iter_mut() {
            if let Ok(msg) = station.cli.recv_message() {
                match msg {
                    OwnedMessage::Close(_) => {
                        let msg = OwnedMessage::Close(None);
                        station.cli.send_message(&msg).unwrap();
                        println!("[WS]: {} disconnected", station.addr);
                    }
                    OwnedMessage::Ping(ping) => {
                        let msg = OwnedMessage::Pong(ping);
                        station.cli.send_message(&msg).unwrap();
                    }
                    OwnedMessage::Text(data) => {
                        println!("[WS]: Received message from {}: {}", station.addr, data);
                        if station.id.is_some() {
                                let msg = serde_json::to_string(&StateMessage {
                                    state: "idle".to_string()
                                }).unwrap();
                                let msg = OwnedMessage::Text(msg);
                                station.cli.send_message(&msg).unwrap();
                        } else {
                            if let Ok(reg) = serde_json::from_str::<RegisterMessage>(&data) {
                                station.id = Some(reg.id);
                                station.token = Some(reg.token);
                                let msg = OwnedMessage::Text("{}".to_string());
                                station.cli.send_message(&msg).unwrap();
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

struct Station {
    cli: Client<TcpStream>,
    addr: SocketAddr,
    id: Option<usize>,
    token: Option<String>
}

#[derive(Debug, Deserialize)]
struct RegisterMessage {
    id: usize,
    token: String
}

#[derive(Debug, Serialize)]
struct StateMessage {
    state: String
}
