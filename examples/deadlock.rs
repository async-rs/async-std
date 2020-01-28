use async_std::task;
use async_std::sync::channel;
use async_std::sync::Mutex;

async fn say_hi() {
    let m = Mutex::new(0);

    let guard = m.lock().await;
    let guard = m.lock().await;
    // let (s, r) = channel(1);
    // drop(r);
    // println!("send 1");
    // s.send(1).await;
    // println!("send 2");
    // s.send(2).await;
}

fn main() {
    // task::block_on(task::spawn(say_hi()));
    task::block_on(say_hi());
}
