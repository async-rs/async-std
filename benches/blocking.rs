#![feature(test)]

extern crate test;

use async_std::task;
use async_std::task::blocking::JoinHandle;
use futures::future::join_all;
use std::thread;
use std::time::Duration;
use test::Bencher;

// Benchmark for a 10K burst task spawn
#[bench]
fn blocking(b: &mut Bencher) {
    b.iter(|| {
        let handles = (0..10_000)
            .map(|_| {
                task::blocking::spawn(async {
                    let duration = Duration::from_millis(1);
                    thread::sleep(duration);
                })
            })
            .collect::<Vec<JoinHandle<()>>>();

        task::block_on(join_all(handles));
    });
}

// Benchmark for a single blocking task spawn
#[bench]
fn blocking_single(b: &mut Bencher) {
    b.iter(|| {
        task::blocking::spawn(async {
            let duration = Duration::from_millis(1);
            thread::sleep(duration);
        })
    });
}
