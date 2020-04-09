//! Monitor for the runtime.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::rt;
use crate::task;

pub fn run() {
    const PROB_INTERVAL: Duration = Duration::from_millis(500);
    const SCALE_DOWN_INTERVAL: Duration = Duration::from_secs(5);

    let running = &Arc::new(AtomicBool::new(false));

    {
        let running = Arc::clone(running);
        task::spawn(async move {
            loop {
                running.store(true, Ordering::SeqCst);
                task::sleep(PROB_INTERVAL).await;
            }
        });
    }

    let mut next_scalling_down = Instant::now() + SCALE_DOWN_INTERVAL;

    loop {
        running.store(false, Ordering::SeqCst);
        thread::sleep(PROB_INTERVAL + Duration::from_millis(10));
        if !running.load(Ordering::SeqCst) {
            eprintln!("WARNING: You are blocking the runtime, please use spawn_blocking");
            rt::scale_up();
        }

        if next_scalling_down <= Instant::now() {
            rt::scale_down();
            next_scalling_down += SCALE_DOWN_INTERVAL;
        }
    }
}
