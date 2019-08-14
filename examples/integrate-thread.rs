#![feature(async_await)]

use async_std::task;

use std::{thread,time};
use futures::channel::oneshot;

struct AsyncHandle<T> {
    handle: thread::JoinHandle<T>,
    notifier: oneshot::Receiver<()>,
}

impl<T> AsyncHandle<T> {
    fn thread(&self) -> &std::thread::Thread {
        self.handle.thread()
    }

    async fn join(self) -> std::thread::Result<T> {
        // ignore results, the join handle will propagate panics
        let _ = self.notifier.await;
        self.handle.join()
    }
}


fn spawn<F,T>(f: F) -> AsyncHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = oneshot::channel::<()>();

    let thread_handle = thread::spawn(move || {
        let res = f();
        sender.send(()).unwrap();
        res
    });

    AsyncHandle {
        handle: thread_handle,
        notifier: receiver
    }
}

fn main() {
    let thread_handle = spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        String::from("Finished")
    });

    task::block_on(async move {
        println!("waiting for thread 1");
        let thread_result = thread_handle.join().await;
        match thread_result {
            Ok(s) => println!("Result: {}", s),
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let thread_handle = spawn(move || {
        panic!("aaah!");
        String::from("Finished!")
    });

    task::block_on(async move {
        println!("waiting for thread 2");
        let thread_result = thread_handle.join().await;
        match thread_result {
            Ok(s) => println!("Result: {}", s),
            Err(e) => println!("Error: {:?}", e),
        }
    });
}