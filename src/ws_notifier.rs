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
use std::time::{Duration, Instant};
use std::thread;

use mysql::Pool;
use serde::{Deserialize, Serialize};
use serde_json;
use websocket::sync::{Server, Client, stream::TcpStream};
use websocket::OwnedMessage;

use crate::model::*;
use crate::auth::*;

const WS_PORT: usize = 8001;
const ALIVE_TIMEOUT: Duration = Duration::new(600, 0);

pub fn run(db_conn: Arc<Mutex<Pool>>, reqs: Arc<Mutex<Vec<WsRequest>>>) {
    let mut server = Server::bind(format!("0.0.0.0:{}", WS_PORT)).unwrap();
    server.set_nonblocking(true).unwrap();
    println!("[WS]: Started");

    let mut connections: Vec<Connection> = Vec::new();
    let mut stations: Vec<(Connection, usize)> = Vec::new();

    loop {
        if let Ok(request) = server.accept() {
            let cli = request.accept().unwrap();
            let addr = cli.peer_addr().unwrap();
            cli.set_nonblocking(true).unwrap();
            connections.push(Connection {
                cli,
                last_seen: Instant::now()
            });
            println!("[WS]: Connection from {}", addr);
        }

        connections = connections.into_iter().filter_map(|c| process_con(c, &mut stations, db_conn.clone())).collect();
        stations = stations.into_iter().filter_map(|s| process_station(s)).collect();

        let reqs = std::mem::replace(reqs.lock().unwrap().as_mut(), Vec::new());
        for r in reqs.into_iter() {
            match r {
                WsRequest::UpdateState(r) => {
                    if let Some(station) = stations.iter_mut().find(|(_, id)| id == &r.id) {
                        let msg = OwnedMessage::Text(serde_json::to_string(&StateMessage {
                            state: r.state
                        }).unwrap());
                        station.0.cli.send_message(&msg).unwrap();
                    }
                },
                WsRequest::UpdateConf(r) => {
                    if let Some(station) = stations.iter_mut().find(|(_, id)| id == &r.id) {
                        let msg = OwnedMessage::Text(serde_json::to_string(&ConfMessage {
                            conf: r.conf
                        }).unwrap());
                        station.0.cli.send_message(&msg).unwrap();
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

fn process_con(mut con: Connection, stations: &mut Vec<(Connection, usize)>, db_conn: Arc<Mutex<Pool>>) -> Option<Connection> {
    if let Ok(msg) = con.cli.recv_message() {
        match msg {
            OwnedMessage::Close(_) => {
                let msg = OwnedMessage::Close(None);
                con.cli.send_message(&msg).unwrap();
                return None;
            }
            OwnedMessage::Ping(ping) => {
                let msg = OwnedMessage::Pong(ping);
                con.cli.send_message(&msg).unwrap();
                con.last_seen = Instant::now();
            }
            OwnedMessage::Text(data) => {
                if let Ok(reg) = serde_json::from_str::<RegisterMessage>(&data) {
                    let mut db = db_conn.lock().unwrap().get_conn().unwrap();
                    if let Ok(token) = get_station(reg.id, &mut db).map(|s| s.token) {
                        let auth = BasicAuth::from_parts(&reg.id.to_string(), &reg.token);
                        if auth.verify(&token) {
                            con.last_seen = Instant::now();
                            let msg = OwnedMessage::Text("{}".to_string());
                            con.cli.send_message(&msg).unwrap();

                            stations.iter_mut().filter(|(_, id)| id == &reg.id).for_each(|(con, _)| con.cli.send_message(&OwnedMessage::Close(None)).unwrap());
                            stations.retain(|(_, id)| id != &reg.id);
                            stations.push((con, reg.id));

                            println!("[WS]: Station {:?} registered", reg.id);
                            return None;
                        }
                    }
                }
            }
            _ => (),
        }
    }

    if con.last_seen.elapsed() < ALIVE_TIMEOUT {
        Some(con)
    } else {
        None
    }
}

fn process_station((mut con, id): (Connection, usize)) -> Option<(Connection, usize)> {
    if let Ok(msg) = con.cli.recv_message() {
        match msg {
            OwnedMessage::Close(_) => {
                let msg = OwnedMessage::Close(None);
                con.cli.send_message(&msg).unwrap();
                println!("[WS]: Station {:?} disconnected (close)", id);
                return None;
            }
            OwnedMessage::Ping(ping) => {
                let msg = OwnedMessage::Pong(ping);
                con.cli.send_message(&msg).unwrap();
                con.last_seen = Instant::now();
            }
            _ => (),
        }
    }

    if con.last_seen.elapsed() < ALIVE_TIMEOUT {
        Some((con, id))
    } else {
        println!("[WS]: Station {:?} disconnected (timeout)", id);
        None
    }
}

struct Connection {
    cli: Client<TcpStream>,
    last_seen: Instant
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

#[derive(Debug, Serialize)]
struct ConfMessage {
    conf: String
}

pub enum WsRequest {
    UpdateState(WsUpdateState),
    UpdateConf(WsUpdateConf)
}

pub struct WsUpdateState {
    pub id: usize,
    pub state: String
}

pub struct WsUpdateConf {
    pub id: usize,
    pub conf: String
}
