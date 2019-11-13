#![feature(test)]

mod async_std {
    extern crate test;

    use async_std::sync::{Arc, Mutex};
    use async_std::task;
    use test::Bencher;

    #[bench]
    fn create(b: &mut Bencher) {
        b.iter(|| Mutex::new(()));
    }

    #[bench]
    fn contention(b: &mut Bencher) {
        b.iter(|| task::block_on(run(10, 1000)));
    }

    #[bench]
    fn no_contention(b: &mut Bencher) {
        b.iter(|| task::block_on(run(1, 10000)));
    }

    async fn run(task: usize, iter: usize) {
        let m = Arc::new(Mutex::new(()));
        let mut tasks = Vec::new();

        for _ in 0..task {
            let m = m.clone();
            tasks.push(task::spawn(async move {
                for _ in 0..iter {
                    let _ = m.lock().await;
                }
            }));
        }

        for t in tasks {
            t.await;
        }
    }
}

mod std {
    extern crate test;

    use std::sync::{Arc, Mutex};
    use std::thread;
    use test::Bencher;

    #[bench]
    fn create(b: &mut Bencher) {
        b.iter(|| Mutex::new(()));
    }

    #[bench]
    fn contention(b: &mut Bencher) {
        b.iter(|| run(10, 1000));
    }

    #[bench]
    fn no_contention(b: &mut Bencher) {
        b.iter(|| run(1, 10000));
    }

    fn run(thread: usize, iter: usize) {
        let m = Arc::new(Mutex::new(()));
        let mut threads = Vec::new();

        for _ in 0..thread {
            let m = m.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..iter {
                    let _ = m.lock().unwrap();
                }
            }));
        }

        for t in threads {
            t.join().unwrap();
        }
    }
}

mod parking_lot {
    extern crate test;

    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::thread;
    use test::Bencher;

    #[bench]
    fn create(b: &mut Bencher) {
        b.iter(|| Mutex::new(()));
    }

    #[bench]
    fn contention(b: &mut Bencher) {
        b.iter(|| run(10, 1000));
    }

    #[bench]
    fn no_contention(b: &mut Bencher) {
        b.iter(|| run(1, 10000));
    }

    fn run(thread: usize, iter: usize) {
        let m = Arc::new(Mutex::new(()));
        let mut threads = Vec::new();

        for _ in 0..thread {
            let m = m.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..iter {
                    let _ = m.lock();
                }
            }));
        }

        for t in threads {
            t.join().unwrap();
        }
    }
}
