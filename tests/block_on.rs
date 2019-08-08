#![feature(async_await)]

use async_std::task;

#[test]
fn smoke() {
    let res = task::block_on(async { 1 + 2 });
    assert_eq!(res, 3);
}

#[test]
#[should_panic = "foo"]
fn panic() {
    task::block_on(async {
        // This panic should get propagated into the parent thread.
        panic!("foo");
    });
}
