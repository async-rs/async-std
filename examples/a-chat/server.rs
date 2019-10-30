use std::{
    collections::hash_map::{Entry, HashMap},
    sync::Arc,
};

use async_std::{
    io::BufReader,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    task,
    sync::{channel, Sender, Receiver},
    stream,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
enum Void {}

pub(crate) fn main() -> Result<()> {
    task::block_on(accept_loop("127.0.0.1:8080"))
}

async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (broker_sender, broker_receiver) = channel(10);
    let broker = task::spawn(broker_loop(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
    }
    drop(broker_sender);
    broker.await;
    Ok(())
}

async fn connection_loop(broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => return Err("peer disconnected immediately".into()),
        Some(line) => line?,
    };
    let (_shutdown_sender, shutdown_receiver) = channel::<Void>(0);
    broker
        .send(Event::NewPeer {
            name: name.clone(),
            stream: Arc::clone(&stream),
            shutdown: shutdown_receiver,
        })
        .await;

    while let Some(line) = lines.next().await {
        let line = line?;
        let (dest, msg) = match line.find(':') {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        };
        let dest: Vec<String> = dest
            .split(',')
            .map(|name| name.trim().to_string())
            .collect();
        let msg: String = msg.trim().to_string();

        broker
            .send(Event::Message {
                from: name.clone(),
                to: dest,
                msg,
            })
            .await;
    }

    Ok(())
}

#[derive(Debug)]
enum ConnectionWriterEvent {
    Message(String),
    Shutdown
}

async fn connection_writer_loop(
    messages: &mut Receiver<String>,
    stream: Arc<TcpStream>,
    shutdown: Receiver<Void>,
) -> Result<()> {
    let mut stream = &*stream;
    let messages = messages.map(ConnectionWriterEvent::Message);
    let shutdown = shutdown.map(|_| ConnectionWriterEvent::Shutdown).chain(stream::once(ConnectionWriterEvent::Shutdown));

    let mut events = shutdown.merge(messages);

    while let Some(event) = events.next().await {
        match event {
            ConnectionWriterEvent::Message(msg) => {
                stream.write_all(msg.as_bytes()).await?;
            }
            ConnectionWriterEvent::Shutdown => {
                break
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
        shutdown: Receiver<Void>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

#[derive(Debug)]
enum BrokerEvent {
     ClientEvent(Event),
     Disconnection((String, Receiver<String>)),
     Shutdown,
}

async fn broker_loop(events: Receiver<Event>) {
    let (disconnect_sender, disconnect_receiver) = channel(10);

    let mut peers: HashMap<String, Sender<String>> = HashMap::new();
    let disconnect_receiver = disconnect_receiver.map(BrokerEvent::Disconnection);
    let events = events.map(BrokerEvent::ClientEvent).chain(stream::once(BrokerEvent::Shutdown));

    let mut stream = disconnect_receiver.merge(events);

    while let Some(event) = stream.next().await {
        match event {
            BrokerEvent::ClientEvent(Event::Message { from, to, msg }) => {
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        let msg = format!("from {}: {}\n", from, msg);
                        peer.send(msg).await;
                    }
                }
            }
            BrokerEvent::ClientEvent(Event::NewPeer {
                name,
                stream,
                shutdown,
            }) => match peers.entry(name.clone()) {
                Entry::Occupied(..) => (),
                Entry::Vacant(entry) => {
                    let (client_sender, mut client_receiver) = channel(10);
                    entry.insert(client_sender);
                    let  disconnect_sender = disconnect_sender.clone();
                    spawn_and_log_error(async move {
                        let res =
                            connection_writer_loop(&mut client_receiver, stream, shutdown).await;
                        disconnect_sender
                            .send((name, client_receiver))
                            .await;
                        res
                    });
                }
            }
            BrokerEvent::Disconnection((name, _pending_messages)) => {
                assert!(peers.remove(&name).is_some());
            }
            BrokerEvent::Shutdown => break,
        }
    }
    drop(peers);
    drop(disconnect_sender);
    while let Some(BrokerEvent::Disconnection((_name, _pending_messages))) = stream.next().await {}
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{}", e)
        }
    })
}
