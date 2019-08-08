# Stability and SemVer

`async-std` and `async-task` follow https://semver.org/.

In short: we are versioning our software as `MAJOR.MINOR.PATCH`. We increase the:

* MAJOR version when there are incompatible API changes,
* MINOR version when we introducece functionality in a backwards-compatible manner
* PATCH version when we make backwards-compatible bug fixes

## Future expectations

`async-std` uses the `AsyncRead/AsyncWrite/AsyncSeek/AsyncBufRead` and the `Stream` traits from the `futures-rs` library. We expect those to be conservatively updated and in lockstep. Breaking changes in these traits will lead to a major version upgrade, for which we will provide migration documentation.

## Minimum version policy

The current tentative policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `async-std` 1.0 requires Rust 1.37.0, then `async-std` 1.0.z for all values of z will also require Rust 1.37.0 or newer. However, `async-std` 1.y for y > 0 may require a newer minimum version of Rust.

In general, this crate will be conservative with respect to the minimum supported version of Rust. With `async/await` being a new feature though, we will track changes in a measured pace.

## Security fixes

Security fixes will be applied to _all_ minor branches of this library in all _supported_ major revisions. This policy might change in the future, in which case we give at least _3 month_ of ahead notice.

## Credits

This policy is based on [burntsushis regex crate][regex-policy].

[regex-policy]: https://github.com/rust-lang/regex#minimum-rust-version-policy
