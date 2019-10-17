# Changelog

All notable changes to async-std will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

# [0.99.10] - 2019-10-16

This patch stabilizes several core concurrency macros, introduces async versions
of `Path` and `PathBuf`, and adds almost 100 other commits.

## Examples

__Asynchronously read directories from the filesystem__
```rust
use async_std::fs;
use async_std::path::Path;
use async_std::prelude::*;

let path = Path::new("/laputa");
let mut dir = fs::read_dir(&path).await.unwrap();
while let Some(entry) = dir.next().await {
    if let Ok(entry) = entry {
        println!("{:?}", entry.path());
    }
}
```

__Cooperatively reschedule the current task on the executor__
```rust
use async_std::prelude::*;
use async_std::task;

task::spawn(async {
    let x = fibonnacci(1000); // Do expensive work
    task::yield_now().await;  // Allow other tasks to run
    x + fibonnacci(100)       // Do more work
})
```

__Create an interval stream__
```rust
use async_std::prelude::*;
use async_std::stream;
use std::time::Duration;

let mut interval = stream::interval(Duration::from_secs(4));
while let Some(_) = interval.next().await {
    println!("prints every four seconds");
}
```

## Added

- Added `FutureExt` to the `prelude`, allowing us to extend `Future`
- Added `Stream::cmp`
- Added `Stream::ge`
- Added `Stream::last`
- Added `Stream::le`
- Added `Stream::lt`
- Added `Stream::merge` as "unstable", replacing `stream::join!`
- Added `Stream::partial_cmp`
- Added `Stream::take_while`
- Added `Stream::try_fold`
- Added `future::IntoFuture` as "unstable"
- Added `io::BufRead::split`
- Added `io::Write::write_fmt`
- Added `print!`, `println!`, `eprint!`, `eprintln!` macros as "unstable"
- Added `process` as "unstable", re-exporting std types only for now
- Added `std::net` re-exports to the `net` submodule
- Added `std::path::PathBuf` with all associated methods
- Added `std::path::Path` with all associated methods
- Added `stream::ExactSizeStream` as "unstable"
- Added `stream::FusedStream` as "unstable"
- Added `stream::Product`
- Added `stream::Sum`
- Added `stream::from_fn`
- Added `stream::interval` as "unstable"
- Added `stream::repeat_with`
- Added `task::spawn_blocking` as "unstable", replacing `task::blocking`
- Added `task::yield_now`
- Added `write!` and `writeln!` macros as "unstable"
- Stabilized `future::join!` and `future::try_join!`
- Stabilized `future::timeout`
- Stabilized `path`
- Stabilized `task::ready!`

## Changed

- Fixed `BufWriter::into_inner` so it calls `flush` before yielding
- Refactored `io::BufWriter` internals
- Refactored `net::ToSocketAddrs` internals
- Removed Travis CI entirely
- Rewrote the README.md
- Stabilized `io::Cursor`
- Switched bors over to use GitHub actions
- Updated the `io` documentation to match std's `io` docs
- Updated the `task` documentation to match std's `thread` docs

## Removed

- Removed the "unstable" `stream::join!` in favor of `Stream::merge`
- Removed the "unstable" `task::blocking` in favor of `task::spawn_blocking`

# [0.99.9] - 2019-10-08

This patch upgrades our `futures-rs` version, allowing us to build on the 1.39
beta. Additionally we've introduced `map` and `for_each` to `Stream`. And we've
added about a dozen new `FromStream` implementations for `std` types, bringing
us up to par with std's `FromIterator` implementations.

And finally we've added a new "unstable" `task::blocking` function which can be
used to convert blocking code into async code using a threadpool. We've been
using this internally for a while now to async-std to power our `fs` and
`net::SocketAddr` implementations. With this patch userland code now finally has
access to this too.

## Example

__Create a stream of tuples, and collect into a hashmap__
```rust
let a = stream::once(1u8);
let b = stream::once(0u8);

let s = a.zip(b);

let map: HashMap<u8, u8> = s.collect().await;
assert_eq!(map.get(&1), Some(&0u8));
```

__Spawn a blocking task on a dedicated threadpool__
```rust
task::blocking(async {
    println!("long-running task here");
}).await;
```

## Added

- Added `stream::Stream::map`
- Added `stream::Stream::for_each`
- Added `stream::Stream::try_for_each`
- Added `task::blocking` as "unstable"
- Added `FromStream` for all `std::{option, collections, result, string, sync}` types.
- Added the `path` submodule as "unstable".

## Changed

- Updated `futures-preview` to `0.3.0-alpha.19`, allowing us to build on `rustc 1.39.0-beta`.
- As a consequence of this upgrade, all of our concrete stream implementations
  now make use of `Stream::size_hint` to optimize internal allocations.
- We now use GitHub Actions through [actions-rs](https://github.com/actions-rs),
  in addition to Travis CI. We intend to fully switch in the near future.
- Fixed a bug introduced in 0.99.6 where Unix Domain Listeners would sometimes become unresponsive.
- Updated our `sync::Barrier` docs to match std.
- Updated our `stream::FromStream` docs to match std's `FromIterator`.

# [0.99.8] - 2019-09-28

## Added

- Added README to examples directory.
- Added concurrency documentation to the futures submodule.
- Added `io::Read::take` method.
- Added `io::Read::by_ref` method.
- Added `io::Read::chain` method.

## Changed

- Pin futures-preview to `0.3.0-alpha.18`, to avoid rustc upgrade problems.
- Simplified extension traits using a macro.
- Use the `broadcast` module with `std::sync::Mutex`, reducing dependencies.

# [0.99.7] - 2019-09-26

## Added

- Added `future::join` macro as "unstable"
- Added `future::select` macro as "unstable"
- Added `future::try_join` macro as "unstable"
- Added `future::try_select` macro as "unstable"
- Added `io::BufWriter` struct
- Added `stream::Extend` trait
- Added `stream::Stream::chain` method
- Added `stream::Stream::filter` method
- Added `stream::Stream::inspect` method
- Added `stream::Stream::skip_while` method
- Added `stream::Stream::skip` method
- Added `stream::Stream::step_by` method
- Added `sync::Arc` struct from stdlib
- Added `sync::Barrier` struct as "unstable"
- Added `sync::Weak` struct from stdlib
- Added `task::ready` macro as "unstable"

## Changed

- Correctly marked the `pin` submodule as "unstable" in the docs
- Updated tutorial to have certain functions suffixed with `_loop`
- `io` traits are now re-exports of futures-rs types, allowing them to be
  implemented
- `stream` traits are now re-exports of futures-rs types, allowing them to be
  implemented
- `prelude::*` now needs to be in scope for functions `io` and `stream` traits
  to work

# [0.99.6] - 2019-09-19

## Added

- Added `stream::Stream::collect` as "unstable"
- Added `stream::Stream::enumerate`
- Added `stream::Stream::fuse`
- Added `stream::Stream::fold`
- Added `stream::Stream::scan`
- Added `stream::Stream::zip`
- Added `stream::join` macro as "unstable"
- Added `stream::DoubleEndedStream` as "unstable"
- Added `stream::FromStream` trait as "unstable"
- Added `stream::IntoStream` trait as "unstable"
- Added `io::Cursor` as "unstable"
- Added `io::BufRead::consume` method
- Added `io::repeat`
- Added `io::Slice` and `io::SliceMut`
- Added documentation for feature flags
- Added `pin` submodule as "unstable"
- Added the ability to `collect` a stream of `Result<T, E>`s into a
  `Result<impl FromStream<T>, E>`

## Changed

- Refactored the scheduling algorithm of our executor to use work stealing
- Refactored the network driver, removing 400 lines of code
- Removed the `Send` bound from `task::block_on`
- Removed `Unpin` bound from `impl<T: futures::stream::Stream> Stream for T`

# [0.99.5] - 2019-09-12

## Added

- Added tests for `io::timeout`
- Added `io::BufRead::fill_buf`, an `async fn` counterpart to `poll_fill_buf`
- Added `fs::create_dir_all`
- Added `future::timeout`, a free function to time out futures after a threshold
- Added `io::prelude`
- Added `net::ToSocketAddrs`, a non-blocking version of std's `ToSocketAddrs`
- Added `stream::Stream::all`
- Added `stream::Stream::filter_map`
- Added `stream::Stream::find_map`
- Added `stream::Stream::find`
- Added `stream::Stream::min_by`
- Added `stream::Stream::nth`

## Changed

- Polished the text and examples of the tutorial
- `cargo fmt` on all examples
- Simplified internals of `TcpStream::connect_to`
- Modularized our CI setup, enabled a rustfmt fallback, and improved caching
- Reduced our dependency on the `futures-rs` crate, improving compilation times
- Split `io::Read`, `io::Write`, `io::BufRead`, and `stream::Stream` into
  multiple files
- `fs::File` now flushes more often to prevent flushes during `seek`
- Updated all dependencies
- Fixed a bug in the conversion of `File` into raw handle
- Fixed compilation errors on the latest nightly

## Removed

# [0.99.4] - 2019-08-21

## Changes

- Many small changes in the book, mostly typos
- Documentation fixes correcting examples
- Now works with recent nightly with stabilised async/await (> 2019-08-21)

# [0.99.3] - 2019-08-16

- Initial beta release

[Unreleased]: https://github.com/async-rs/async-std/compare/v0.99.10...HEAD
[0.99.10]: https://github.com/async-rs/async-std/compare/v0.99.9...v0.99.10
[0.99.9]: https://github.com/async-rs/async-std/compare/v0.99.8...v0.99.9
[0.99.8]: https://github.com/async-rs/async-std/compare/v0.99.7...v0.99.8
[0.99.7]: https://github.com/async-rs/async-std/compare/v0.99.6...v0.99.7
[0.99.6]: https://github.com/async-rs/async-std/compare/v0.99.5...v0.99.6
[0.99.5]: https://github.com/async-rs/async-std/compare/v0.99.4...v0.99.5
[0.99.4]: https://github.com/async-rs/async-std/compare/v0.99.3...v0.99.4
[0.99.3]: https://github.com/async-rs/async-std/tree/v0.99.3
