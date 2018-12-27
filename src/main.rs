
extern crate websocket;

use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

use websocket::OwnedMessage;
use websocket::sync::Server;

fn main() {
	let server = Server::bind("0.0.0.0:12223").unwrap();

	let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<String>();
	let client_senders: Arc<Mutex<Vec<mpsc::Sender<String>>>> = Arc::new(Mutex::new(vec![]));

	{
		let client_senders = client_senders.clone();
		thread::spawn(move || {
			while let Ok(msg) = dispatcher_rx.recv() {
				for sender in client_senders.lock().unwrap().iter() {
					sender.send(msg.clone()).unwrap();
				}
			}
		});
	}

	for request in server.filter_map(Result::ok) {
        let dispatcher = dispatcher_tx.clone();
		let (client_tx, client_rx) = mpsc::channel();
		client_senders.lock().unwrap().push(client_tx);

		thread::spawn(move || {
            let mut client = request.use_protocol("rust-websocket").accept().unwrap();
	        let ip = client.peer_addr().unwrap();

            println!("CONNECTED: {}", ip);

            let message = OwnedMessage::Text("Welcome.".to_string());
            client.send_message(&message).unwrap();

            let (mut receiver, mut sender) = client.split().unwrap();

            let(tx, rx) = mpsc::channel::<OwnedMessage>();
            thread::spawn(move || {
                for message in receiver.incoming_messages() {
                    tx.send(message.unwrap()).unwrap();
                }
            });

            loop {
                if let Ok(message) = rx.try_recv() {
                    match message {
                        OwnedMessage::Close(_) => {
                            let message = OwnedMessage::Close(None);
                            sender.send_message(&message).unwrap();
                            println!("DISCONNECTED: {}", ip);
                            return;
                        },
                        OwnedMessage::Ping(payload) => {
                            let message = OwnedMessage::Pong(payload);
                            sender.send_message(&message).unwrap();
                        },
                        OwnedMessage::Text(payload_string) => {
                            let msg_string = format!("MESSAGE: {}: ", payload_string);
                            dispatcher.send(msg_string).unwrap();
                        },
                        _ => {
                        }
                    }
                }
                if let Ok(message) = client_rx.try_recv() {
                    let message = OwnedMessage::Text(message);
                    sender.send_message(&message).unwrap();
                }
            }
        });
    }
}
