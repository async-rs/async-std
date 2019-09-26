# `async-std` Examples

This directory contains example code that makes use of `async-std`, each of which can be run from the command line.

## Examples

- [Hello World][hello-world]

Spawns a task that says hello.

```
cargo run --example hello-world
```

- [Line Count][line-count]

Counts the number of lines in a file given as an argument.

```shell
cargo run --example line-count -- ./Cargo.toml
```

- [List Dir][list-dir]

Lists files in a directory given as an argument.

```shell
cargo run --example list-dir -- .
```

- [Logging][logging]

Prints the runtime's execution log on the standard output.

```shell
cargo run --example logging
```

- [Print File][print-file]

Prints a file given as an argument to stdout.

```shell
cargo run --example print-file ./Cargo.toml
```

[hello-world]: https://github.com/async-rs/async-std/blob/master/examples/hello-world.rs
[line-count]: https://github.com/async-rs/async-std/blob/master/examples/line-count.rs
[list-dir]: https://github.com/async-rs/async-std/blob/master/examples/list-dir.rs
[logging]: https://github.com/async-rs/async-std/blob/master/examples/logging.rs
[print-file]: https://github.com/async-rs/async-std/blob/master/examples/print-file.rs
