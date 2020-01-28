use async_std::task;

fn main() {
    task::block_on(async {
        let mut i = 0;
        loop {
            i += 1;
            dbg!(i);

            task::spawn(async {
                let mut v = Vec::new();
                let m = std::sync::Arc::new(async_std::sync::Mutex::new(0));

                for _ in 0..1000 {
                    let m = m.clone();
                    v.push(task::spawn(async move {
                        for _ in 0..10000 {
                            *m.lock().await += 1;
                        }
                    }));
                }

                for t in v {
                    t.await;
                }
            })
            .await;
        }
    });
}
