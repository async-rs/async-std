use async_std::prelude::*;
use async_std::stream;
use async_std::sync::channel;
use async_std::task;

#[test]
/// Checks that streams are merged fully even if one of the components
/// experiences delay.
fn merging_delayed_streams_work() {
    let (sender, receiver) = channel::<i32>(10);

    let mut s = receiver.merge(stream::empty());
    let t = task::spawn(async move {
        let mut xs = Vec::new();
        while let Some(x) = s.next().await {
            xs.push(x);
        }
        xs
    });

    task::block_on(async move {
        task::sleep(std::time::Duration::from_millis(500)).await;
        sender.send(92).await;
        drop(sender);
        let xs = t.await;
        assert_eq!(xs, vec![92])
    });

    let (sender, receiver) = channel::<i32>(10);

    let mut s = stream::empty().merge(receiver);
    let t = task::spawn(async move {
        let mut xs = Vec::new();
        while let Some(x) = s.next().await {
            xs.push(x);
        }
        xs
    });

    task::block_on(async move {
        task::sleep(std::time::Duration::from_millis(500)).await;
        sender.send(92).await;
        drop(sender);
        let xs = t.await;
        assert_eq!(xs, vec![92])
    });
}
