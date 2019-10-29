# Specification and Getting Started

## Specification

The chat uses a simple text protocol over TCP.
The protocol consists of utf-8 messages, separated by `\n`.

The client connects to the server and sends login as a first line.
After that, the client can send messages to other clients using the following syntax:

```text
login1, login2, ... loginN: message
```

Each of the specified clients then receives a `from login: message` message.

A possible session might look like this

```text
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

At the moment `async-std` requires Rust nightly, so let's add a rustup override for convenience:

```bash
$ rustup override add nightly
$ rustc --version
rustc 1.38.0-nightly (c4715198b 2019-08-05)
```

Add the following lines to `Cargo.toml`:

```toml
[dependencies]
futures-preview = { version = "0.3.0-alpha.19", features = [ "async-await" ] }
async-std = "0.99"
```
