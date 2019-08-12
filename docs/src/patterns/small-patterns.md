# Small Patterns

A collection of small, useful patterns.

## Splitting streams

`async-std` doesn't provide a `split()` method on `io` handles. Instead, splitting a stream into a read and write half can be done like this:

```rust
use async_std::io;

async fn echo(stream: io::TcpStream) {
    let (reader, writer) = &mut (&stream, &stream);
    io::copy(reader, writer).await?;
}
```