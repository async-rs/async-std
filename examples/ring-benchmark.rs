//! Ring benchmark.
//!
//! Source: Programming Erlang: Software for a Concurrent World, by Joe Armstrong, Chapter 8.11.2
//!
//! "Create N processes in a ring. Send a message round the ring M times so that a total of N * M
//! messages get sent. Time how long this takes for different values of N and M."

use std::time::SystemTime;

use async_std::sync::channel;
use async_std::task;

#[derive(Debug)]
struct Payload(u64);
struct End;

fn main() {
    task::block_on(async move {
        let n = 1000000;
        let m = 10;
        let limit = m * n;

        let full_now = SystemTime::now();
        let mut senders = Vec::new();
        let mut receivers = Vec::new();

        for _ in 0..n {
            let (s, r) = channel::<Payload>(1);
            senders.push(s);
            receivers.push(r);
        }

        let mut senders = senders.into_iter();
        let mut receivers = receivers.into_iter();

        let first_sender = senders.next().unwrap();
        let (buz_send, buz_recv) = channel::<End>(1);

        while let Some(r) = receivers.next() {
            if let Some(s) = senders.next() {
                task::spawn(async move {
                    while let Some(mut p) = r.recv().await {
                        p.0 += 1;
                        s.send(p).await;
                    }
                });
            } else {
                let s = first_sender.clone();
                let buz_send = buz_send.clone();

                let mut entry_count = 0;

                task::spawn(async move {
                    while let Some(mut p) = r.recv().await {
                        dbg!(entry_count);
                        entry_count += 1;
                        p.0 += 1;

                        if p.0 >= limit {
                            println!("Reached limit: {}, payload: {}", limit, p.0);
                            break;
                        }
                        s.send(p).await;
                    }
                    buz_send.send(End).await;
                });
            }
        }

        let now = SystemTime::now();
        first_sender.send(Payload(0)).await;
        buz_recv.recv().await;
        let elapsed = now.elapsed().unwrap();

        println!(
            "Time taken: {}.{:06} seconds",
            elapsed.as_secs(),
            elapsed.subsec_micros()
        );
        let elapsed = full_now.elapsed().unwrap();

        println!(
            "Time taken in full: {}.{:06} seconds",
            elapsed.as_secs(),
            elapsed.subsec_micros()
        );
    });
}
