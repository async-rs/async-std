# Async version of Rust's standard library

[![Build Status](https://travis-ci.org/async-rs/async-std.svg?branch=master)](https://travis-ci.org/stjepang/async-std)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/async-rs/async-std)
[![Cargo](https://img.shields.io/crates/v/async-std.svg)](https://crates.io/crates/async-std)
[![Documentation](https://docs.rs/async-std/badge.svg)](https://docs.rs/async-std)
[![chat](https://img.shields.io/discord/598880689856970762.svg?logo=discord)](https://discord.gg/JvZeVNe)

This crate provides an async version of [`std`]. It provides all the interfaces you are used to, but in an async version and ready for Rust's `async/await`-syntax.

[`std`]: https://doc.rust-lang.org/std/index.html

## Documentation

`async-std` comes with [extensive API domentation][docs] and a [book][book].

[docs]: https://docs.rs/async-std
[book]: https://book.async.rs

## Quickstart

Add the following lines to you `Cargo.toml`:

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
#![feature(async_await)]

use async_std::task;

fn main() {
    task::block_on(async {
        println!("Hello, world!");
    })
}
```

## Take a look around

Clone the repo:

```
git clone git@github.com:stjepang/async-std.git && cd async-std
```

Read the docs:

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
