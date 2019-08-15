# Tutorial: Implementing a Chat Server

In this tutorial, we will implement an asynchronous chat on top of async-std.

## Specification

The chat uses a simple text protocol over TCP.
Protocol consists of utf-8 messages, separated by `\n`.

The client connects to the server and sends login as a first line.
After that, the client can send messages to other clients using the following syntax:

```
login1, login2, ... login2: message
```

Each of the specified clients than receives a `from login: message` message.

A possible session might look like this

```
On Alice's computer:   |   On Bob's computer:

> alice                |   > bob
> bob: hello               < from alice: hello
                       |   > alice, bob: hi!
                           < from bob: hi!
< from bob: hi!        |
```

The main challenge for the chat server is keeping track of many concurrent connections.
The main challenge for the chat client is managing concurrent outgoing messages, incoming messages and user's typing.

## Getting Started

Let's create a new Cargo project:

```bash
$ cargo new a-chat
$ cd a-chat
```

At the moment `async-std` requires nightly, so let's add a rustup override for convenience:

```bash
$ rustup override add nightly
$ rustc --version
rustc 1.38.0-nightly (c4715198b 2019-08-05)
```

## Accept Loop

Let's implement the scaffold of the server: a loop that binds a TCP socket to an address and starts accepting connections.


First of all, let's add required import boilerplate:

```rust
#![feature(async_await)]

use std::net::ToSocketAddrs; // 1

use async_std::{
    prelude::*, // 2
    task,       // 3
    net::TcpListener, // 4
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; // 5
```

1. `async_std` uses `std` types where appropriate.
    We'll need `ToSocketAddrs` to specify address to listen on.
2. `prelude` re-exports some traits required to work with futures and streams
3. The `task` module roughtly corresponds to `std::thread` module, but tasks are much lighter weight.
   A single thread can run many tasks.
4. For the socket type, we use `TcpListener` from `async_std`, which is just like `std::net::TcpListener`, but is non-blocking and uses `async` API.
5. We will skip implementing comprehensive error handling in this example.
   To propagate the errors, we will use a boxed error trait object.
   Do you know that there's `From<&'_ str> for Box<dyn Error>` implementation in stdlib, which allows you to use strings with `?` operator?


Now we can write the server's accept loop:

```rust
async fn server(addr: impl ToSocketAddrs) -> Result<()> { // 1
    let listener = TcpListener::bind(addr).await?; // 2
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await { // 3
        // TODO
    }
    Ok(())
}
```

1. We mark `server` function as `async`, which allows us to use `.await` syntax inside.
2. `TcpListener::bind` call returns a future, which we `.await` to extract the `Result`, and then `?` to get a `TcpListener`.
   Note how `.await` and `?` work nicely together.
   This is exactly how `std::net::TcpListener` works, but with `.await` added.
   Mirroring API of `std` is an explicit design goal of `async_std`.
3. Here, we would like to iterate incoming sockets, just how one would do in `std`:

   ```rust
   let listener: std::net::TcpListener = unimplemented!();
   for stream in listener.incoming() {

   }
   ```

   Unfortunately this doesn't quite work with `async` yet, because there's no support for `async` for-loops in the language yet.
   For this reason we have to implement the loop manually, by using `while let Some(item) = iter.next().await` pattern.

Finally, let's add main:

```rust
fn main() -> Result<()> {
    let fut = server("127.0.0.1:8080");
    task::block_on(fut)
}
```

The crucial thing to realise that is in Rust, unlike other languages, calling an async function does **not** run any code.
Async functions only construct futures, which are inert state machines.
To start stepping through the future state-machine in an async function, you should use `.await`.
In a non-async function, a way to execute a future is to handle it to the executor.
In this case, we use `task::block_on` to execute future on the current thread and block until it's done.

## Receiving messages

Let's implement the receiving part of the protocol.
We need to:

1. split incoming `TcpStream` on `\n` and decode bytes as utf-8
2. interpret the first line as a login
3. parse the rest of the lines as a  `login: message`

```rust
use async_std::net::TcpStream;

async fn server(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        let _handle = task::spawn(client(stream)); // 1
    }
    Ok(())
}

async fn client(stream: TcpStream) -> Result<()> {
    let reader = BufReader::new(&stream); // 2
    let mut lines = reader.lines();

    let name = match lines.next().await { // 3
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };
    println!("name = {}", name);

    while let Some(line) = lines.next().await { // 4
        let line = line?;
        let (dest, msg) = match line.find(':') { // 5
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),
        };
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();
    }
    Ok(())
}
```

1. We use `task::spawn` function to spawn an independent task for working with each client.
   That is, after accepting the client the `server` loop immediately starts waiting for the next one.
   This is the core benefit of event-driven architecture: we serve many number of clients concurrently, without spending many hardware threads.

2. Luckily, the "split byte stream into lines" functionality is already implemented.
   `.lines()` call returns a stream of `String`'s.

3. We get the first line -- login

4. And, once again, we implement a manual async for loop.

5. Finally, we parse each line into a list of destination logins and the message itself.

## Managing Errors

One serious problem in the above solution is that, while we correctly propagate errors in the `client`, we just drop the error on the floor afterwards!
That is, `task::spawn` does not return error immediately (it can't, it needs to run the future to completion first), only after it is joined.
We can "fix" it by waiting for the task to be joined, like this:

```rust
let handle = task::spawn(client(stream));
handle.await?
```

The `.await` waits until the client finishes, and `?` propagates the result.

There are two problems with this solution however!
*First*, because we immediately await the client, we can only handle one client at time, and that completely defeats the purpose of async!
*Second*, if a client encounters an IO error, the whole server immediately exits.
That is, a flaky internet connection of one peer brings down the whole chat room!

A correct way to handle client errors in this case is log them, and continue serving other clients.
So let's use a helper function for this:

```rust
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
```

## Sending Messages

Now it's time to implement the other half -- sending messages.
A most obvious way to implement sending is to give each `client` access to the write half of `TcpStream` of each other clients.
That way, a client can directly `.write_all` a message to recipients.
However, this would be wrong: if Alice sends `bob: foo`, and Charley sends `bob: bar`, Bob might actually receive `fobaor`.
Sending a message over a socket might require several syscalls, so two concurrent `.write_all`'s might interfere with each other!

As a rule of thumb, only a single task should write to each `TcpStream`.
So let's create a `client_writer` task which receives messages over a channel and writes them to the socket.
This task would be the point of serialization of messages.
if Alice and Charley send two messages to Bob at the same time, Bob will see the messages in the same order as they arrive in the channel.

```rust
use futures::channel::mpsc; // 1
use futures::SinkExt;

type Sender<T> = mpsc::UnboundedSender<T>; // 2
type Receiver<T> = mpsc::UnboundedReceiver<T>;

async fn client_writer(
    mut messages: Receiver<String>,
    stream: Arc<TcpStream>, // 3
) -> Result<()> {
    let mut stream = &*stream;
    while let Some(msg) = messages.next().await {
        stream.write_all(msg.as_bytes()).await?;
    }
    Ok(())
}
```

1. We will use channels from the `futures` crate.
2. For simplicity, we will use `unbounded` channels, and won't be discussing backpressure in this tutorial.
3. As `client` and `client_writer` share the same `TcpStream`, we need to put it into an `Arc`.
   Note that because `client` only reads from and `client_writer` only writes to the stream, so we don't get a race here.


## Connecting Readers and Writers

So how we make sure that messages read in `client` flow into the relevant `client_writer`?
We should somehow maintain an `peers: HashMap<String, Sender<String>>` map which allows a client to find destination channels.
However, this map would be a bit of shared mutable state, so we'll have to wrap an `RwLock` over it and answer tough questions of what should happen if the client joins at the same moment as it receives a message.

One trick to make reasoning about state simpler comes from the actor model.
We can create a dedicated broker tasks which owns the `peers` map and communicates with other tasks by channels.
By hiding `peers` inside such "actor" task, we remove the need for mutxes and also make serialization point explicit.
The order of events "Bob sends message to Alice" and "Alice joins" is determined by the order of the corresponding events in the broker's event queue.

```rust
#[derive(Debug)]
enum Event { // 1
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

async fn broker(mut events: Receiver<Event>) -> Result<()> {
    let mut peers: HashMap<String, Sender<String>> = HashMap::new(); // 2

    while let Some(event) = events.next().await {
        match event {
            Event::Message { from, to, msg } => {  // 3
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        peer.send(format!("from {}: {}\n", from, msg)).await?
                    }
                }
            }
            Event::NewPeer { name, stream } => {
                match peers.entry(name) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender); // 4
                        spawn_and_log_error(client_writer(client_receiver, stream)); // 5
                    }
                }
            }
        }
    }
    Ok(())
}
```

1. Broker should handle two types of events: a message or an arrival of a new peer.
2. Internal state of the broker is a `HashMap`.
   Note how we don't need a `Mutex` here and can confidently say, at each iteration of the broker's loop, what is the current set of peers
3. To handle a message we send it over a channel to each destination
4. To handle new peer, we first register it in the peer's map ...
5. ... and then spawn a dedicated task to actually write the messages to the socket.

## All Together

At this point, we only need to start broker to get a fully-functioning (in the happy case!) chat:

```rust
#![feature(async_await)]

use std::{
    net::ToSocketAddrs,
    sync::Arc,
    collections::hash_map::{HashMap, Entry},
};

use futures::{
    channel::mpsc,
    SinkExt,
};

use async_std::{
    io::BufReader,
    prelude::*,
    task,
    net::{TcpListener, TcpStream},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;


fn main() -> Result<()> {
    task::block_on(server("127.0.0.1:8080"))
}

async fn server(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (broker_sender, broker_receiver) = mpsc::unbounded(); // 1
    let _broker_handle = task::spawn(broker(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(client(broker_sender.clone(), stream));
    }
    Ok(())
}

async fn client(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream); // 2
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };
    broker.send(Event::NewPeer { name: name.clone(), stream: Arc::clone(&stream) }).await // 3
        .unwrap();

    while let Some(line) = lines.next().await {
        let line = line?;
        let (dest, msg) = match line.find(':') {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),
        };
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();

        broker.send(Event::Message { // 4
            from: name.clone(),
            to: dest,
            msg,
        }).await.unwrap();
    }
    Ok(())
}

async fn client_writer(
    mut messages: Receiver<String>,
    stream: Arc<TcpStream>,
) -> Result<()> {
    let mut stream = &*stream;
    while let Some(msg) = messages.next().await {
        stream.write_all(msg.as_bytes()).await?;
    }
    Ok(())
}

#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

async fn broker(mut events: Receiver<Event>) -> Result<()> {
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await {
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
                        entry.insert(client_sender); // 4
                        spawn_and_log_error(client_writer(client_receiver, stream)); // 5
                    }
                }
            }
        }
    }
    Ok(())
}
```

1. Inside the `server`, we create broker's channel and `task`.
2. Inside `client`, we need to wrap `TcpStream` into an `Arc`, to be able to share it with the `client_writer`.
3. On login, we notify the broker.
   Note that we `.unwrap` on send: broker should outlive all the clients and if that's not the case the broker probably panicked, so we can escalate the panic as well.
4. Similarly, we forward parsed messages to the broker, assuming that it is alive.

## Clean Shutdown

On of the problems of the current implementation is that it doesn't handle graceful shutdown.
If we break from the accept loop for some reason, all in-flight tasks are just dropped on the floor.
A more correct shutdown sequence would be:

1. Stop accepting new clients
2. Deliver all pending messages
3. Exit the process

A clean shutdown in a channel based architecture is easy, although it can appear a magic trick at first.
In Rust, receiver side of a channel is closed as soon as all senders are dropped.
That is, as soon as producers exit and drop their senders, the rest of the system shutdowns naturally.
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
    let broker = task::spawn(broker(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(client(broker_sender.clone(), stream));
    }
    drop(broker_sender); // 1
    broker.await?; // 5
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
        writer.await?;
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

## Handling Disconnections

Currently, we only ever *add* new peers to the map.
This is clearly wrong: if a peer closes connection to the chat, we should not try to send any more messages to it.

One subtlety with handling disconnection is that we can detect it either in the reader's task, or in the writer's task.
The most obvious solution here is to just remove the peer from the `peers` map in both cases, but this would be wrong.
If *both* read and write fail, we'll remove the peer twice, but it can be the case that the peer reconnected between the two failures!
To fix this, we will only remove the peer when the write side finishes.
If the read side finishes we will notify the write side that it should stop as well.
That is, we need to add an ability to signal shutdown for the writer task.

One way to approach this is a `shutdown: Receiver<()>` channel.
There's a more minimal solution however, which makes a clever use of RAII.
Closing a channel is a synchronization event, so we don't need to send a shutdown message, we can just drop the sender.
This way, we statically guarantee that we issue shutdown exactly once, even if we early return via `?` or panic.

First, let's add shutdown channel to the `client`:

```rust
#[derive(Debug)]
enum Void {} // 1

#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
        shutdown: Receiver<Void>, // 2
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

async fn client(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    // ...

    let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>(); // 3
    broker.send(Event::NewPeer {
        name: name.clone(),
        stream: Arc::clone(&stream),
        shutdown: shutdown_receiver,
    }).await.unwrap();

    // ...
}
```

1. To enforce that no messages are send along the shutdown channel, we use an uninhabited type.
2. We pass the shutdown channel to the writer task
3. In the reader, we create an `_shutdown_sender` whose only purpose is to get dropped.

In the `client_writer`, we now need to chose between shutdown and message channels.
We use `select` macro for this purpose:

```rust
use futures::select;

async fn client_writer(
    messages: &mut Receiver<String>,
    stream: Arc<TcpStream>,
    mut shutdown: Receiver<Void>, // 1
) -> Result<()> {
    let mut stream = &*stream;
    loop { // 2
        select! {
            msg = messages.next() => match msg {
                Some(msg) => stream.write_all(msg.as_bytes()).await?,
                None => break,
            },
            void = shutdown.next() => match void {
                Some(void) => match void {}, // 3
                None => break,
            }
        }
    }
    Ok(())
}
```

1. We add shutdown channel as an argument.
2. Because of `select`, we can't use a `white let` loop, so we desugar it further into a `loop`.
3. In the shutdown case we use `match void {}` as a statically-checked `unreachable!()`.

Another problem is that between the moment we detect disconnection in `client_writer` and the moment when we actually remove the peer from the `peers` map, new messages might be pushed into the peer's channel.
To not lose these messages completely, we'll return the messages channel back to broker.
This also allows us to establish a useful invariant that the message channel strictly outlives the peer in the `peers` map, and make the broker itself infailable.

The final code looks like this:

```rust
#![feature(async_await)]

use std::{
    net::ToSocketAddrs,
    sync::Arc,
    collections::hash_map::{HashMap, Entry},
};

use futures::{
    channel::mpsc,
    SinkExt,
    select,
};

use async_std::{
    io::BufReader,
    prelude::*,
    task,
    net::{TcpListener, TcpStream},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[derive(Debug)]
enum Void {}

fn main() -> Result<()> {
    task::block_on(server("127.0.0.1:8080"))
}

async fn server(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker = task::spawn(broker(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(client(broker_sender.clone(), stream));
    }
    drop(broker_sender);
    broker.await;
    Ok(())
}

async fn client(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };
    let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
    broker.send(Event::NewPeer {
        name: name.clone(),
        stream: Arc::clone(&stream),
        shutdown: shutdown_receiver,
    }).await.unwrap();

    while let Some(line) = lines.next().await {
        let line = line?;
        let (dest, msg) = match line.find(':') {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),
        };
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();

        broker.send(Event::Message {
            from: name.clone(),
            to: dest,
            msg,
        }).await.unwrap();
    }

    Ok(())
}

async fn client_writer(
    messages: &mut Receiver<String>,
    stream: Arc<TcpStream>,
    mut shutdown: Receiver<Void>,
) -> Result<()> {
    let mut stream = &*stream;
    loop {
        select! {
            msg = messages.next() => match msg {
                Some(msg) => stream.write_all(msg.as_bytes()).await?,
                None => break,
            },
            void = shutdown.next() => match void {
                Some(void) => match void {},
                None => break,
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

async fn broker(mut events: Receiver<Event>) {
    let (disconnect_sender, mut disconnect_receiver) = // 1
        mpsc::unbounded::<(String, Receiver<String>)>();
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    loop {
        let event = select! {
            event = events.next() => match event {
                None => break, // 2
                Some(event) => event,
            },
            disconnect = disconnect_receiver.next() => {
                let (name, _pending_messages) = disconnect.unwrap(); // 3
                assert!(peers.remove(&name).is_some());
                continue;
            },
        };
        match event {
            Event::Message { from, to, msg } => {
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        peer.send(format!("from {}: {}\n", from, msg)).await
                            .unwrap() // 6
                    }
                }
            }
            Event::NewPeer { name, stream, shutdown } => {
                match peers.entry(name.clone()) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, mut client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let mut disconnect_sender = disconnect_sender.clone();
                        spawn_and_log_error(async move {
                            let res = client_writer(&mut client_receiver, stream, shutdown).await;
                            disconnect_sender.send((name, client_receiver)).await // 4
                                .unwrap();
                            res
                        });
                    }
                }
            }
        }
    }
    drop(peers); // 5
    drop(disconnect_sender); // 6
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {
    }
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
```

1. In the broker, we create a channel to reap disconnected peers and their undelivered messages.
2. The broker's main loop exits when the input events channel is exhausted (that is, when all readers exit).
3. Because broker itself holds a `disconnect_sender`, we know that the disconnections channel can't be fully drained in the main loop.
4. We send peer's name and pending messages to the disconnections channel in both the happy and the not-so-happy path.
   Again, we can safely unwrap because broker outlives writers.
5. We drop `peers` map to close writers' messages channel and shut down the writers for sure.
   It is not strictly necessary in the current setup, where the broker waits for readers' shutdown anyway.
   However, if we add a server-initiated shutdown (for example, kbd:[ctrl+c] handling), this will be a way for the broker to shutdown the writers.
6. Finally, we close and drain the disconnections channel.

## Implementing a client

Let's now implement the client for the chat.
Because the protocol is line-based, the implementation is pretty straightforward:

* Lines read from stdin should be send over the socket.
* Lines read from the socket should be echoed to stdout.

Unlike the server, the client needs only limited concurrency, as it interacts with only a single user.
For this reason, async doesn't bring a lot of performance benefits in this case.

However, async is still useful for managing concurrency!
Specifically, the client should *simultaneously* read from stdin and from the socket.
Programming this with threads is cumbersome, especially when implementing clean shutdown.
With async, we can just use the `select!` macro.

```rust
#![feature(async_await)]

use std::net::ToSocketAddrs;

use futures::select;

use async_std::{
    prelude::*,
    net::TcpStream,
    task,
    io::{stdin, BufReader},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


fn main() -> Result<()> {
    task::block_on(try_main("127.0.0.1:8080"))
}

async fn try_main(addr: impl ToSocketAddrs) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = (&stream, &stream); // 1
    let reader = BufReader::new(reader);
    let mut lines_from_server = futures::StreamExt::fuse(reader.lines()); // 2

    let stdin = BufReader::new(stdin());
    let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines()); // 2
    loop {
        select! { // 3
            line = lines_from_server.next() => match line {
                Some(line) => {
                    let line = line?;
                    println!("{}", line);
                },
                None => break,
            },
            line = lines_from_stdin.next() => match line {
                Some(line) => {
                    let line = line?;
                    writer.write_all(line.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
                None => break,
            }
        }
    }
    Ok(())
}
```

1. Here we split `TcpStream` into read and write halfs: there's `impl AsyncRead for &'_ TcpStream`, just like the one in std.
2. We crate a steam of lines for both the socket and stdin.
3. In the main select loop, we print the lines we receive from server and send the lines we read from the console.
