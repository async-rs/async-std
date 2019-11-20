# Examples

This directory contains example code that makes use of `async-std`, each of
which can be run from the command line.

### [Hello World][hello-world]

Spawns a task that says hello.

```shell
cargo run --example hello-world
```

### [Line Count][line-count]

Counts the number of lines in a file given as an argument.

```shell
cargo run --example line-count -- ./Cargo.toml
```

### [List Dir][list-dir]

Lists files in a directory given as an argument.

```shell
cargo run --example list-dir -- .
```

### [Logging][logging]

Prints the runtime's execution log on the standard output.

```shell
cargo run --example logging
```

### [Print File][print-file]

Prints a file given as an argument to stdout.

```shell
cargo run --example print-file ./Cargo.toml
```

### [Ring Benchmark][ring-benchmark]

```shell
cargo run --release --features unstable --example ring-benchmark
```

### [Socket Timeouts][socket-timeouts]

Prints response of GET request made to TCP server with 5 second socket timeout

```shell
cargo run --example socket-timeouts
```

### [Stdin Echo][stdin-echo]

Echoes lines read on stdin to stdout.

```shell
cargo run --example stdin-echo
```

### [Stdin Timeout][stdin-timeout]

Reads a line from stdin, or exits with an error if nothing is read in 5 seconds.

```shell
cargo run --example stdin-timeout
```

### [Surf Web][surf-web]

Sends an HTTP request to the Rust website.

```shell
cargo run --example surf-web
```

### [Task Local][task-local]

Creates a task-local value.

```shell
cargo run --example task-local
```

### [Task Name][task-name]

Spawns a named task that prints its name.

```shell
cargo run --example task-name
```

### [TCP Client][tcp-client]

Connects to localhost over TCP.

First, start the echo server:

```shell
cargo run --example tcp-echo
```

Then run the client:

```shell
cargo run --example tcp-client
```

### [TCP Echo][tcp-echo]

TCP echo server.

Start the echo server:

```shell
cargo run --example tcp-echo
```

Make requests by running the client example:

```shell
cargo run --example tcp-client
```

### [TCP IPv4 and IPv6][tcp-ipv4-and-6-echo]

TCP echo server accepting connections both on both IPv4 and IPV6 sockets.

Start the echo server:

```shell
cargo run --features unstable --example tcp-ipv4-and-6-echo
```

Make requests by running the client example:

```shell
cargo run --example tcp-client
```

### [UDP Client][udp-client]

Connects to localhost over UDP.

First, start the echo server:

```shell
cargo run --example udp-echo
```

Then run the client:

```shell
cargo run --example udp-client
```

### [UDP Echo][udp-echo]

UDP echo server.

Start the echo server:

```shell
cargo run --example udp-echo
```

Make requests by running the client example:

```shell
cargo run --example udp-client
```

[hello-world]: https://github.com/async-rs/async-std/blob/master/examples/hello-world.rs
[line-count]: https://github.com/async-rs/async-std/blob/master/examples/line-count.rs
[list-dir]: https://github.com/async-rs/async-std/blob/master/examples/list-dir.rs
[logging]: https://github.com/async-rs/async-std/blob/master/examples/logging.rs
[print-file]: https://github.com/async-rs/async-std/blob/master/examples/print-file.rs
[ring-benchmark]: https://github.com/async-rs/async-std/blob/master/examples/ring-benchmark.rs
[socket-timeouts]: https://github.com/async-rs/async-std/blob/master/examples/socket-timeouts.rs
[stdin-echo]: https://github.com/async-rs/async-std/blob/master/examples/stdin-echo.rs
[stdin-timeout]: https://github.com/async-rs/async-std/blob/master/examples/stdin-timeout.rs
[surf-web]: https://github.com/async-rs/async-std/blob/master/examples/surf-web.rs
[task-local]: https://github.com/async-rs/async-std/blob/master/examples/task-local.rs
[task-name]: https://github.com/async-rs/async-std/blob/master/examples/task-name.rs
[tcp-client]: https://github.com/async-rs/async-std/blob/master/examples/tcp-client.rs
[tcp-echo]: https://github.com/async-rs/async-std/blob/master/examples/tcp-echo.rs
[tcp-ipv4-and-6-echo]: https://github.com/async-rs/async-std/blob/master/examples/tcp-ipv4-and-6-echo.rs
[udp-client]: https://github.com/async-rs/async-std/blob/master/examples/udp-client.rs
[udp-echo]: https://github.com/async-rs/async-std/blob/master/examples/udp-echo.rs
