use async_std::task;
use async_std::task::blocking::JoinHandle;
use futures::future::join_all;
use std::thread;
use std::time::Duration;
use std::time::Instant;

// Test for slow joins without task bursts during joins.
#[test]
fn slow_join() {
    let thread_join_time_max = 11_000;
    let start = Instant::now();

    // Send an initial batch of million bursts.
    let handles = (0..1_000_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(1);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Let them join to see how it behaves under different workloads.
    let duration = Duration::from_millis(thread_join_time_max);
    thread::sleep(duration);

    // Spawn yet another batch of work on top of it
    let handles = (0..10_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(100);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Slow joins shouldn't cause internal slow down
    let elapsed = start.elapsed().as_millis() - thread_join_time_max as u128;
    println!("Slow task join. Monotonic exec time: {:?} ns", elapsed);

    // Should be less than 25_000 ns
    // Previous implementation is around this threshold.
    assert_eq!(elapsed < 25_000, true);
}

// Test for slow joins with task burst.
#[test]
fn slow_join_interrupted() {
    let thread_join_time_max = 2_000;
    let start = Instant::now();

    // Send an initial batch of million bursts.
    let handles = (0..1_000_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(1);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Let them join to see how it behaves under different workloads.
    // This time join under the time window.
    let duration = Duration::from_millis(thread_join_time_max);
    thread::sleep(duration);

    // Spawn yet another batch of work on top of it
    let handles = (0..10_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(100);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Slow joins shouldn't cause internal slow down
    let elapsed = start.elapsed().as_millis() - thread_join_time_max as u128;
    println!("Slow task join. Monotonic exec time: {:?} ns", elapsed);

    // Should be less than 25_000 ns
    // Previous implementation is around this threshold.
    assert_eq!(elapsed < 25_000, true);
}

// This test is expensive but it proves that longhauling tasks are working in adaptive thread pool.
// Thread pool which spawns on-demand will panic with this test.
#[test]
#[ignore]
fn longhauling_task_join() {
    let thread_join_time_max = 11_000;
    let start = Instant::now();

    // First batch of overhauling tasks
    let handles = (0..100_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(1000);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Let them join to see how it behaves under different workloads.
    let duration = Duration::from_millis(thread_join_time_max);
    thread::sleep(duration);

    // Send yet another medium sized batch to see how it scales.
    let handles = (0..10_000)
        .map(|_| {
            task::blocking::spawn(async {
                let duration = Duration::from_millis(100);
                thread::sleep(duration);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    task::block_on(join_all(handles));

    // Slow joins shouldn't cause internal slow down
    let elapsed = start.elapsed().as_millis() - thread_join_time_max as u128;
    println!(
        "Long-hauling task join. Monotonic exec time: {:?} ns",
        elapsed
    );

    // Should be less than 200_000 ns
    // Previous implementation will panic when this test is running.
    assert_eq!(elapsed < 200_000, true);
}
