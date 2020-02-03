# Changelog

All notable changes to async-std will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

# [1.5.0] - 2020-02-03

[API Documentation](https://docs.rs/async-std/1.5.0/async-std)

This patch includes various quality of life improvements to async-std.
Including improved performance, stability, and the addition of various
`Clone` impls that replace the use of `Arc` in many cases.

## Added

- Added links to various ecosystem projects from the README ([#660](https://github.com/async-rs/async-std/pull/660))
- Added an example on `FromStream` for `Result<T, E>` ([#643](https://github.com/async-rs/async-std/pull/643))
- Added `stream::pending` as "unstable" ([#615](https://github.com/async-rs/async-std/pull/615))
- Added an example of `stream::timeout` to document the error flow ([#675](https://github.com/async-rs/async-std/pull/675))
- Implement `Clone` for `DirEntry` ([#682](https://github.com/async-rs/async-std/pull/682))
- Implement `Clone` for `TcpStream` ([#689](https://github.com/async-rs/async-std/pull/689))

## Changed

- Removed internal comment on `stream::Interval` ([#645](https://github.com/async-rs/async-std/pull/645))
- The "unstable" feature can now be used without requiring the "default" feature ([#647](https://github.com/async-rs/async-std/pull/647))
- Removed unnecessary trait bound on `stream::FlatMap` ([#651](https://github.com/async-rs/async-std/pull/651))
- Updated the "broadcaster" dependency used by "unstable" to `1.0.0` ([#681](https://github.com/async-rs/async-std/pull/681))
- Updated `async-task` to 1.2.1 ([#676](https://github.com/async-rs/async-std/pull/676))
- `task::block_on` now parks after a single poll, improving performance in many cases ([#684](https://github.com/async-rs/async-std/pull/684))
- Improved reading flow of the "client" part of the async-std tutorial ([#550](https://github.com/async-rs/async-std/pull/550))
- Use `take_while` instead of `scan` in `impl` of `Product`, `Sum` and `FromStream` ([#667](https://github.com/async-rs/async-std/pull/667))
- `TcpStream::connect` no longer uses a thread from the threadpool, improving performance ([#687](https://github.com/async-rs/async-std/pull/687))

## Fixed

- Fixed crate documentation typo ([#655](https://github.com/async-rs/async-std/pull/655))
- Fixed documentation for `UdpSocket::recv` ([#648](https://github.com/async-rs/async-std/pull/648))
- Fixed documentation for `UdpSocket::send` ([#671](https://github.com/async-rs/async-std/pull/671))
- Fixed typo in stream documentation ([#650](https://github.com/async-rs/async-std/pull/650))
- Fixed typo on `sync::JoinHandle` documentation ([#659](https://github.com/async-rs/async-std/pull/659))
- Removed use of `std::error::Error::description` which failed CI ([#661](https://github.com/async-rs/async-std/pull/662))
- Removed the use of rustfmt's unstable `format_code_in_doc_comments` option which failed CI ([#685](https://github.com/async-rs/async-std/pull/685))
- Fixed a code typo in the `task::sleep` example ([#688](https://github.com/async-rs/async-std/pull/688))

# [1.4.0] - 2019-12-20

[API Documentation](https://docs.rs/async-std/1.4.0/async-std)

This patch adds `Future::timeout`, providing a method counterpart to the
`future::timeout` free function. And includes several bug fixes around missing
APIs. Notably we're not shipping our new executor yet, first announced [on our
blog](https://async.rs/blog/stop-worrying-about-blocking-the-new-async-std-runtime/).

## Examples

```rust
use async_std::prelude::*;
use async_std::future;
use std::time::Duration;

let fut = future::pending::<()>(); // This future will never resolve.
let res = fut.timeout(Duration::from_millis(100)).await;
assert!(res.is_err()); // The future timed out, returning an err.
```

## Added

- Added `Future::timeout` as "unstable" [(#600)](https://github.com/async-rs/async-std/pull/600)

## Fixes

- Fixed a doc test and enabled it on CI [(#597)](https://github.com/async-rs/async-std/pull/597)
- Fixed a rendering issue with the `stream` submodule documentation [(#621)](https://github.com/async-rs/async-std/pull/621)
- `Write::write_fmt`'s future is now correctly marked as `#[must_use]` [(#628)](https://github.com/async-rs/async-std/pull/628)
- Fixed the missing `io::Bytes` export [(#633)](https://github.com/async-rs/async-std/pull/633)
- Fixed the missing `io::Chain` export [(#633)](https://github.com/async-rs/async-std/pull/633)
- Fixed the missing `io::Take` export [(#633)](https://github.com/async-rs/async-std/pull/633)

# [1.3.0] - 2019-12-12

[API Documentation](https://docs.rs/async-std/1.3.0/async-std)

This patch introduces `Stream::delay`, more methods on `DoubleEndedStream`,
and improves compile times. `Stream::delay` is a new API that's similar to
[`task::sleep`](https://docs.rs/async-std/1.2.0/async_std/task/fn.sleep.html),
but can be passed as part of as stream, rather than as a separate block. This is
useful for examples, or when manually debugging race conditions.

## Examples

```rust
let start = Instant::now();
let mut s = stream::from_iter(vec![0u8, 1]).delay(Duration::from_millis(200));

// The first time will take more than 200ms due to delay.
s.next().await;
assert!(start.elapsed().as_millis() >= 200);

// There will be no delay after the first time.
s.next().await;
assert!(start.elapsed().as_millis() <= 210);
```

## Added

- Added `Stream::delay` as "unstable" [(#309)](https://github.com/async-rs/async-std/pull/309)
- Added `DoubleEndedStream::next_back` as "unstable" [(#562)](https://github.com/async-rs/async-std/pull/562)
- Added `DoubleEndedStream::nth_back` as "unstable" [(#562)](https://github.com/async-rs/async-std/pull/562)
- Added `DoubleEndedStream::rfind` as "unstable" [(#562)](https://github.com/async-rs/async-std/pull/562)
- Added `DoubleEndedStream::rfold` as "unstable" [(#562)](https://github.com/async-rs/async-std/pull/562)
- Added `DoubleEndedStream::try_rfold` as "unstable" [(#562)](https://github.com/async-rs/async-std/pull/562)
- `stream::Once` now implements `DoubleEndedStream` [(#562)](https://github.com/async-rs/async-std/pull/562)
- `stream::FromIter` now implements `DoubleEndedStream` [(#562)](https://github.com/async-rs/async-std/pull/562)

## Changed

- Removed our dependency on `async-macros`, speeding up compilation [(#610)](https://github.com/async-rs/async-std/pull/610)

## Fixes

- Fixed a link in the task docs [(#598)](https://github.com/async-rs/async-std/pull/598)
- Fixed the `UdpSocket::recv` example [(#603)](https://github.com/async-rs/async-std/pull/603)
- Fixed a link to `task::block_on` [(#608)](https://github.com/async-rs/async-std/pull/608)
- Fixed an incorrect API mention in `task::Builder` [(#612)](https://github.com/async-rs/async-std/pull/612)
- Fixed leftover mentions of `futures-preview` [(#595)](https://github.com/async-rs/async-std/pull/595)
- Fixed a typo in the tutorial [(#614)](https://github.com/async-rs/async-std/pull/614)
- `<TcpStream as Write>::poll_close` now closes the write half of the stream [(#618)](https://github.com/async-rs/async-std/pull/618)

# [1.2.0] - 2019-11-27

[API Documentation](https://docs.rs/async-std/1.2.0/async-std)

This patch includes some minor quality-of-life improvements, introduces a
new `Stream::unzip` API, and adds verbose errors to our networking types.

This means if you can't connect to a socket, you'll never have to wonder again
*which* address it was you couldn't connect to, instead of having to go through
the motions to debug what the address was.

## Example

Unzip a stream of tuples into two collections:

```rust
use async_std::prelude::*;
use async_std::stream;

let s = stream::from_iter(vec![(1,2), (3,4)]);

let (left, right): (Vec<_>, Vec<_>) = s.unzip().await;

assert_eq!(left, [1, 3]);
assert_eq!(right, [2, 4]);
```

## Added

- Added `Stream::unzip` as "unstable".
- Added verbose errors to the networking types.

## Changed

- Enabled CI on master branch.
- `Future::join` and `Future::try_join` can now join futures with different
  output types.

## Fixed

- Fixed the docs and `Debug` output of `BufWriter`.
- Fixed a bug in `Stream::throttle` that made it consume too much CPU.

# [1.1.0] - 2019-11-21

[API Documentation](https://docs.rs/async-std/1.1.0/async-std)

This patch introduces a faster scheduler algorithm, `Stream::throttle`, and
stabilizes `task::yield_now`. Additionally we're introducing several more stream
APIs, bringing us to almost complete parity with the standard library.

Furthermore our `path` submodule now returns more context in errors. So if
opening a file fails, async-std will tell you *which* file was failed to open,
making it easier to write and debug programs.

## Examples

```rust
let start = Instant::now();

let mut s = stream::interval(Duration::from_millis(5))
    .throttle(Duration::from_millis(10))
    .take(2);

s.next().await;
assert!(start.elapsed().as_millis() >= 5);

s.next().await;
assert!(start.elapsed().as_millis() >= 15);

s.next().await;
assert!(start.elapsed().as_millis() >= 25);
```

## Added

- Added `Stream::throttle` as "unstable".
- Added `Stream::count` as "unstable".
- Added `Stream::max` as "unstable".
- Added `Stream::successors` as "unstable".
- Added `Stream::by_ref` as "unstable".
- Added `Stream::partition` as "unstable".
- Added contextual errors to the `path` submodule.
- Added `os::windows::symlink_dir` as "unstable".
- Added `os::windows::symlink_file` as "unstable".
- Stabilized `task::yield_now`.

## Fixes

- We now ignore seek errors when rolling back failed `read` calls on `File`.
- Fixed a bug where `Stream::max_by_key` was returning the wrong result.
- Fixed a bug where `Stream::min_by_key` was returning the wrong result.

## Changed

- Applied various fixes to the tutorial.
- Fixed an issue with Clippy.
- Optimized an internal code generation macro, improving compilation speeds.
- Removed an `Unpin` bound from `stream::Once`.
- Removed various extra internal uses of `pin_mut!`.
- Simplified `Stream::any` and `Stream::all`'s internals.
- The `surf` example is now enabled again.
- Tweaked some streams internals.
- Updated `futures-timer` to 2.0.0, improving compilation speed.
- Upgraded `async-macros` to 2.0.0.
- `Stream::merge` now uses randomized ordering to reduce overall latency.
- The scheduler is now more efficient by keeping a slot for the next task to
  run. This is similar to Go's scheduler, and Tokio's scheduler.
- Fixed the documentation of the `channel` types to link back to the `channel`
  function.

# [1.0.1] - 2019-11-12

[API Documentation](https://docs.rs/async-std/1.0.1/async-std)

We were seeing a regression in our fs performance, caused by too many
long-running tasks. This patch fixes that regression by being more proactive
about closing down idle threads.

## Changes

- Improved thread startup/shutdown algorithm in `task::spawn_blocking`.
- Fixed a typo in the tutorial.

# [1.0.0] - 2019-11-11

[API Documentation](https://docs.rs/async-std/1.0.0/async-std)

This release marks the `1.0.0` release of async-std; a major milestone for our
development. This release itself mostly includes quality of life improvements
for all of modules, including more consistent API bounds for a lot of our
submodules.

The biggest change is that we're now using the full semver range,
`major.minor.patch`, and any breaking changes to our "stable" APIs will require
an update of the `major` number.

We're excited we've hit this milestone together with you all. Thank you!

## Added

- Added `Future::join` as "unstable", replacing `future::join!`.
- Added `Future::try_join` as "unstable", replacing `future::try_join!`.
- Enabled `stable` and `beta` channel testing on CI.
- Implemented `FromIterator` and `Extend` for `PathBuf`.
- Implemented `FromStream` for `PathBuf`.
- Loosened the trait bounds of `io::copy` on "unstable".

## Changed

- Added a `Sync` bound to `RwLock`, resolving a memory safety issue.
- Fixed a bug in `Stream::take_while` where it could continue after it should've
  ended.
- Fixed a bug where our `attributes` Cargo feature wasn't working as intended.
- Improved documentation of `Stream::merge`, documenting  ordering guarantees.
- Update doc imports in examples to prefer async-std's types.
- Various quality of life improvements to the `future` submodule.
- Various quality of life improvements to the `path` submodule.
- Various quality of life improvements to the `stream` submodule.

## Removed

- Removed `future::join!` in favor of `Future::join`.
- Removed `future::try_join!` in favor of `Future::try_join`.

# [0.99.12] - 2019-11-07

[API Documentation](https://docs.rs/async-std/0.99.12/async-std)

This patch upgrades us to `futures` 0.3, support for `async/await` on Rust
Stable, performance improvements, and brand new module-level documentation.

## Added

- Added `Future::flatten` as "unstable".
- Added `Future::race` as "unstable" (replaces `future::select!`).
- Added `Future::try_race` as "unstable" (replaces `future::try_select!`).
- Added `Stderr::lock` as "unstable".
- Added `Stdin::lock` as "unstable".
- Added `Stdout::lock` as "unstable".
- Added `Stream::copied` as "unstable".
- Added `Stream::eq` as "unstable".
- Added `Stream::max_by_key` as "unstable".
- Added `Stream::min` as "unstable".
- Added `Stream::ne` as "unstable".
- Added `Stream::position` as "unstable".
- Added `StreamExt` and `FutureExt` as enumerable in the `prelude`.
- Added `TcpListener` and `TcpStream` integration tests.
- Added `stream::from_iter`.
- Added `sync::WakerSet` for internal use.
- Added an example to handle both `IP v4` and `IP v6` connections.
- Added the `default` Cargo feature.
- Added the `attributes` Cargo feature.
- Added the `std` Cargo feature.

## Changed

- Fixed a bug in the blocking threadpool where it didn't spawn more than one thread.
- Fixed a bug with `Stream::merge` where sometimes it ended too soon.
- Fixed a bug with our GitHub actions setup.
- Fixed an issue where our channels could spuriously deadlock.
- Refactored the `task` module.
- Removed a deprecated GitHub action.
- Replaced `futures-preview` with `futures`.
- Replaced `lazy_static` with `once_cell`.
- Replaced all uses of `VecDequeue` in the examples with `stream::from_iter`.
- Simplified `sync::RwLock` using the internal `sync::WakerSet` type.
- Updated the `path` submodule documentation to match std.
- Updated the mod-level documentation to match std.

## Removed

- Removed `future::select!` (replaced by `Future::race`).
- Removed `future::try_select!` (replaced by `Future::try_race`).

# [0.99.11] - 2019-10-29

This patch introduces `async_std::sync::channel`, a novel asynchronous port of
the ultra-fast Crossbeam channels. This has been one of the most anticipated
features for async-std, and we're excited to be providing a first version of
this!

In addition to channels, this patch has the regular list of new methods, types,
and doc fixes.

## Examples

__Send and receive items from a channel__
```rust
// Create a bounded channel with a max-size of 1
let (s, r) = channel(1);

// This call returns immediately because there is enough space in the channel.
s.send(1).await;

task::spawn(async move {
    // This call blocks the current task because the channel is full.
    // It will be able to complete only after the first message is received.
    s.send(2).await;
});

// Receive items from the channel
task::sleep(Duration::from_secs(1)).await;
assert_eq!(r.recv().await, Some(1));
assert_eq!(r.recv().await, Some(2));
```

## Added
- Added `Future::delay` as "unstable"
- Added `Stream::flat_map` as "unstable"
- Added `Stream::flatten` as "unstable"
- Added `Stream::product` as "unstable"
- Added `Stream::sum` as "unstable"
- Added `Stream::min_by_key`
- Added `Stream::max_by`
- Added `Stream::timeout` as "unstable"
- Added `sync::channel` as "unstable".
- Added doc links from instantiated structs to the methods that create them.
- Implemented `Extend` + `FromStream` for `PathBuf`.

## Changed
- Fixed an issue with `block_on` so it works even when nested.
- Fixed issues with our Clippy check on CI.
- Replaced our uses of `cfg_if` with our own macros, simplifying the codebase.
- Updated the homepage link in `Cargo.toml` to point to [async.rs](https://async.rs).
- Updated the module-level documentation for `stream` and `sync`.
- Various typos and grammar fixes.
- Removed redundant file flushes, improving the performance of `File` operations

## Removed
Nothing was removed in this release.

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

[Unreleased]: https://github.com/async-rs/async-std/compare/v1.5.0...HEAD
[1.5.0]: https://github.com/async-rs/async-std/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/async-rs/async-std/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/async-rs/async-std/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/async-rs/async-std/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/async-rs/async-std/compare/v1.0.1...v1.1.0
[1.0.1]: https://github.com/async-rs/async-std/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/async-rs/async-std/compare/v0.99.12...v1.0.0
[0.99.12]: https://github.com/async-rs/async-std/compare/v0.99.11...v0.99.12
[0.99.11]: https://github.com/async-rs/async-std/compare/v0.99.10...v0.99.11
[0.99.10]: https://github.com/async-rs/async-std/compare/v0.99.9...v0.99.10
[0.99.9]: https://github.com/async-rs/async-std/compare/v0.99.8...v0.99.9
[0.99.8]: https://github.com/async-rs/async-std/compare/v0.99.7...v0.99.8
[0.99.7]: https://github.com/async-rs/async-std/compare/v0.99.6...v0.99.7
[0.99.6]: https://github.com/async-rs/async-std/compare/v0.99.5...v0.99.6
[0.99.5]: https://github.com/async-rs/async-std/compare/v0.99.4...v0.99.5
[0.99.4]: https://github.com/async-rs/async-std/compare/v0.99.3...v0.99.4
[0.99.3]: https://github.com/async-rs/async-std/tree/v0.99.3
