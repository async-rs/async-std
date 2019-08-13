# Async version of the Rust standard library

<!-- [![Build Status](https://travis-ci.org/stjepang/async-std.svg?branch=master)]( -->
<!-- https://travis-ci.org/stjepang/async-std) -->
<!-- [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)]( -->
<!-- https://github.com/stjepang/async-std) -->
<!-- [![Cargo](https://img.shields.io/crates/v/async-std.svg)]( -->
<!-- https://crates.io/crates/async-std) -->
<!-- [![Documentation](https://docs.rs/async-std/badge.svg)]( -->
<!-- https://docs.rs/async-std) -->
[![chat](https://img.shields.io/discord/598880689856970762.svg?logo=discord)](https://discord.gg/JvZeVNe)

This crate is an async version of [`std`].

[`std`]: https://doc.rust-lang.org/std/index.html

## Quickstart

Clone the repo:

```
git clone git@github.com:stjepang/async-std.git && cd async-std
```

Read the docs:

```
cargo doc --features docs --open
```

Check out the [examples](examples). To run an example:

```
cargo run --example hello-world
```

## Hello world

```rust
#![feature(async_await)]

use async_std::task;

fn main() {
    task::block_on(async {
        println!("Hello, world!");
    })
}
```

## Low-Friction Sockets with Built-In Timeouts

```rust
#![feature(async_await)]

use std::time::Duration;

use async_std::{
    prelude::*,
    task,
    net::TcpStream,
};

async fn get() -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect("example.com:80").await?;
    stream.write_all(b"GET /index.html HTTP/1.0\r\n\r\n").await?;

    let mut buf = vec![];
    stream.read_to_end(&mut buf)
        .timeout(Duration::from_secs(5))
        .await?;

    Ok(buf)
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

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
