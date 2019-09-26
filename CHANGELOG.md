# Changelog

All notable changes to async-std will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

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

[Unreleased]: https://github.com/async-rs/async-std/compare/v0.99.7...HEAD
[0.99.7]: https://github.com/async-rs/async-std/compare/v0.99.6...0.99.7
[0.99.6]: https://github.com/async-rs/async-std/compare/v0.99.5...0.99.6
[0.99.5]: https://github.com/async-rs/async-std/compare/v0.99.4...v0.99.5
[0.99.4]: https://github.com/async-rs/async-std/compare/v0.99.3...v0.99.4
[0.99.3]: https://github.com/async-rs/async-std/tree/v0.99.3
