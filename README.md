# Async version of the Rust standard library

<!-- [![Build Status](https://travis-ci.com/async-rs/async-std.svg?branch=master)]( -->
<!-- https://travis-ci.com/async-rs/async-std) -->
<!-- [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)]( -->
<!-- https://github.com/async-rs/async-std) -->
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
git clone git@github.com:async-rs/async-std.git && cd async-std
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

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
