# Async version of Rust's standard library

[![Build Status](https://travis-ci.com/async-rs/async-std.svg?branch=master)](https://travis-ci.com/async-rs/async-std)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/async-rs/async-std)
[![Cargo](https://img.shields.io/crates/v/async-std.svg)](https://crates.io/crates/async-std)
[![Documentation](https://docs.rs/async-std/badge.svg)](https://docs.rs/async-std)
[![chat](https://img.shields.io/discord/598880689856970762.svg?logo=discord)](https://discord.gg/JvZeVNe)

This crate provides an async version of [`std`]. It provides all the interfaces you
are used to, but in an async version and ready for Rust's `async`/`await` syntax.

[`std`]: https://doc.rust-lang.org/std/index.html

## Documentation

`async-std` comes with [extensive API documentation][docs] and a [book][book].

[docs]: https://docs.rs/async-std
[book]: https://book.async.rs

## Quickstart

Add the following lines to your `Cargo.toml`:

```toml
[dependencies]
async-std = "0.99"
```

Or use [cargo add][cargo-add] if you have it installed:

```sh
$ cargo add async-std
```

[cargo-add]: https://github.com/killercup/cargo-edit

## Hello world

```rust
use async_std::task;

fn main() {
    task::block_on(async {
        println!("Hello, world!");
    })
}
```

## Low-Friction Sockets with Built-In Timeouts

```rust
use std::time::Duration;

use async_std::{
    prelude::*,
    task,
    io,
    net::TcpStream,
};

async fn get() -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect("example.com:80").await?;
    stream.write_all(b"GET /index.html HTTP/1.0\r\n\r\n").await?;

    let mut buf = vec![];

    io::timeout(Duration::from_secs(5), async {
        stream.read_to_end(&mut buf).await?;
        Ok(buf)
    }).await
}

fn main() {
    task::block_on(async {
        let raw_response = get().await.expect("request");
        let response = String::from_utf8(raw_response)
            .expect("utf8 conversion");
        println!("received: {}", response);
    });
}
```

## Features

`async-std` is strongly commited to following semver. This means your code won't
break unless _you_ decide to upgrade.

However every now and then we come up with something that we think will work
_great_ for `async-std`, and we want to provide a sneak-peek so you can try it
out. This is what we call _"unstable"_ features. You can try out the unstable
features by enabling the `unstable` feature in you `Cargo.toml` file:

```toml
[dependencies]
async-std = { version = "0.99.5", features = ["unstable"] }
```

Just be careful when running these features, as they may change between
versions.

## Take a look around

Clone the repo:

```
git clone git@github.com:async-rs/async-std.git && cd async-std
```

Generate docs:

```
cargo doc --features docs.rs --open
```

Check out the [examples](examples). To run an example:

```
cargo run --example hello-world
```

## Contributing

See [our contribution document][contribution].

[contribution]: https://async.rs/contribute

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
