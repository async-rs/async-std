# Exercise: Waiting for `std::thread`

Parallel processing is usually done via [threads].
In `async-std`, we have similar concept, called a [`task`].
These two worlds seem different - and in some regards, they are - though they
are easy to connect.
In this exercise, you will learn how to connect to concurrent/parallel components easily, by connecting a thread to a task.

## Understanding the problem

The standard thread API in Rust is `std::thread`. Specifically, it contains the [`spawn`] function, which allows us to start a thread:

```rust
std::thread::spawn(|| {
    println!("in child thread");
})
println!("in parent thread");
```

This creates a thread, _immediately_ [schedules] it to run, and continues. This is crucial: once the thread is spawned, it is independent of its _parent thread_. If you want to wait for the thread to end, you need to capture its [`JoinHandle`] and join it with your current thread:

```rust
let thread = std::thread::spawn(|| {
    println!("in child thread");
})
thread.join();
println!("in parent thread");
```

This comes at a cost though: the waiting thread will [block] until the child is done. Wouldn't it be nice if we could just use the `.await` syntax here and leave the opportunity for another task to be scheduled while waiting?

## Backchannels





[threads]: TODO: wikipedia
[`task`]: TODO: docs link
[`spawn`]: TODO: docs link
[`JoinHandle`]: TODO: docs link
[schedules]: TODO: Glossary link
[block]: TODO: Link to blocking