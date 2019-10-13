//! Internal types for stdio.
//!
//! This module is a port of `libstd/io/stdio.rs`,and contains internal types for `print`/`eprint`.

use crate::io::{stderr, stdout, Write};
use std::fmt;

/// Write `args` `global_s`. `label` identifies the stream in a panic message.
async fn print_to<T>(
    args: fmt::Arguments<'_>,
    global_s: fn() -> T,
    label: &str,
) where
    T: Write,
{
    if let Err(e) = global_s().write_fmt(args).await {
        panic!("failed printing to {}: {}", label, e);
    }
}

#[doc(hidden)]
pub async fn _print(args: fmt::Arguments<'_>) {
    print_to(args, stdout, "stdout");
}

#[doc(hidden)]
pub async fn _eprint(args: fmt::Arguments<'_>) {
    print_to(args, stderr, "stderr");
}
