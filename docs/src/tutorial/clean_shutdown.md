## Clean Shutdown

One of the problems of the current implementation is that it doesn't handle graceful shutdown.
If we break from the accept loop for some reason, all in-flight tasks are just dropped on the floor.
A more correct shutdown sequence would be:

1. Stop accepting new clients
2. Deliver all pending messages
3. Exit the process

A clean shutdown in a channel based architecture is easy, although it can appear a magic trick at first.
In Rust, receiver side of a channel is closed as soon as all senders are dropped.
That is, as soon as producers exit and drop their senders, the rest of the system shuts down naturally.
In `async_std` this translates to two rules:

1. Make sure that channels form an acyclic graph.
2. Take care to wait, in the correct order, until intermediate layers of the system process pending messages.

In `a-chat`, we already have an unidirectional flow of messages: `reader -> broker -> writer`.
However, we never wait for broker and writers, which might cause some messages to get dropped.
Let's add waiting to the server:

```rust
async fn server(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker_handle = task::spawn(broker(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(client(broker_sender.clone(), stream));
    }
    drop(broker_sender); // 1
    broker_handle.await?; // 5
    Ok(())
}
```

And to the broker:

```rust
async fn broker(mut events: Receiver<Event>) -> Result<()> {
    let mut writers = Vec::new();
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await { // 2
        match event {
            Event::Message { from, to, msg } => {
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        peer.send(format!("from {}: {}\n", from, msg)).await?
                    }
                }
            }
            Event::NewPeer { name, stream} => {
                match peers.entry(name) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let handle = spawn_and_log_error(client_writer(client_receiver, stream));
                        writers.push(handle); // 4
                    }
                }
            }
        }
    }
    drop(peers); // 3
    for writer in writers { // 4
        writer.await;
    }
    Ok(())
}
```

Notice what happens with all of the channels once we exit the accept loop:

1. First, we drop the main broker's sender.
   That way when the readers are done, there's no sender for the broker's channel, and the chanel closes.
2. Next, the broker exits `while let Some(event) = events.next().await` loop.
3. It's crucial that, at this stage, we drop the `peers` map.
   This drops writer's senders.
4. Now we can join all of the writers.
5. Finally, we join the broker, which also guarantees that all the writes have terminated.
