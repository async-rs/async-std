#![cfg(not(target_os = "unknown"))]

use async_std::task;

#[test]
fn smoke() {
    let res = task::block_on(async { 1 + 2 });
    assert_eq!(res, 3);
}

#[test]
#[should_panic = "boom"]
fn panic() {
    task::block_on(async {
        // This panic should get propagated into the parent thread.
        panic!("boom");
    });
}

#[cfg(feature = "unstable")]
#[test]
fn nested_block_on_local() {
    let x = task::block_on(async {
        let a =
            task::block_on(async { task::block_on(async { async_std::future::ready(3).await }) });
        let b = task::spawn_local(async {
            task::block_on(async { async_std::future::ready(2).await })
        })
        .await;
        let c =
            task::block_on(async { task::block_on(async { async_std::future::ready(1).await }) });
        a + b + c
    });

    assert_eq!(x, 3 + 2 + 1);

    let y = task::block_on(async {
        let a =
            task::block_on(async { task::block_on(async { async_std::future::ready(3).await }) });
        let b = task::spawn_local(async {
            task::block_on(async { async_std::future::ready(2).await })
        })
        .await;
        let c =
            task::block_on(async { task::block_on(async { async_std::future::ready(1).await }) });
        a + b + c
    });

    assert_eq!(y, 3 + 2 + 1);
}

#[test]
fn nested_block_on() {
    let x = task::block_on(async {
        let a =
            task::block_on(async { task::block_on(async { async_std::future::ready(3).await }) });
        let b =
            task::block_on(async { task::block_on(async { async_std::future::ready(2).await }) });
        let c =
            task::block_on(async { task::block_on(async { async_std::future::ready(1).await }) });
        a + b + c
    });

    assert_eq!(x, 3 + 2 + 1);

    let y = task::block_on(async {
        let a =
            task::block_on(async { task::block_on(async { async_std::future::ready(3).await }) });
        let b =
            task::block_on(async { task::block_on(async { async_std::future::ready(2).await }) });
        let c =
            task::block_on(async { task::block_on(async { async_std::future::ready(1).await }) });
        a + b + c
    });

    assert_eq!(y, 3 + 2 + 1);
}
