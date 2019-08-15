# Overview

![async-std logo](./images/horizontal_color.svg)

`async-std` along with its [supporting libraries][organization] is a library making your life in async programming easier. It provides provide fundamental implementations for downstream libraries and applications alike. The name reflects the approach of this library: it is a closely modeled to the Rust main standard library as possible, replacing all components by async counterparts.

`async-std` provides an interface to all important primitives: filesystem operations, network operations and concurrency basics like timers. It also exposes an `task` in a model similar to the `thread` module found in the Rust standard lib.  But it does not only include io primitives, but also `async/await` compatible versions of primitives like `Mutex`. You can read more about `async-std` in [the overview chapter][overview-std].

[organization]: https://github.com/async-rs/async-std
[overview-std]: overview/async-std/
