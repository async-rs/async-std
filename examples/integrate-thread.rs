#![feature(async_await)]

use async_std::task;

use futures::channel::oneshot;
use std::{thread, time};

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

fn spawn<F, T>(f: F) -> AsyncHandle<T>
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
        notifier: receiver,
    }
}

fn main() {
    let sleepy_thread = spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        String::from("Finished")
    });

    task::block_on(async move {
        println!("waiting for sleepy thread");
        let thread_result = sleepy_thread.join().await;
        match thread_result {
            Ok(s) => println!("Result: {}", s),
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let panicing_thread = spawn(move || {
        panic!("aaah!");
        String::from("Finished!")
    });

    task::block_on(async move {
        println!("waiting for panicking thread");
        let thread_result = panicing_thread.join().await;
        match thread_result {
            Ok(s) => println!("Result: {}", s),
            Err(e) => println!("Error: {:?}", e),
        }
    });
}
