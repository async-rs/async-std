#![feature(test)]

extern crate test;

use async_std::task;
use async_std::task_local;
use test::{black_box, Bencher};
use std::thread;
use std::time::Duration;
use async_std::task::blocking::JoinHandle;
use futures::future::{join_all};


#[bench]
fn blocking(b: &mut Bencher) {
    b.iter(|| {
        let handles = (0..10_000).map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(1);
                thread::sleep(duration);
            })
        }).collect::<Vec<JoinHandle<()>>>();

        task::block_on(join_all(handles));
    });
}
