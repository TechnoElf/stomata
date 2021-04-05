use std::thread;
use std::sync::{Mutex, Arc};

use mysql::Conn;

use websocket::sync::Server;
use websocket::OwnedMessage;

const WS_PORT: usize = 8001;

pub fn run(_db_conn: Arc<Mutex<Conn>>) {
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
