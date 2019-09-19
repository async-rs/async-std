# Changelog

All notable changes to async-std will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

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

[Unreleased]: https://github.com/async-rs/async-std/compare/v0.99.5...HEAD
[0.99.5]: https://github.com/async-rs/async-std/compare/v0.99.4...v0.99.5
[0.99.4]: https://github.com/async-rs/async-std/compare/v0.99.3...v0.99.4
[0.99.3]: https://github.com/async-rs/async-std/tree/v0.99.3
