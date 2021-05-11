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
use std::cmp::Ordering;

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

    let mut stations: Vec<Station> = Vec::new();

    loop {
        if let Ok(request) = server.accept() {
            let cli = request.accept().unwrap();
            let addr = cli.peer_addr().unwrap();
            cli.set_nonblocking(true).unwrap();
            stations.push(Station {
                cli,
                id: None,
                last_seen: Instant::now()
            });
            println!("[WS]: Connection from {}", addr);
        }

        stations = stations.into_iter().filter_map(|s| process_rx(s, db_conn.clone())).collect();
        stations.sort_by(|a, b| {
            if let Some(a_id) = a.id {
                if let Some(b_id) = b.id {
                    if a_id == b_id {
                        b.last_seen.cmp(&a.last_seen)
                    } else {
                        b_id.cmp(&a_id)
                    }
                } else {
                    Ordering::Less
                }
            } else {
                Ordering::Greater
            }
        });
        stations.dedup_by(|a, b| {
            if let Some(a_id) = a.id {
                if let Some(b_id) = b.id {
                    if a_id == b_id {
                        println!("[WS]: Station {:?} disconnected (duplication)", b.id);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        });

        let reqs = std::mem::replace(reqs.lock().unwrap().as_mut(), Vec::new());
        for r in reqs.into_iter() {
            match r {
                WsRequest::UpdateState(r) => {
                    if let Some(station) = stations.iter_mut().find(|s| s.id == Some(r.id)) {
                        let msg = OwnedMessage::Text(serde_json::to_string(&StateMessage {
                            state: r.state
                        }).unwrap());
                        station.cli.send_message(&msg).unwrap();
                    }
                },
                WsRequest::UpdateConf(r) => {
                    if let Some(station) = stations.iter_mut().find(|s| s.id == Some(r.id)) {
                        let msg = OwnedMessage::Text(serde_json::to_string(&ConfMessage {
                            conf: r.conf
                        }).unwrap());
                        station.cli.send_message(&msg).unwrap();
                    }
                }
            }
            
        }
    }
}

fn process_rx(mut station: Station, db_conn: Arc<Mutex<Pool>>) -> Option<Station> {
    if let Ok(msg) = station.cli.recv_message() {
        match msg {
            OwnedMessage::Close(_) => {
                let msg = OwnedMessage::Close(None);
                station.cli.send_message(&msg).unwrap();
                println!("[WS]: Station {:?} disconnected (close)", station.id);
                return None;
            }
            OwnedMessage::Ping(ping) => {
                let msg = OwnedMessage::Pong(ping);
                station.cli.send_message(&msg).unwrap();
                station.last_seen = Instant::now();
            }
            OwnedMessage::Text(data) => {
                if station.id.is_none() {
                    if let Ok(reg) = serde_json::from_str::<RegisterMessage>(&data) {
                        let mut db = db_conn.lock().unwrap().get_conn().unwrap();
                        let token = get_station(reg.id, &mut db).unwrap().token;
                        let auth = BasicAuth::from_parts(&reg.id.to_string(), &reg.token);

                        if auth.verify(&token) {
                            station.id = Some(reg.id);
                            let msg = OwnedMessage::Text("{}".to_string());
                            station.cli.send_message(&msg).unwrap();
                            station.last_seen = Instant::now();
                        }
                    }
                }
            }
            _ => (),
        }
    }

    if station.last_seen.elapsed() < ALIVE_TIMEOUT {
        Some(station)
    } else {
        println!("[WS]: Station {:?} disconnected (timeout)", station.id);
        None
    }
}

struct Station {
    cli: Client<TcpStream>,
    id: Option<usize>,
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
