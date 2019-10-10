use async_std::thread;

#[test]
fn smoke() {
    let res = thread::spawn_task(async { 1 + 2 });
    assert_eq!(res, 3);
}

#[test]
#[should_panic = "boom"]
fn panic() {
    thread::spawn_task(async {
        // This panic should get propagated into the parent thread.
        panic!("boom");
    });
}
