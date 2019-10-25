#![feature(test)]

extern crate test;

use async_std::task;
use test::{black_box, Bencher};

#[bench]
fn block_on(b: &mut Bencher) {
    b.iter(|| task::block_on(async {}));
}
